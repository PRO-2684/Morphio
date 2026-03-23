#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

//! Utilities for patching OpenType fonts so one word renders as another.

mod font;
mod gsub;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use read_fonts::{FontRef, ReadError};
use write_fonts::{BuilderError, FontBuilder};

/// An owned font payload that can reference a specific entry in a font collection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontAsset {
    bytes: Vec<u8>,
    index: u32,
}

impl FontAsset {
    /// Create a new font asset from raw bytes and a collection index.
    pub fn new(bytes: Vec<u8>, index: u32) -> Self {
        Self { bytes, index }
    }

    /// Borrow the raw font bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the asset and return the raw font bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Return the selected collection index.
    pub fn index(&self) -> u32 {
        self.index
    }
}

/// The main trait for "morphing" text.
pub trait TextMorph {
    /// Borrow the font bytes that should be patched.
    fn font_bytes(&self) -> &[u8];

    /// Return the selected collection index.
    ///
    /// Implementations that only support plain sfnt fonts can keep the default `0`.
    fn font_index(&self) -> u32 {
        0
    }

    /// Replace the owned font payload after patching.
    ///
    /// The rebuilt payload is always a single-font sfnt, so the new index is `0`.
    fn replace_font(&mut self, bytes: Vec<u8>, index: u32);

    /// Patch the font, so it shows `from_word` as `to_word`.
    ///
    /// The two words must have the same length, must be non-empty, and the font must
    /// contain glyphs for all characters in both words.
    fn morph(&mut self, from_word: &str, to_word: &str) -> Result<(), MorphError> {
        let font =
            FontRef::from_index(self.font_bytes(), self.font_index()).map_err(MorphError::Read)?;
        let (from_glyphs, to_glyphs) = font::validate_words(&font, from_word, to_word)?;
        let gsub = gsub::patch_gsub(&font, &from_glyphs, &to_glyphs)?;

        let mut builder = FontBuilder::new();
        builder
            .add_table(&gsub)
            .map_err(MorphError::Builder)?
            .copy_missing_tables(font);

        self.replace_font(builder.build(), 0);
        Ok(())
    }
}

impl TextMorph for Vec<u8> {
    fn font_bytes(&self) -> &[u8] {
        self.as_slice()
    }

    fn replace_font(&mut self, bytes: Vec<u8>, _: u32) {
        *self = bytes;
    }
}

impl TextMorph for FontAsset {
    fn font_bytes(&self) -> &[u8] {
        self.bytes()
    }

    fn font_index(&self) -> u32 {
        self.index
    }

    fn replace_font(&mut self, bytes: Vec<u8>, index: u32) {
        self.bytes = bytes;
        self.index = index;
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

#[cfg(test)]
mod tests {
    use read_fonts::TableProvider;

    use super::*;

    fn fixture() -> FontAsset {
        let bytes = std::fs::read("tests/fonts/msyh.ttc").expect("fixture font should exist");
        FontAsset::new(bytes, 0)
    }

    #[test]
    fn rejects_different_lengths() {
        let mut font = fixture();
        let err = font.morph("abc", "xy").expect_err("expected a validation error");
        assert!(matches!(err, MorphError::DifferentLengths));
    }

    #[test]
    fn rejects_empty_words() {
        let mut font = fixture();
        let err = font.morph("", "").expect_err("expected a validation error");
        assert!(matches!(err, MorphError::EmptyWord));
    }

    #[test]
    fn rejects_missing_glyphs() {
        let mut font = fixture();
        let err = font.morph("abc", "😀bc").expect_err("expected a missing glyph error");
        assert!(matches!(err, MorphError::MissingGlyph('😀')));
    }

    #[test]
    fn builds_a_font_with_calt_feature() {
        let mut font = fixture();
        font.morph("abc", "xyz").expect("font should patch successfully");
        assert_eq!(font.index(), 0);

        let rebuilt = FontRef::new(font.bytes()).expect("patched font should parse as sfnt");
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
}
