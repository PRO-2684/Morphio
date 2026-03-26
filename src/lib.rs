//! # Morphio
//!
//! Morphs the font, so one word renders as another.
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
//! Then, call the [`Morphio::morph`] method on the parsed font, passing in the two words you want to morph between.
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
//! For multiple rules, use [`Morphio::morph_many`] with [`MorphRule`]. Rules are applied in the order provided.
//! Chained or circular rule sets are not currently analyzed or rejected.
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
#![allow(clippy::multiple_crate_versions, reason = "Dependency")]

mod error;
mod font;
mod gsub;
mod recipe;
mod ttc;

#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Object, Reflect};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

pub use error::MorphError;
use read_fonts::{FileRef, FontRef};
pub use recipe::{MorphOptions, MorphRule, OwnedMorphRule, Recipe};
use ttc::build_ttc;
use write_fonts::FontBuilder;

/// The main trait for "morphing" text.
pub trait Morphio {
    /// Patch the font so it renders `from_word` as `to_word`, returning the rebuilt font bytes. Note that the two words:
    ///
    /// - Must be non-empty
    /// - Must be fully supported by the font (i.e. all glyphs must be present)
    ///
    /// If multiple fonts are present (e.g. in a TTC), all fonts will be patched.
    ///
    /// ## Errors
    ///
    /// See the [`MorphError`] enum for possible error cases.
    fn morph(&self, from_word: &str, to_word: &str) -> Result<Vec<u8>, MorphError> {
        self.morph_many(&[MorphRule::new(from_word, to_word)])
    }

    /// Patch the font with options, so it renders `from_word` as `to_word`, returning the rebuilt font bytes. Note that the two words:
    ///
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
        options: MorphOptions,
    ) -> Result<Vec<u8>, MorphError> {
        self.morph_many_with_options(&[MorphRule::new(from_word, to_word)], options)
    }

    /// Patch the font with multiple rules, returning the rebuilt font bytes.
    ///
    /// Rules are applied in the order provided. Chained or circular rule sets
    /// are not currently analyzed or rejected.
    ///
    /// ## Errors
    ///
    /// See the [`MorphError`] enum for possible error cases.
    fn morph_many(&self, rules: &[MorphRule<'_>]) -> Result<Vec<u8>, MorphError> {
        self.morph_many_with_options(rules, MorphOptions::default())
    }

    /// Patch the font with multiple rules and options, returning the rebuilt font bytes.
    ///
    /// Rules are applied in the order provided. Chained or circular rule sets
    /// are not currently analyzed or rejected.
    ///
    /// ## Errors
    ///
    /// See the [`MorphError`] enum for possible error cases.
    fn morph_many_with_options(
        &self,
        rules: &[MorphRule<'_>],
        options: MorphOptions,
    ) -> Result<Vec<u8>, MorphError>;

    /// Patch the font with a recipe, returning the rebuilt font bytes.
    ///
    /// ## Errors
    ///
    /// See the [`MorphError`] enum for possible error cases.
    fn morph_with_recipe(&self, recipe: &Recipe) -> Result<Vec<u8>, MorphError> {
        let rules = recipe
            .rules
            .iter()
            .map(|rule| MorphRule::new(&rule.from, &rule.to))
            .collect::<Vec<_>>();
        self.morph_many_with_options(&rules, recipe.options)
    }
}

impl Morphio for FontRef<'_> {
    fn morph_many_with_options(
        &self,
        rules: &[MorphRule<'_>],
        options: MorphOptions,
    ) -> Result<Vec<u8>, MorphError> {
        morph_font(self.clone(), rules, options)
    }
}

impl Morphio for FileRef<'_> {
    fn morph_many_with_options(
        &self,
        rules: &[MorphRule<'_>],
        options: MorphOptions,
    ) -> Result<Vec<u8>, MorphError> {
        match self {
            Self::Font(font) => font.morph_many_with_options(rules, options),
            Self::Collection(collection) => {
                let fonts = collection
                    .iter()
                    .map(|font| font.map_err(MorphError::Read))
                    .map(|font| font.and_then(|font| font.morph_many_with_options(rules, options)))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(build_ttc(&fonts))
            }
        }
    }
}

fn morph_font(
    font: FontRef<'_>,
    rules: &[MorphRule<'_>],
    options: MorphOptions,
) -> Result<Vec<u8>, MorphError> {
    let resolved_rules = font::resolve_rules(&font, rules, options.skip_missing_glyphs)?;
    let gsub = gsub::patch_gsub(&font, &resolved_rules, options)?;

    let mut builder = FontBuilder::new();
    builder.add_table(&gsub)?;
    builder.copy_missing_tables(font);

    Ok(builder.build())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = morphFontMany)]
/// WebAssembly entry point that morphs the provided font bytes using multiple rules.
pub fn morph_font_many_wasm(
    font_data: &[u8],
    rules: Array,
    options: MorphOptions,
) -> Result<Vec<u8>, JsValue> {
    let owned_rules = rules
        .iter()
        .map(|entry| {
            let pair = Array::from(&entry);
            if pair.length() != 2 {
                return Err(JsValue::from_str(
                    "each morph rule must be a two-item array: [from, to]",
                ));
            }
            let from = pair
                .get(0)
                .as_string()
                .ok_or_else(|| JsValue::from_str("rule source must be a string"))?;
            let to = pair
                .get(1)
                .as_string()
                .ok_or_else(|| JsValue::from_str("rule target must be a string"))?;
            Ok(OwnedMorphRule { from, to })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let rules = owned_rules
        .iter()
        .map(|rule| MorphRule::new(&rule.from, &rule.to))
        .collect::<Vec<_>>();

    let file = FileRef::new(font_data).map_err(|err| JsValue::from_str(&err.to_string()))?;
    file.morph_many_with_options(&rules, options)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = parseRecipe)]
/// WebAssembly entry point that parses a recipe TOML string into JS-friendly data.
pub fn parse_recipe_wasm(recipe_toml: &str) -> Result<JsValue, JsValue> {
    let recipe =
        Recipe::from_toml(recipe_toml).map_err(|err| JsValue::from_str(&err.to_string()))?;
    let result = Object::new();
    Reflect::set(
        &result,
        &JsValue::from_str("options"),
        &JsValue::from(recipe.options),
    )?;

    let rules = Array::new();
    for rule in recipe.rules {
        let pair = Array::new();
        pair.push(&JsValue::from(rule.from));
        pair.push(&JsValue::from(rule.to));
        rules.push(&pair);
    }
    Reflect::set(&result, &JsValue::from_str("rules"), &rules)?;

    Ok(result.into())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = serializeRecipe)]
/// WebAssembly entry point that serializes structured recipe data to TOML.
pub fn serialize_recipe_wasm(rules: Array, options: MorphOptions) -> Result<String, JsValue> {
    let owned_rules = rules
        .iter()
        .map(|entry| {
            let pair = Array::from(&entry);
            if pair.length() != 2 {
                return Err(JsValue::from_str(
                    "each morph rule must be a two-item array: [from, to]",
                ));
            }
            let from = pair
                .get(0)
                .as_string()
                .ok_or_else(|| JsValue::from_str("rule source must be a string"))?;
            let to = pair
                .get(1)
                .as_string()
                .ok_or_else(|| JsValue::from_str("rule target must be a string"))?;
            Ok(OwnedMorphRule { from, to })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Recipe::new(options, owned_rules)
        .to_toml()
        .map_err(|err| JsValue::from_str(&err.to_string()))
}
