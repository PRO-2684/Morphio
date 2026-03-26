//! Primitive helpers for `1 -> 1` substitutions.

use std::collections::BTreeMap;

use read_fonts::types::GlyphId16;
use write_fonts::tables::layout::SequenceLookupRecord;

use super::{MorphError, create_single_substitution_lookup, push_lookup};

/// Deduplicates identical single substitutions while building one rule.
#[derive(Debug, Default)]
pub struct SingleSubstitutionCache {
    lookup_indices: BTreeMap<(GlyphId16, GlyphId16), u16>,
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
            let index = push_lookup(gsub, create_single_substitution_lookup(src, dst))?;
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
}
