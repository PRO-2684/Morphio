//! Helpers for `1 -> N` substitutions.

use read_fonts::{tables::layout::LookupFlag, types::GlyphId16};
use write_fonts::tables::{
    gsub::{Gsub, MultipleSubstFormat1, Sequence, SubstitutionLookup},
    layout::{Lookup, RangeRecord, SequenceLookupRecord},
    varc::CoverageTable,
};

use super::{MorphError, create_contextual_lookup, push_lookup};

/// Appends a contextual `1 -> N` substitution over `matched_glyphs`.
pub fn append_one_to_n_lookup(
    gsub: &mut Gsub,
    matched_glyphs: &[GlyphId16],
    sequence_index: usize,
    src: GlyphId16,
    dst: &[GlyphId16],
    word_glyph_ranges: Vec<RangeRecord>,
    word_match_start: bool,
    word_match_end: bool,
) -> Result<Vec<u16>, MorphError> {
    let lookup_index = push_lookup(gsub, create_multiple_substitution_lookup(src, dst))?;
    let sequence_index = u16::try_from(sequence_index)
        .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
    let contextual_lookup = create_contextual_lookup(
        matched_glyphs,
        word_glyph_ranges,
        vec![SequenceLookupRecord::new(sequence_index, lookup_index)],
        word_match_start,
        word_match_end,
    );
    let contextual_lookup_index = push_lookup(gsub, contextual_lookup)?;
    Ok(vec![contextual_lookup_index])
}

fn create_multiple_substitution_lookup(src: GlyphId16, dst: &[GlyphId16]) -> SubstitutionLookup {
    let coverage = CoverageTable::format_1(vec![src]);
    let subtable = MultipleSubstFormat1::new(coverage, vec![Sequence::new(dst.to_vec())]);
    SubstitutionLookup::Multiple(Lookup::new(LookupFlag::empty(), vec![subtable]))
}
