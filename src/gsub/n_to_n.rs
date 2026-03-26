//! Helpers for `N -> N` substitutions.

use read_fonts::types::GlyphId16;
use write_fonts::tables::{
    gsub::Gsub,
    layout::RangeRecord,
};

use super::{MorphError, create_contextual_lookup, one_to_one::SingleSubstitutionCache, push_lookup};

/// Appends contextual `N -> N` substitutions over `matched_glyphs`.
///
/// `start_index` specifies where `from_glyphs` begins inside `matched_glyphs`.
pub fn append_n_to_n_lookups(
    gsub: &mut Gsub,
    matched_glyphs: &[GlyphId16],
    from_glyphs: &[GlyphId16],
    to_glyphs: &[GlyphId16],
    start_index: usize,
    single_cache: &mut SingleSubstitutionCache,
    word_glyph_ranges: Vec<RangeRecord>,
    word_match_start: bool,
    word_match_end: bool,
) -> Result<Vec<u16>, MorphError> {
    debug_assert_eq!(from_glyphs.len(), to_glyphs.len());

    let mut sequence_records = Vec::new();
    for (offset, (&src, &dst)) in from_glyphs.iter().zip(to_glyphs.iter()).enumerate() {
        if let Some(record) =
            single_cache.sequence_record(gsub, start_index + offset, src, dst)?
        {
            sequence_records.push(record);
        }
    }

    if sequence_records.is_empty() {
        return Ok(Vec::new());
    }

    let contextual_lookup = create_contextual_lookup(
        matched_glyphs,
        word_glyph_ranges,
        sequence_records,
        word_match_start,
        word_match_end,
    );
    let contextual_lookup_index = push_lookup(gsub, contextual_lookup)?;

    Ok(vec![contextual_lookup_index])
}
