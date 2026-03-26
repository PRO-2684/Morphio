//! Helpers for `1 -> N` substitutions.

use read_fonts::{tables::layout::LookupFlag, types::GlyphId16};
use write_fonts::tables::{
    gsub::{Gsub, MultipleSubstFormat1, Sequence, SubstitutionLookup},
    layout::{Lookup, SequenceLookupRecord},
    varc::CoverageTable,
};

use super::{MorphError, push_lookup};

/// Builds a sequence record for a `1 -> N` substitution.
pub fn build_one_to_n_record(
    gsub: &mut Gsub,
    sequence_index: usize,
    src: GlyphId16,
    dst: &[GlyphId16],
) -> Result<SequenceLookupRecord, MorphError> {
    let lookup_index = push_lookup(gsub, create_multiple_substitution_lookup(src, dst))?;
    let sequence_index = u16::try_from(sequence_index)
        .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
    Ok(SequenceLookupRecord::new(sequence_index, lookup_index))
}

fn create_multiple_substitution_lookup(src: GlyphId16, dst: &[GlyphId16]) -> SubstitutionLookup {
    let coverage = CoverageTable::format_1(vec![src]);
    let subtable = MultipleSubstFormat1::new(coverage, vec![Sequence::new(dst.to_vec())]);
    SubstitutionLookup::Multiple(Lookup::new(LookupFlag::empty(), vec![subtable]))
}
