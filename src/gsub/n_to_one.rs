//! Helpers for `N -> 1` substitutions.

use read_fonts::{tables::layout::LookupFlag, types::GlyphId16};
use write_fonts::tables::{
    gsub::{Gsub, Ligature, LigatureSet, LigatureSubstFormat1, SubstitutionLookup},
    layout::{Lookup, SequenceLookupRecord},
    varc::CoverageTable,
};

use super::{MorphError, push_lookup};

/// Builds a sequence record for an `N -> 1` substitution.
pub fn build_n_to_one_record(
    gsub: &mut Gsub,
    sequence_index: usize,
    src: &[GlyphId16],
    dst: GlyphId16,
) -> Result<SequenceLookupRecord, MorphError> {
    let lookup_index = push_lookup(gsub, create_ligature_substitution_lookup(src, dst))?;
    let sequence_index = u16::try_from(sequence_index)
        .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
    Ok(SequenceLookupRecord::new(sequence_index, lookup_index))
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
