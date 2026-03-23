#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

//! Utilities for patching OpenType fonts so one word renders as another.

mod font;
mod gsub;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use read_fonts::{FileRef, FontRef, ReadError};
use write_fonts::{BuilderError, FontBuilder};

/// The main trait for "morphing" text.
pub trait Morphio {
    /// Patch the font so it shows `from_word` as `to_word`, returning the rebuilt font bytes.
    ///
    /// The two words must have the same length, must be non-empty, and the font must contain glyphs for all characters in both words.
    ///
    /// If multiple fonts are present (e.g. in a TTC), all fonts will be patched.
    fn morph(&self, from_word: &str, to_word: &str) -> Result<Vec<u8>, MorphError>;
}

impl Morphio for FontRef<'_> {
    fn morph(&self, from_word: &str, to_word: &str) -> Result<Vec<u8>, MorphError> {
        morph_font(self.clone(), from_word, to_word)
    }
}

impl Morphio for FileRef<'_> {
    fn morph(&self, from_word: &str, to_word: &str) -> Result<Vec<u8>, MorphError> {
        match self {
            Self::Font(font) => font.morph(from_word, to_word),
            Self::Collection(collection) => {
                let fonts = collection
                    .iter()
                    .map(|font| font.map_err(MorphError::Read))
                    .map(|font| font.and_then(|font| morph_font(font, from_word, to_word)))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(build_ttc(fonts))
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

impl std::fmt::Display for MorphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DifferentLengths => {
                write!(f, "source and target words must have the same length")
            }
            Self::EmptyWord => write!(f, "source and target words must not be empty"),
            Self::MissingCmap => write!(f, "font does not contain a usable Unicode cmap"),
            Self::MissingGlyph(ch) => write!(f, "font is missing a glyph for '{ch}'"),
            Self::Read(err) => write!(f, "{err}"),
            Self::Builder(err) => write!(f, "failed to rebuild font: {}", err.inner),
        }
    }
}

impl std::error::Error for MorphError {}

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

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = morphFont)]
/// WebAssembly entry point that morphs the provided font bytes and returns rebuilt font data.
pub fn morph_font_wasm(
    font_data: &[u8],
    from_word: &str,
    to_word: &str,
) -> Result<Vec<u8>, JsValue> {
    let file = FileRef::new(font_data).map_err(|err| JsValue::from_str(&err.to_string()))?;
    file.morph(from_word, to_word)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Build a TTC from the given font bytes, rebasing all internal offsets by the appropriate amount. Implemented here because `write-fonts` [doesn't support TTC yet](https://github.com/googlefonts/fontations/blob/423de8c29d960f1d2dd691c325a1bf41dda8513e/write-fonts/src/font_builder.rs#L265).
fn build_ttc(mut fonts: Vec<Vec<u8>>) -> Vec<u8> {
    let header_len = 12 + fonts.len() * 4;
    let mut offsets = Vec::with_capacity(fonts.len());
    let mut offset = header_len as u32;

    for font in &fonts {
        offsets.push(offset);
        offset += align_4(font.len()) as u32;
    }

    for (font, offset) in fonts.iter_mut().zip(offsets.iter().copied()) {
        rebase_sfnt_offsets(font, offset);
    }

    let mut ttc = Vec::with_capacity(offset as usize);
    ttc.extend_from_slice(b"ttcf");
    ttc.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    ttc.extend_from_slice(&(fonts.len() as u32).to_be_bytes());
    for offset in offsets {
        ttc.extend_from_slice(&offset.to_be_bytes());
    }

    for font in fonts {
        ttc.extend_from_slice(&font);
        let padding = align_4(font.len()) - font.len();
        ttc.resize(ttc.len() + padding, 0);
    }

    ttc
}

fn align_4(len: usize) -> usize {
    (len + 3) & !3
}

fn rebase_sfnt_offsets(font: &mut [u8], delta: u32) {
    if font.len() < 12 {
        return;
    }

    let num_tables = u16::from_be_bytes([font[4], font[5]]) as usize;
    let records_start = 12;

    for i in 0..num_tables {
        let record_offset = records_start + i * 16;
        if record_offset + 12 > font.len() {
            return;
        }

        let table_offset = u32::from_be_bytes([
            font[record_offset + 8],
            font[record_offset + 9],
            font[record_offset + 10],
            font[record_offset + 11],
        ]);
        let rebased = table_offset.saturating_add(delta).to_be_bytes();
        font[record_offset + 8..record_offset + 12].copy_from_slice(&rebased);
    }
}

#[cfg(test)]
mod tests {
    use read_fonts::{TableProvider, types::Tag};

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
    fn file_ref_morphs_all_fonts_in_collection() {
        let bytes = fixture_bytes();
        let file = FileRef::new(&bytes).expect("fixture should parse");
        let morphed = file
            .morph("abc", "xyz")
            .expect("collection morph should patch every font");
        let rebuilt = FileRef::new(&morphed).expect("patched bytes should still be a valid file");

        let FileRef::Collection(collection) = rebuilt else {
            panic!("patched fixture should remain a collection");
        };
        assert_eq!(collection.len(), 2);

        for font in collection.iter() {
            let font = font.expect("collection member should parse");
            let gsub = font.gsub().expect("patched font should contain GSUB");
            let feature_list = gsub
                .feature_list()
                .expect("patched GSUB should contain a feature list");
            let has_calt = feature_list
                .feature_records()
                .iter()
                .any(|record| record.feature_tag() == Tag::new(b"calt"));
            assert!(
                has_calt,
                "every collection font should expose a calt feature"
            );
        }
    }
}
