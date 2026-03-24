//! Errors for Morphio.

use read_fonts::ReadError;
use std::fmt;
use write_fonts::BuilderError;

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
    /// Out of range trying to represent glyph ID as u16.
    GlyphIdOutOfRange(u32),
    /// An error occurred while reading the font.
    Read(ReadError),
    /// An error occurred while building the font.
    Builder(BuilderError),
}

impl MorphError {
    pub(crate) const fn malformed(message: &'static str) -> Self {
        Self::Read(ReadError::MalformedData(message))
    }
}

impl fmt::Display for MorphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DifferentLengths => {
                write!(f, "source and target words must have the same length")
            }
            Self::EmptyWord => write!(f, "source and target words must not be empty"),
            Self::MissingCmap => write!(f, "font does not contain a usable Unicode cmap"),
            Self::MissingGlyph(ch) => write!(f, "font is missing a glyph for '{ch}'"),
            Self::GlyphIdOutOfRange(id) => {
                write!(f, "glyph ID {id} is out of range for u16 (max 65535)")
            }
            Self::Read(err) => write!(f, "{err}"),
            Self::Builder(err) => write!(f, "failed to rebuild font: {}", err.inner),
        }
    }
}

impl std::error::Error for MorphError {}

impl From<ReadError> for MorphError {
    fn from(err: ReadError) -> Self {
        Self::Read(err)
    }
}

impl From<BuilderError> for MorphError {
    fn from(err: BuilderError) -> Self {
        Self::Builder(err)
    }
}
