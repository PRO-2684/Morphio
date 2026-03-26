//! Module for handling words of variable length.

use super::{MorphError, SequenceLookupRecord, create_contextual_lookup, push_lookup};
use read_fonts::{tables::layout::LookupFlag, types::GlyphId16};
use write_fonts::tables::{
    gsub::{
        Gsub, Ligature, LigatureSet, LigatureSubstFormat1, MultipleSubstFormat1, Sequence,
        SubstitutionLookup,
    },
    layout::{Lookup, RangeRecord},
    varc::CoverageTable,
};

pub fn append_variable_length_lookups(
    gsub: &mut Gsub,
    from_glyphs: &[GlyphId16],
    to_glyphs: &[GlyphId16],
    word_glyph_ranges: Vec<RangeRecord>,
    word_match_start: bool,
    word_match_end: bool,
) -> Result<Vec<u16>, MorphError> {
    if matches!(from_glyphs, [_, ..]) && matches!(to_glyphs, [_]) {
        let collapse_lookup = create_ligature_substitution_lookup(from_glyphs, to_glyphs[0]);
        let collapse_lookup_index = push_lookup(gsub, collapse_lookup)?;
        let collapse_context_index = push_lookup(
            gsub,
            create_contextual_lookup(
                from_glyphs,
                word_glyph_ranges,
                vec![SequenceLookupRecord::new(0, collapse_lookup_index)],
                word_match_start,
                word_match_end,
            ),
        )?;
        return Ok(vec![collapse_context_index]);
    }

    if matches!(from_glyphs, [_]) {
        let expand_lookup = create_multiple_substitution_lookup(from_glyphs[0], to_glyphs);
        let expand_lookup_index = push_lookup(gsub, expand_lookup)?;
        let expand_context_index = push_lookup(
            gsub,
            create_contextual_lookup(
                from_glyphs,
                word_glyph_ranges,
                vec![SequenceLookupRecord::new(0, expand_lookup_index)],
                word_match_start,
                word_match_end,
            ),
        )?;
        return Ok(vec![expand_context_index]);
    }

    if from_glyphs.len() < to_glyphs.len() {
        let mut lookup_indices = Vec::with_capacity(from_glyphs.len());
        for (sequence_index, (&src, &dst)) in from_glyphs[..from_glyphs.len() - 1]
            .iter()
            .zip(to_glyphs.iter())
            .enumerate()
        {
            let lookup_index =
                push_lookup(gsub, super::create_single_substitution_lookup(src, dst))?;
            let sequence_index = u16::try_from(sequence_index)
                .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
            let context_index = push_lookup(
                gsub,
                create_contextual_lookup(
                    from_glyphs,
                    word_glyph_ranges.clone(),
                    vec![SequenceLookupRecord::new(sequence_index, lookup_index)],
                    word_match_start,
                    word_match_end,
                ),
            )?;
            lookup_indices.push(context_index);
        }

        let tail_index = from_glyphs.len() - 1;
        let tail_lookup =
            create_multiple_substitution_lookup(from_glyphs[tail_index], &to_glyphs[tail_index..]);
        let tail_lookup_index = push_lookup(gsub, tail_lookup)?;
        let tail_sequence_index = u16::try_from(tail_index)
            .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
        let tail_context_index = push_lookup(
            gsub,
            create_contextual_lookup(
                from_glyphs,
                word_glyph_ranges,
                vec![SequenceLookupRecord::new(
                    tail_sequence_index,
                    tail_lookup_index,
                )],
                word_match_start,
                word_match_end,
            ),
        )?;
        lookup_indices.push(tail_context_index);
        return Ok(lookup_indices);
    }

    let mut lookup_indices = Vec::with_capacity(to_glyphs.len());
    for (sequence_index, (&src, &dst)) in from_glyphs
        .iter()
        .zip(&to_glyphs[..to_glyphs.len() - 1])
        .enumerate()
    {
        let lookup_index = push_lookup(gsub, super::create_single_substitution_lookup(src, dst))?;
        let sequence_index = u16::try_from(sequence_index)
            .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
        let context_index = push_lookup(
            gsub,
            create_contextual_lookup(
                from_glyphs,
                word_glyph_ranges.clone(),
                vec![SequenceLookupRecord::new(sequence_index, lookup_index)],
                word_match_start,
                word_match_end,
            ),
        )?;
        lookup_indices.push(context_index);
    }

    let ligature_start = to_glyphs.len() - 1;
    let ligature_lookup = create_ligature_substitution_lookup(
        &from_glyphs[ligature_start..],
        to_glyphs[ligature_start],
    );
    let ligature_lookup_index = push_lookup(gsub, ligature_lookup)?;
    let ligature_sequence_index = u16::try_from(ligature_start)
        .map_err(|_| MorphError::malformed("sequence index exceeds u16::MAX"))?;
    let ligature_context_index = push_lookup(
        gsub,
        create_contextual_lookup(
            from_glyphs,
            word_glyph_ranges,
            vec![SequenceLookupRecord::new(
                ligature_sequence_index,
                ligature_lookup_index,
            )],
            word_match_start,
            word_match_end,
        ),
    )?;
    lookup_indices.push(ligature_context_index);

    Ok(lookup_indices)
}

fn create_multiple_substitution_lookup(src: GlyphId16, dst: &[GlyphId16]) -> SubstitutionLookup {
    let coverage = CoverageTable::format_1(vec![src]);
    let subtable = MultipleSubstFormat1::new(coverage, vec![Sequence::new(dst.to_vec())]);
    SubstitutionLookup::Multiple(Lookup::new(LookupFlag::empty(), vec![subtable]))
}

fn create_ligature_substitution_lookup(src: &[GlyphId16], dst: GlyphId16) -> SubstitutionLookup {
    let (first, rest) = src
        .split_first()
        .expect("ligature substitution requires a non-empty source sequence");
    let coverage = CoverageTable::format_1(vec![*first]);
    let ligature = Ligature::new(dst, rest.to_vec());
    let subtable = LigatureSubstFormat1::new(coverage, vec![LigatureSet::new(vec![ligature])]);
    SubstitutionLookup::Ligature(Lookup::new(LookupFlag::empty(), vec![subtable]))
}
