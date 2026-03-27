//! Helpers for `N -> 1` substitutions.

use std::collections::BTreeMap;

use read_fonts::{tables::layout::LookupFlag, types::GlyphId16};
use write_fonts::tables::{
    gsub::{Gsub, Ligature, LigatureSet, LigatureSubstFormat1, SubstitutionLookup},
    layout::{Lookup, SequenceLookupRecord},
    varc::CoverageTable,
};

use super::{MorphError, push_lookup};

#[derive(Debug)]
struct LigatureLookupBucket {
    lookup_index: u16,
    mappings: BTreeMap<Vec<GlyphId16>, GlyphId16>,
}

/// Reuses non-conflicting `N -> 1` lookups across rules.
#[derive(Debug, Default)]
pub struct LigatureSubstitutionCache {
    lookup_indices: BTreeMap<(Vec<GlyphId16>, GlyphId16), u16>,
    buckets: Vec<LigatureLookupBucket>,
}

impl LigatureSubstitutionCache {
    /// Builds a sequence record for an `N -> 1` substitution.
    pub fn sequence_record(
        &mut self,
        gsub: &mut Gsub,
        sequence_index: usize,
        src: &[GlyphId16],
        dst: GlyphId16,
    ) -> Result<SequenceLookupRecord, MorphError> {
        let key = (src.to_vec(), dst);
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
        src: &[GlyphId16],
        dst: GlyphId16,
    ) -> Result<u16, MorphError> {
        if let Some(bucket) = self
            .buckets
            .iter_mut()
            .find(|bucket| !bucket.mappings.contains_key(src))
        {
            bucket.mappings.insert(src.to_vec(), dst);
            overwrite_lookup(gsub, bucket.lookup_index, build_ligature_lookup(&bucket.mappings));
            return Ok(bucket.lookup_index);
        }

        let mut mappings = BTreeMap::new();
        mappings.insert(src.to_vec(), dst);
        let lookup_index = push_lookup(gsub, build_ligature_lookup(&mappings))?;
        self.buckets.push(LigatureLookupBucket {
            lookup_index,
            mappings,
        });
        Ok(lookup_index)
    }
}

fn build_ligature_lookup(mappings: &BTreeMap<Vec<GlyphId16>, GlyphId16>) -> SubstitutionLookup {
    let mut grouped = BTreeMap::<GlyphId16, Vec<Ligature>>::new();
    for (src, dst) in mappings {
        let (first, rest) = src
            .split_first()
            .expect("ligature substitution requires a non-empty source sequence");
        grouped
            .entry(*first)
            .or_default()
            .push(Ligature::new(*dst, rest.to_vec()));
    }

    let coverage = CoverageTable::format_1(grouped.keys().copied().collect());
    let ligature_sets = grouped
        .into_values()
        .map(LigatureSet::new)
        .collect();
    let subtable = LigatureSubstFormat1::new(coverage, ligature_sets);
    SubstitutionLookup::Ligature(Lookup::new(LookupFlag::empty(), vec![subtable]))
}

fn overwrite_lookup(gsub: &mut Gsub, lookup_index: u16, lookup: SubstitutionLookup) {
    gsub.lookup_list.lookups[usize::from(lookup_index)] = lookup.into();
}
