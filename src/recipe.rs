//! Serializable recipe support for reusing morph rules and options.

use serde::{Deserialize, Serialize};
use toml::{
    de::Error as DeError,
    from_str,
    ser::{Error as SerError, to_string_pretty},
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

/// A reusable morph recipe.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    /// Morphing options.
    #[serde(default)]
    pub options: MorphOptions,
    /// Morph rules in application order.
    #[serde(default)]
    pub rules: Vec<OwnedMorphRule>,
}

impl Recipe {
    /// Creates a new recipe from morph options and rules.
    #[must_use]
    pub fn new(options: MorphOptions, rules: Vec<OwnedMorphRule>) -> Self {
        Self { options, rules }
    }

    /// Parses a recipe from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, DeError> {
        from_str(toml_str)
    }

    /// Serializes the recipe to a TOML string.
    pub fn to_toml(&self) -> Result<String, SerError> {
        to_string_pretty(self)
    }

    /// Returns the recipe rules as borrowed morph rules.
    #[must_use]
    pub fn morph_rules(&self) -> Vec<MorphRule<'_>> {
        self.rules
            .iter()
            .map(|rule| MorphRule::new(&rule.from, &rule.to))
            .collect()
    }

    /// Returns the recipe options as morph options.
    #[must_use]
    pub const fn morph_options(&self) -> MorphOptions {
        self.options
    }
}

/// Options for morphing a font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct MorphOptions {
    /// Whether to require a word boundary before the matched source word.
    ///
    /// ## Example
    ///
    /// Say we want to morph "banana" to "orange". With start matching enabled,
    /// `xbanana` will not be affected; with it disabled, `xbanana` can be
    /// rendered as `xorange`.
    #[serde(default = "truthy")]
    pub word_match_start: bool,
    /// Whether to require a word boundary after the matched source word.
    ///
    /// ## Example
    ///
    /// Say we want to morph "banana" to "orange". With end matching enabled,
    /// `bananas` will not be affected; with it disabled, `bananas` can be
    /// rendered as `oranges`.
    #[serde(default = "truthy")]
    pub word_match_end: bool,
    /// Whether to skip rules that reference missing glyphs instead of failing.
    #[serde(default = "falsy")]
    pub skip_missing_glyphs: bool,
}

impl Default for MorphOptions {
    fn default() -> Self {
        Self {
            word_match_start: true,
            word_match_end: true,
            skip_missing_glyphs: false,
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl MorphOptions {
    /// Creates a new [`MorphOptions`].
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    #[must_use]
    #[allow(
        clippy::missing_const_for_fn,
        reason = "wasm_bindgen doesn't support const fns"
    )]
    pub fn new(word_match_start: bool, word_match_end: bool, skip_missing_glyphs: bool) -> Self {
        Self {
            word_match_start,
            word_match_end,
            skip_missing_glyphs,
        }
    }
}

/// A single morph rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MorphRule<'a> {
    /// Source word.
    pub from: &'a str,
    /// Target word.
    pub to: &'a str,
}

impl<'a> MorphRule<'a> {
    /// Creates a new [`MorphRule`].
    #[must_use]
    pub const fn new(from: &'a str, to: &'a str) -> Self {
        Self { from, to }
    }
}

/// An owned version of [`MorphRule`] for serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedMorphRule {
    /// Source word.
    pub from: String,
    /// Target word.
    pub to: String,
}

impl OwnedMorphRule {
    /// Creates a new [`OwnedMorphRule`].
    #[must_use]
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

const fn falsy() -> bool {
    false
}

const fn truthy() -> bool {
    true
}
