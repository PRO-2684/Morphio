//! Helpers for `N -> 1` substitutions.

use read_fonts::{tables::layout::LookupFlag, types::GlyphId16};
use write_fonts::tables::{
    gsub::{Gsub, Ligature, LigatureSet, LigatureSubstFormat1, SubstitutionLookup},
    layout::{Lookup, RangeRecord, SequenceLookupRecord},
    varc::CoverageTable,
};

use super::{MorphError, create_contextual_lookup, push_lookup};

/// Appends a contextual `N -> 1` substitution over `matched_glyphs`.
pub fn append_n_to_one_lookup(
    gsub: &mut Gsub,
    matched_glyphs: &[GlyphId16],
    sequence_index: usize,
    src: &[GlyphId16],
    dst: GlyphId16,
    word_glyph_ranges: Vec<RangeRecord>,
    word_match_start: bool,
    word_match_end: bool,
) -> Result<Vec<u16>, MorphError> {
    let lookup_index = push_lookup(gsub, create_ligature_substitution_lookup(src, dst))?;
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

fn create_ligature_substitution_lookup(src: &[GlyphId16], dst: GlyphId16) -> SubstitutionLookup {
    let (first, rest) = src
        .split_first()
        .expect("ligature substitution requires a non-empty source sequence");
    let coverage = CoverageTable::format_1(vec![*first]);
    let ligature = Ligature::new(dst, rest.to_vec());
    let subtable = LigatureSubstFormat1::new(coverage, vec![LigatureSet::new(vec![ligature])]);
    SubstitutionLookup::Ligature(Lookup::new(LookupFlag::empty(), vec![subtable]))
}
