//! Primitive helpers for `1 -> 1` substitutions.

use read_fonts::types::GlyphId16;
use write_fonts::tables::{
    gsub::{SingleSubst, SubstitutionLookup},
    layout::{CoverageTable, Lookup, LookupFlag, SequenceLookupRecord},
};

use std::collections::BTreeMap;

use super::{MorphError, shared::SharedLookupCache};

/// Reuses non-conflicting `1 -> 1` lookups across rules.
#[derive(Debug)]
pub struct SingleSubstitutionCache {
    cache: SharedLookupCache<GlyphId16, GlyphId16>,
}

impl Default for SingleSubstitutionCache {
    fn default() -> Self {
        Self {
            cache: SharedLookupCache::new(build_single_lookup),
        }
    }
}

impl SingleSubstitutionCache {
    /// Returns a sequence record for `src -> dst`, creating the underlying
    /// single-substitution lookup on demand. Identity mappings are skipped.
    pub fn sequence_record(
        &mut self,
        gsub: &mut write_fonts::tables::gsub::Gsub,
        sequence_index: usize,
        src: GlyphId16,
        dst: GlyphId16,
    ) -> Result<Option<SequenceLookupRecord>, MorphError> {
        if src == dst {
            return Ok(None);
        }

        let lookup_index = self.cache.lookup_index(gsub, src, dst)?;
        let sequence_index = u16::try_from(sequence_index)
            .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
        Ok(Some(SequenceLookupRecord::new(
            sequence_index,
            lookup_index,
        )))
    }
}

fn build_single_lookup(mappings: &BTreeMap<GlyphId16, GlyphId16>) -> SubstitutionLookup {
    let coverage = CoverageTable::format_1(mappings.keys().copied().collect());
    let subtable = SingleSubst::format_2(coverage, mappings.values().copied().collect());
    SubstitutionLookup::Single(Lookup::new(LookupFlag::empty(), vec![subtable]))
}
