//! Shared cache infrastructure for conflict-aware primitive GSUB lookups.

use std::collections::BTreeMap;

use write_fonts::tables::{gsub::Gsub, gsub::SubstitutionLookup};

use super::{MorphError, push_lookup};

#[derive(Debug)]
struct LookupBucket<Src, Dst> {
    lookup_index: u16,
    mappings: BTreeMap<Src, Dst>,
}

/// Reuses a primitive lookup until the same source would need a different output.
///
/// The cache keeps one bucket per non-conflicting lookup. When a new mapping
/// conflicts on the source side, a fresh lookup bucket is created.
#[derive(Debug)]
pub struct SharedLookupCache<Src, Dst> {
    lookup_indices: BTreeMap<(Src, Dst), u16>,
    buckets: Vec<LookupBucket<Src, Dst>>,
    build_lookup: fn(&BTreeMap<Src, Dst>) -> SubstitutionLookup,
}

impl<Src, Dst> SharedLookupCache<Src, Dst>
where
    Src: Clone + Ord,
    Dst: Clone + Ord,
{
    pub fn new(build_lookup: fn(&BTreeMap<Src, Dst>) -> SubstitutionLookup) -> Self {
        Self {
            lookup_indices: BTreeMap::new(),
            buckets: Vec::new(),
            build_lookup,
        }
    }

    /// Returns the lookup index for the requested mapping, reusing or creating
    /// a conflict-free bucket as needed.
    pub fn lookup_index(&mut self, gsub: &mut Gsub, src: Src, dst: Dst) -> Result<u16, MorphError> {
        if let Some(index) = self.lookup_indices.get(&(src.clone(), dst.clone())) {
            return Ok(*index);
        }

        let lookup_index = self.insert_mapping(gsub, src.clone(), dst.clone())?;
        self.lookup_indices.insert((src, dst), lookup_index);
        Ok(lookup_index)
    }

    fn insert_mapping(&mut self, gsub: &mut Gsub, src: Src, dst: Dst) -> Result<u16, MorphError> {
        if let Some(bucket) = self
            .buckets
            .iter_mut()
            .find(|bucket| !bucket.mappings.contains_key(&src))
        {
            bucket.mappings.insert(src, dst);
            overwrite_lookup(
                gsub,
                bucket.lookup_index,
                (self.build_lookup)(&bucket.mappings),
            );
            return Ok(bucket.lookup_index);
        }

        let mut mappings = BTreeMap::new();
        mappings.insert(src, dst);
        let lookup_index = push_lookup(gsub, (self.build_lookup)(&mappings))?;
        self.buckets.push(LookupBucket {
            lookup_index,
            mappings,
        });
        Ok(lookup_index)
    }
}

fn overwrite_lookup(gsub: &mut Gsub, lookup_index: u16, lookup: SubstitutionLookup) {
    gsub.lookup_list.lookups[usize::from(lookup_index)] = lookup.into();
}
