#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

//! Utilities for patching OpenType fonts so one word renders as another.

mod font;
mod gsub;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use read_fonts::{FileRef, FontRef, ReadError};
use write_fonts::{BuilderError, FontBuilder};

/// The main trait for "morphing" text.
pub trait TextMorph {
    /// Patch the font so it shows `from_word` as `to_word`, returning the rebuilt font bytes.
    ///
    /// The two words must have the same length, must be non-empty, and the font must
    /// contain glyphs for all characters in both words.
    fn morph(&self, from_word: &str, to_word: &str) -> Result<Vec<u8>, MorphError>;
}

impl TextMorph for FontRef<'_> {
    fn morph(&self, from_word: &str, to_word: &str) -> Result<Vec<u8>, MorphError> {
        morph_font(self.clone(), from_word, to_word)
    }
}

impl TextMorph for FileRef<'_> {
    fn morph(&self, from_word: &str, to_word: &str) -> Result<Vec<u8>, MorphError> {
        match self {
            Self::Font(font) => font.morph(from_word, to_word),
            Self::Collection(collection) => {
                if collection.len() == 1 {
                    collection
                        .get(0)
                        .map_err(MorphError::Read)?
                        .morph(from_word, to_word)
                } else {
                    Err(MorphError::CollectionIndexRequired(collection.len()))
                }
            }
        }
    }
}

/// Errors that can occur during morphing.
#[derive(Debug, Clone)]
pub enum MorphError {
    /// The two words have different lengths.
    DifferentLengths,
    /// The input word is empty.
    EmptyWord,
    /// The font does not have a usable Unicode cmap.
    MissingCmap,
    /// The font is missing a glyph for a character in one of the words.
    MissingGlyph(char),
    /// The file is a font collection with multiple entries and an explicit font selection is required.
    CollectionIndexRequired(u32),
    /// An error occurred while reading the font.
    Read(ReadError),
    /// An error occurred while building the font.
    Builder(BuilderError),
}

impl MorphError {
    pub(crate) fn malformed(message: &'static str) -> Self {
        Self::Read(ReadError::MalformedData(message))
    }
}

fn morph_font(font: FontRef<'_>, from_word: &str, to_word: &str) -> Result<Vec<u8>, MorphError> {
    let (from_glyphs, to_glyphs) = font::validate_words(&font, from_word, to_word)?;
    let gsub = gsub::patch_gsub(&font, &from_glyphs, &to_glyphs)?;

    let mut builder = FontBuilder::new();
    builder
        .add_table(&gsub)
        .map_err(MorphError::Builder)?
        .copy_missing_tables(font);

    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use read_fonts::TableProvider;

    use super::*;

    fn fixture_bytes() -> Vec<u8> {
        std::fs::read("tests/fonts/msyh.ttc").expect("fixture font should exist")
    }

    #[test]
    fn rejects_different_lengths() {
        let bytes = fixture_bytes();
        let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
        let err = font
            .morph("abc", "xy")
            .expect_err("expected a validation error");
        assert!(matches!(err, MorphError::DifferentLengths));
    }

    #[test]
    fn rejects_empty_words() {
        let bytes = fixture_bytes();
        let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
        let err = font.morph("", "").expect_err("expected a validation error");
        assert!(matches!(err, MorphError::EmptyWord));
    }

    #[test]
    fn rejects_missing_glyphs() {
        let bytes = fixture_bytes();
        let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
        let err = font
            .morph("abc", "😀bc")
            .expect_err("expected a missing glyph error");
        assert!(matches!(err, MorphError::MissingGlyph('😀')));
    }

    #[test]
    fn builds_a_font_with_calt_feature() {
        let bytes = fixture_bytes();
        let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
        let morphed = font
            .morph("abc", "xyz")
            .expect("font should patch successfully");

        let rebuilt = FontRef::new(&morphed).expect("patched font should parse as sfnt");
        let gsub = rebuilt.gsub().expect("patched font should contain GSUB");
        let feature_list = gsub
            .feature_list()
            .expect("patched GSUB should contain a feature list");
        let has_calt = feature_list
            .feature_records()
            .iter()
            .any(|record| record.feature_tag() == read_fonts::types::Tag::new(b"calt"));

        assert!(has_calt, "patched font should expose a calt feature");
    }

    #[test]
    fn file_ref_delegates_to_single_font() {
        let bytes = fixture_bytes();
        let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
        let file = FileRef::Font(font);
        let morphed = file
            .morph("abc", "xyz")
            .expect("single-font FileRef should patch successfully");

        assert!(
            FontRef::new(&morphed).is_ok(),
            "patched bytes should be a standalone sfnt"
        );
    }

    #[test]
    fn file_ref_rejects_multi_font_collection() {
        let bytes = fixture_bytes();
        let file = FileRef::new(&bytes).expect("fixture should parse");
        let err = file
            .morph("abc", "xyz")
            .expect_err("multi-font collection should require an explicit index");

        assert!(matches!(err, MorphError::CollectionIndexRequired(count) if count > 1));
    }
}
