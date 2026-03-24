//! # Morphio
//!
//! Morphs the font, so it renders worda as wordb.
//!
//! ## Usage
//!
//! ### Loading and parsing a font
//!
//! You should load the font file as bytes (e.g. using [`std::fs::read`]) and parse it using [`read_fonts::FontRef`].
//!
//! ```rust
//! use read_fonts::FontRef;
//!
//! let path_to_font = "tests/fonts/INKFREE.TTF";
//! let font_data = std::fs::read(path_to_font).unwrap();
//! let font = FontRef::new(&font_data).unwrap();
//! ```
//!
//! ### Morphing the font
//!
//! Then, call the [`Morphio::morph`] method on the parsed font, passing in the two words you want to morph between. Note that the two words must be of the same length.
//!
//! ```rust
//! # use read_fonts::FontRef;
//! use morphio::Morphio;
//!
//! # let path_to_font = "tests/fonts/INKFREE.TTF";
//! # let font_data = std::fs::read(path_to_font).unwrap();
//! # let font = FontRef::new(&font_data).unwrap();
//! let morphed_font_data = font.morph("worda", "wordb").unwrap(); // Of type `Vec<u8>`, containing the bytes of the morphed font
//! ```
//!
//! ### Verifying the morphed font
//!
//! We can verify the result by trying to parse the morphed font and check the expected GSUB feature is present.
//!
//! ```rust
//! # use read_fonts::FontRef;
//! # use morphio::Morphio;
//! use read_fonts::{TableProvider, types::Tag};
//!
//! # let path_to_font = "tests/fonts/INKFREE.TTF";
//! # let font_data = std::fs::read(path_to_font).unwrap();
//! # let font = FontRef::new(&font_data).unwrap();
//! # let morphed_font_data = font.morph("worda", "wordb").unwrap();
//! let morphed_font = FontRef::new(&morphed_font_data).unwrap();
//! let gsub = morphed_font.gsub().unwrap();
//! let feature_list = gsub.feature_list().unwrap();
//! let has_calt = feature_list.feature_records().iter().any(|record| record.feature_tag() == Tag::new(b"calt"));
//! assert!(has_calt, "morphed font should expose a calt feature for the morphing substitution");
//! ```
//!
//! ### Font collection support
//!
//! [`Morphio::morph`] can also be called on a font collection (e.g. a TTC). In this case, the morphing will be applied to every font in the collection, and the result will still be a valid font collection containing the morphed fonts.
//!
//! To load a font collection, use [`read_fonts::FileRef`] instead of [`read_fonts::FontRef`].
//!
//! ```rust
//! use read_fonts::FileRef;
//! use morphio::Morphio;
//!
//! let path_to_collection = "tests/fonts/msyh.ttc";
//! let collection_data = std::fs::read(path_to_collection).unwrap();
//! let collection = FileRef::new(&collection_data).unwrap();
//! let morphed_collection_data = collection.morph("abc", "xyz").unwrap();
//! let morphed_collection = FileRef::new(&morphed_collection_data).unwrap(); // Verify that it can be parsed
//! ```
//!
//! ## WebAssembly
//!
//! This library can also be compiled to WebAssembly, providing the [`morph_font_wasm`] function as the entry point (the JavaScript name is `morphFont`). See it in action in the [demo](https://pro-2684.github.io/Morphio/), and the source code is available in [the GitHub repo](https://github.com/PRO-2684/Morphio).

#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

mod error;
mod font;
mod gsub;
mod ttc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

pub use error::MorphError;
use read_fonts::{FileRef, FontRef};
use ttc::build_ttc;
use write_fonts::FontBuilder;

/// Options for morphing a font.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct MorphOptions {
    /// Whether to enable word matching.
    ///
    /// ## Example
    ///
    /// Say we want to morph "banana" to "orange". With word match enabled, "bananas" won't be affected; with it disabled, "bananas" will be rendered as "oranges".
    pub word_match: bool,
}

impl Default for MorphOptions {
    fn default() -> Self {
        Self { word_match: true }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl MorphOptions {
    /// Creates a new [`MorphOptions`].
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new(word_match: bool) -> Self {
        Self { word_match }
    }
}

/// The main trait for "morphing" text.
pub trait Morphio {
    /// Patch the font so it renders `from_word` as `to_word`, returning the rebuilt font bytes. Note that the two words:
    ///
    /// - Must have the same length
    /// - Must be non-empty
    /// - Must be fully supported by the font (i.e. all glyphs must be present)
    ///
    /// If multiple fonts are present (e.g. in a TTC), all fonts will be patched.
    ///
    /// ## Errors
    ///
    /// See the [`MorphError`] enum for possible error cases.
    fn morph(&self, from_word: &str, to_word: &str) -> Result<Vec<u8>, MorphError> {
        self.morph_with_options(from_word, to_word, &Default::default())
    }

    /// Patch the font with options, so it renders `from_word` as `to_word`, returning the rebuilt font bytes. Note that the two words:
    ///
    /// - Must have the same length
    /// - Must be non-empty
    /// - Must be fully supported by the font (i.e. all glyphs must be present)
    ///
    /// If multiple fonts are present (e.g. in a TTC), all fonts will be patched.
    ///
    /// ## Errors
    ///
    /// See the [`MorphError`] enum for possible error cases.
    fn morph_with_options(
        &self,
        from_word: &str,
        to_word: &str,
        options: &MorphOptions,
    ) -> Result<Vec<u8>, MorphError>;
}

impl Morphio for FontRef<'_> {
    fn morph_with_options(
        &self,
        from_word: &str,
        to_word: &str,
        options: &MorphOptions,
    ) -> Result<Vec<u8>, MorphError> {
        morph_font(self.clone(), from_word, to_word, options)
    }
}

impl Morphio for FileRef<'_> {
    fn morph_with_options(
        &self,
        from_word: &str,
        to_word: &str,
        options: &MorphOptions,
    ) -> Result<Vec<u8>, MorphError> {
        match self {
            Self::Font(font) => font.morph_with_options(from_word, to_word, options),
            Self::Collection(collection) => {
                let fonts = collection
                    .iter()
                    .map(|font| font.map_err(MorphError::Read))
                    .map(|font| {
                        font.and_then(|font| font.morph_with_options(from_word, to_word, options))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(build_ttc(fonts))
            }
        }
    }
}

fn morph_font(
    font: FontRef<'_>,
    from_word: &str,
    to_word: &str,
    options: &MorphOptions,
) -> Result<Vec<u8>, MorphError> {
    let (from_glyphs, to_glyphs) = font::validate_words(&font, from_word, to_word)?;
    let gsub = gsub::patch_gsub(&font, &from_glyphs, &to_glyphs, options)?;

    let mut builder = FontBuilder::new();
    builder.add_table(&gsub)?.copy_missing_tables(font);

    Ok(builder.build())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = morphFont)]
/// WebAssembly entry point that morphs the provided font bytes and returns rebuilt font data.
pub fn morph_font_wasm(
    font_data: &[u8],
    from_word: &str,
    to_word: &str,
    options: MorphOptions,
) -> Result<Vec<u8>, JsValue> {
    let file = FileRef::new(font_data).map_err(|err| JsValue::from_str(&err.to_string()))?;
    file.morph_with_options(from_word, to_word, &options)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

#[cfg(test)]
mod tests {
    use read_fonts::{TableProvider, types::Tag};

    use super::*;

    fn fixture_bytes() -> Vec<u8> {
        std::fs::read("tests/fonts/msyh.ttc").expect("fixture font should exist")
    }

    fn impact_bytes() -> Vec<u8> {
        std::fs::read("tests/fonts/IMPACT.TTF").expect("impact fixture should exist")
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

    #[test]
    fn impact_adds_calt_to_all_latin_langsys_records() {
        let bytes = impact_bytes();
        let font = FontRef::new(&bytes).expect("impact fixture should parse");
        let morphed = font
            .morph("banana", "orange")
            .expect("impact font should patch successfully");
        let rebuilt = FontRef::new(&morphed).expect("patched impact should parse");
        let gsub = rebuilt.gsub().expect("patched impact should contain GSUB");
        let feature_list = gsub
            .feature_list()
            .expect("patched GSUB should contain a feature list");
        let calt_index = feature_list
            .feature_records()
            .iter()
            .position(|record| record.feature_tag() == Tag::new(b"calt"))
            .expect("patched impact should expose calt");
        let calt_index =
            u16::try_from(calt_index).expect("feature index should fit into u16 for test");

        let script_list = gsub
            .script_list()
            .expect("patched GSUB should contain scripts");
        let latn = script_list
            .script_records()
            .iter()
            .find(|record| record.script_tag() == Tag::new(b"latn"))
            .expect("patched impact should keep latn script");
        let script = latn
            .script(script_list.offset_data())
            .expect("latn script should resolve");

        let default_lang_sys = script
            .default_lang_sys()
            .expect("latn script should have a default langsys")
            .expect("latn default langsys should resolve");
        assert!(
            default_lang_sys
                .feature_indices()
                .iter()
                .any(|index| index.get() == calt_index),
            "latn default langsys should include calt",
        );

        for record in script.lang_sys_records() {
            let lang_sys = record
                .lang_sys(script.offset_data())
                .expect("langsys should resolve");
            assert!(
                lang_sys
                    .feature_indices()
                    .iter()
                    .any(|index| index.get() == calt_index),
                "every latn langsys should include calt",
            );
        }
    }
}
