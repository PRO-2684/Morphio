//! Helpers for `1 -> N` substitutions.

use std::collections::BTreeMap;

use read_fonts::{tables::layout::LookupFlag, types::GlyphId16};
use write_fonts::tables::{
    gsub::{Gsub, MultipleSubstFormat1, Sequence, SubstitutionLookup},
    layout::{Lookup, SequenceLookupRecord},
    varc::CoverageTable,
};

use super::{MorphError, push_lookup};

#[derive(Debug)]
struct MultipleLookupBucket {
    lookup_index: u16,
    mappings: BTreeMap<GlyphId16, Vec<GlyphId16>>,
}

/// Reuses non-conflicting `1 -> N` lookups across rules.
#[derive(Debug, Default)]
pub struct MultipleSubstitutionCache {
    lookup_indices: BTreeMap<(GlyphId16, Vec<GlyphId16>), u16>,
    buckets: Vec<MultipleLookupBucket>,
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
        let key = (src, dst.to_vec());
        let lookup_index = if let Some(index) = self.lookup_indices.get(&key) {
            *index
        } else {
            let index = self.insert_mapping(gsub, src, dst)?;
            self.lookup_indices.insert(key, index);
            index
        };
        let sequence_index = u16::try_from(sequence_index)
            .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
        Ok(SequenceLookupRecord::new(sequence_index, lookup_index))
    }

    fn insert_mapping(
        &mut self,
        gsub: &mut Gsub,
        src: GlyphId16,
        dst: &[GlyphId16],
    ) -> Result<u16, MorphError> {
        if let Some(bucket) = self
            .buckets
            .iter_mut()
            .find(|bucket| !bucket.mappings.contains_key(&src))
        {
            bucket.mappings.insert(src, dst.to_vec());
            overwrite_lookup(gsub, bucket.lookup_index, build_multiple_lookup(&bucket.mappings));
            return Ok(bucket.lookup_index);
        }

        let mut mappings = BTreeMap::new();
        mappings.insert(src, dst.to_vec());
        let lookup_index = push_lookup(gsub, build_multiple_lookup(&mappings))?;
        self.buckets.push(MultipleLookupBucket {
            lookup_index,
            mappings,
        });
        Ok(lookup_index)
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

fn overwrite_lookup(gsub: &mut Gsub, lookup_index: u16, lookup: SubstitutionLookup) {
    gsub.lookup_list.lookups[usize::from(lookup_index)] = lookup.into();
}
