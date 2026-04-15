//! Helpers for `1 -> N` substitutions.

use read_fonts::{tables::layout::LookupFlag, types::GlyphId16};
use write_fonts::tables::{
    gsub::{Gsub, MultipleSubstFormat1, Sequence, SubstitutionLookup},
    layout::{Lookup, SequenceLookupRecord},
    varc::CoverageTable,
};

use std::collections::BTreeMap;

use super::{MorphError, shared::SharedLookupCache};

/// Reuses non-conflicting `1 -> N` lookups across rules.
#[derive(Debug)]
pub struct MultipleSubstitutionCache {
    cache: SharedLookupCache<GlyphId16, Vec<GlyphId16>>,
}

impl Default for MultipleSubstitutionCache {
    fn default() -> Self {
        Self {
            cache: SharedLookupCache::new(build_multiple_lookup),
        }
    }
}

impl MultipleSubstitutionCache {
    /// Builds a sequence record for a `1 -> N` substitution.
    pub fn sequence_record(
        &mut self,
        gsub: &mut Gsub,
        sequence_index: usize,
        src: GlyphId16,
        dst: &[GlyphId16],
    ) -> Result<SequenceLookupRecord, MorphError> {
        let lookup_index = self.cache.lookup_index(gsub, src, dst.to_vec())?;
        let sequence_index = u16::try_from(sequence_index)
            .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
        Ok(SequenceLookupRecord::new(sequence_index, lookup_index))
    }
}

fn build_multiple_lookup(mappings: &BTreeMap<GlyphId16, Vec<GlyphId16>>) -> SubstitutionLookup {
    let coverage = CoverageTable::format_1(mappings.keys().copied().collect());
    let sequences = mappings
        .values()
        .map(|dst| Sequence::new(dst.clone()))
        .collect();
    let subtable = MultipleSubstFormat1::new(coverage, sequences);
    SubstitutionLookup::Multiple(Lookup::new(LookupFlag::empty(), vec![subtable]))
}
