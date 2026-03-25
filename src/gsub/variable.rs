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
    let mut lookup_indices = Vec::new();

    let expand_input = if matches!(from_glyphs, [_]) {
        from_glyphs
    } else {
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
        lookup_indices.push(collapse_context_index);
        std::slice::from_ref(&placeholder)
    };

    if !matches!(to_glyphs, [_]) {
        let expand_lookup = create_multiple_substitution_lookup(expand_input[0], to_glyphs);
        let expand_lookup_index = push_lookup(gsub, expand_lookup)?;
        let expand_context_index = push_lookup(
            gsub,
            create_contextual_lookup(
                expand_input,
                word_glyph_ranges,
                vec![SequenceLookupRecord::new(0, expand_lookup_index)],
            ),
        )?;
        lookup_indices.push(expand_context_index);
    }

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
