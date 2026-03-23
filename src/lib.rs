#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

pub mod ext;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

use read_fonts::{FontRef, ReadError, TableProvider, tables::cmap::Cmap, types::NameId};
use write_fonts::{
    BuilderError, FontBuilder, NullableOffsetMarker, OffsetMarker,
    from_obj::ToOwnedTable,
    tables::{
        gsub::{Gsub, SingleSubst, SubstitutionChainContext, SubstitutionLookup},
        layout::{
            ChainedSequenceContext, ChainedSequenceContextFormat1, ChainedSequenceRule,
            ChainedSequenceRuleSet, CoverageFormat1, Lookup, SequenceLookupRecord,
        },
        name::Name,
    },
};

/// The main trait for "morphing" text.
pub trait TextMorph {
    // todo
    /// Patch the font, so it shows worda (from_word) as wordb (to_word). Note that the two words must be the same length, and the font must contain glyphs for all characters in both words.
    fn morph(&mut self, from_word: &str, to_word: &str) -> Result<(), MorphError> {
        todo!()
    }
}

/// Errors that can occur during morphing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MorphError {
    /// The two words have different lengths.
    DifferentLengths,
    /// The font is missing a glyph for a character in one of the words.
    MissingGlyph(char),
    /// An error occurred while reading the font.
    ReadError,
    /// An error occurred while building the font.
    BuilderError,
}
