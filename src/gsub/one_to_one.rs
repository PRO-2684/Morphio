//! Primitive helpers for `1 -> 1` substitutions.

use std::collections::BTreeMap;

use read_fonts::types::GlyphId16;
use write_fonts::tables::{
    gsub::Gsub,
    layout::{CoverageTable, Lookup, LookupFlag, SequenceLookupRecord},
};

use super::{MorphError, push_lookup};

#[derive(Debug)]
struct SingleLookupBucket {
    lookup_index: u16,
    mappings: BTreeMap<GlyphId16, GlyphId16>,
}

/// Deduplicates identical single substitutions while building one rule.
#[derive(Debug, Default)]
pub struct SingleSubstitutionCache {
    lookup_indices: BTreeMap<(GlyphId16, GlyphId16), u16>,
    buckets: Vec<SingleLookupBucket>,
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

        let lookup_index = if let Some(index) = self.lookup_indices.get(&(src, dst)) {
            *index
        } else {
            let index = self.insert_mapping(gsub, src, dst)?;
            self.lookup_indices.insert((src, dst), index);
            index
        };
        let sequence_index = u16::try_from(sequence_index)
            .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
        Ok(Some(SequenceLookupRecord::new(
            sequence_index,
            lookup_index,
        )))
    }

    fn insert_mapping(
        &mut self,
        gsub: &mut Gsub,
        src: GlyphId16,
        dst: GlyphId16,
    ) -> Result<u16, MorphError> {
        if let Some(bucket) = self
            .buckets
            .iter_mut()
            .find(|bucket| !bucket.mappings.contains_key(&src))
        {
            bucket.mappings.insert(src, dst);
            overwrite_lookup(gsub, bucket.lookup_index, build_single_lookup(&bucket.mappings));
            return Ok(bucket.lookup_index);
        }

        let mut mappings = BTreeMap::new();
        mappings.insert(src, dst);
        let lookup_index = push_lookup(gsub, build_single_lookup(&mappings))?;
        self.buckets.push(SingleLookupBucket {
            lookup_index,
            mappings,
        });
        Ok(lookup_index)
    }
}

fn build_single_lookup(mappings: &BTreeMap<GlyphId16, GlyphId16>) -> write_fonts::tables::gsub::SubstitutionLookup {
    let coverage = CoverageTable::format_1(mappings.keys().copied().collect());
    let subtable = write_fonts::tables::gsub::SingleSubst::format_2(
        coverage,
        mappings.values().copied().collect(),
    );
    write_fonts::tables::gsub::SubstitutionLookup::Single(Lookup::new(
        LookupFlag::empty(),
        vec![subtable],
    ))
}

fn overwrite_lookup(
    gsub: &mut Gsub,
    lookup_index: u16,
    lookup: write_fonts::tables::gsub::SubstitutionLookup,
) {
    gsub.lookup_list.lookups[usize::from(lookup_index)] = lookup.into();
}
