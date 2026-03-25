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
    placeholder: GlyphId16,
    word_glyph_ranges: Vec<RangeRecord>,
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
            ),
        )?;
        return Ok(vec![expand_context_index]);
    }

    let collapse_lookup = create_ligature_substitution_lookup(from_glyphs, placeholder);
    let collapse_lookup_index = push_lookup(gsub, collapse_lookup)?;
    let collapse_context_index = push_lookup(
        gsub,
        create_contextual_lookup(
            from_glyphs,
            word_glyph_ranges.clone(),
            vec![SequenceLookupRecord::new(0, collapse_lookup_index)],
        ),
    )?;

    let expand_input = std::slice::from_ref(&placeholder);
    let expand_lookup = create_multiple_substitution_lookup(placeholder, to_glyphs);
    let expand_lookup_index = push_lookup(gsub, expand_lookup)?;
    let expand_context_index = push_lookup(
        gsub,
        create_contextual_lookup(
            expand_input,
            word_glyph_ranges,
            vec![SequenceLookupRecord::new(0, expand_lookup_index)],
        ),
    )?;

    Ok(vec![collapse_context_index, expand_context_index])
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
