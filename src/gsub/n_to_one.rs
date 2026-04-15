//! Helpers for `N -> 1` substitutions.

use read_fonts::{tables::layout::LookupFlag, types::GlyphId16};
use write_fonts::tables::{
    gsub::{Gsub, Ligature, LigatureSet, LigatureSubstFormat1, SubstitutionLookup},
    layout::{Lookup, SequenceLookupRecord},
    varc::CoverageTable,
};

use std::collections::BTreeMap;

use super::{MorphError, shared::SharedLookupCache};

/// Reuses non-conflicting `N -> 1` lookups across rules.
#[derive(Debug)]
pub struct LigatureSubstitutionCache {
    cache: SharedLookupCache<Vec<GlyphId16>, GlyphId16>,
}

impl Default for LigatureSubstitutionCache {
    fn default() -> Self {
        Self {
            cache: SharedLookupCache::new(build_ligature_lookup),
        }
    }
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
        let lookup_index = self.cache.lookup_index(gsub, src.to_vec(), dst)?;
        let sequence_index = u16::try_from(sequence_index)
            .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
        Ok(SequenceLookupRecord::new(sequence_index, lookup_index))
    }
}

fn build_ligature_lookup(mappings: &BTreeMap<Vec<GlyphId16>, GlyphId16>) -> SubstitutionLookup {
    let mut grouped = BTreeMap::<GlyphId16, Vec<(Vec<GlyphId16>, Ligature)>>::new();
    for (src, dst) in mappings {
        let (first, rest) = src
            .split_first()
            .expect("ligature substitution requires a non-empty source sequence");
        grouped
            .entry(*first)
            .or_default()
            .push((src.clone(), Ligature::new(*dst, rest.to_vec())));
    }

    let coverage = CoverageTable::format_1(grouped.keys().copied().collect());
    let ligature_sets = grouped
        .into_values()
        .map(|mut ligatures| {
            ligatures.sort_by(|(lhs_src, _), (rhs_src, _)| {
                rhs_src
                    .len()
                    .cmp(&lhs_src.len())
                    .then_with(|| lhs_src.cmp(rhs_src))
            });
            LigatureSet::new(
                ligatures
                    .into_iter()
                    .map(|(_, ligature)| ligature)
                    .collect(),
            )
        })
        .collect();
    let subtable = LigatureSubstFormat1::new(coverage, ligature_sets);
    SubstitutionLookup::Ligature(Lookup::new(LookupFlag::empty(), vec![subtable]))
}
