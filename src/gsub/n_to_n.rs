//! Helpers for `N -> N` substitutions.

use read_fonts::types::GlyphId16;
use write_fonts::tables::{gsub::Gsub, layout::SequenceLookupRecord};

use super::{MorphError, one_to_one::SingleSubstitutionCache};

/// Builds sequence records for `N -> N` substitutions.
///
/// `start_index` specifies where `from_glyphs` begins inside the surrounding
/// matched sequence.
pub fn build_n_to_n_records(
    gsub: &mut Gsub,
    from_glyphs: &[GlyphId16],
    to_glyphs: &[GlyphId16],
    start_index: usize,
    single_cache: &mut SingleSubstitutionCache,
) -> Result<Vec<SequenceLookupRecord>, MorphError> {
    debug_assert_eq!(from_glyphs.len(), to_glyphs.len());

    let mut sequence_records = Vec::new();
    for (offset, (&src, &dst)) in from_glyphs.iter().zip(to_glyphs.iter()).enumerate() {
        if let Some(record) =
            single_cache.sequence_record(gsub, start_index + offset, src, dst)?
        {
            sequence_records.push(record);
        }
    }

    Ok(sequence_records)
}
