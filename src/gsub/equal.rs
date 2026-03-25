//! Module for handling words of equal length.

use std::collections::BTreeMap;

use super::{MorphError, create_contextual_lookup, create_single_substitution_lookup, push_lookup};
use read_fonts::types::GlyphId16;
use write_fonts::tables::{
    gsub::Gsub,
    layout::{RangeRecord, SequenceLookupRecord},
};

pub fn append_equal_length_lookups(
    gsub: &mut Gsub,
    from_glyphs: &[GlyphId16],
    to_glyphs: &[GlyphId16],
    word_glyph_ranges: Vec<RangeRecord>,
) -> Result<Vec<u16>, MorphError> {
    let mut pair_lookup_indices = BTreeMap::new();
    let mut sequence_records = Vec::new();

    for (sequence_index, (&src, &dst)) in from_glyphs.iter().zip(to_glyphs).enumerate() {
        if src == dst {
            continue;
        }

        let lookup_index = if let Some(index) = pair_lookup_indices.get(&(src, dst)) {
            *index
        } else {
            let index = push_lookup(gsub, create_single_substitution_lookup(src, dst))?;
            pair_lookup_indices.insert((src, dst), index);
            index
        };
        let sequence_index = u16::try_from(sequence_index)
            .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
        sequence_records.push(SequenceLookupRecord::new(sequence_index, lookup_index));
    }

    let contextual_lookup =
        create_contextual_lookup(from_glyphs, word_glyph_ranges, sequence_records);
    let contextual_lookup_index = push_lookup(gsub, contextual_lookup)?;

    Ok(vec![contextual_lookup_index])
}
