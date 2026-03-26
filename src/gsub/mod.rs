//! Internal helpers for building and wiring GSUB tables.

mod equal;
mod feature;
mod variable;

use super::{
    MorphError, MorphOptions,
    font::{ResolvedMorphRule, word_glyph_ranges},
};
use equal::append_equal_length_lookups;
use feature::{ensure_feature, ensure_script_feature};
use read_fonts::{
    FontRef, TableProvider, TopLevelTable,
    types::{GlyphId16, Tag},
};
use variable::append_variable_length_lookups;
use write_fonts::{
    from_obj::ToOwnedTable,
    tables::{
        gsub::{Gsub, SingleSubst, SubstitutionChainContext, SubstitutionLookup},
        layout::{
            ChainedSequenceContext, CoverageTable, FeatureList, Lookup, LookupFlag, LookupList,
            RangeRecord, ScriptList, SequenceLookupRecord,
        },
    },
};

const CALT_TAG: Tag = Tag::new(b"calt");
const DFLT_TAG: Tag = Tag::new(b"DFLT");
const LATN_TAG: Tag = Tag::new(b"latn");

pub fn patch_gsub(
    font: &FontRef<'_>,
    rules: &[ResolvedMorphRule],
    options: &MorphOptions,
) -> Result<Gsub, MorphError> {
    let mut gsub = load_gsub(font)?;
    let lookup_indices = append_word_substitution_lookups(
        font,
        &mut gsub,
        rules,
        options.word_match_start,
        options.word_match_end,
    )?;
    let feature_index = ensure_feature(&mut gsub, CALT_TAG, &lookup_indices)?;
    ensure_script_feature(&mut gsub, DFLT_TAG, feature_index);
    ensure_script_feature(&mut gsub, LATN_TAG, feature_index);
    Ok(gsub)
}

fn load_gsub(font: &FontRef<'_>) -> Result<Gsub, MorphError> {
    if font.data_for_tag(Gsub::TAG).is_some() {
        Ok(font.gsub().map(|table| table.to_owned_table())?)
    } else {
        Ok(Gsub::new(
            ScriptList::new(Vec::new()),
            FeatureList::new(Vec::new()),
            LookupList::new(Vec::new()),
        ))
    }
}

fn append_word_substitution_lookups(
    font: &FontRef<'_>,
    gsub: &mut Gsub,
    rules: &[ResolvedMorphRule],
    word_match_start: bool,
    word_match_end: bool,
) -> Result<Vec<u16>, MorphError> {
    let word_glyph_ranges = if word_match_start || word_match_end {
        word_glyph_ranges(font)?
    } else {
        Vec::new()
    };

    let mut lookup_indices = Vec::new();

    for rule in rules {
        if rule.from_glyphs.len() == rule.to_glyphs.len() {
            lookup_indices.extend(append_equal_length_lookups(
                gsub,
                &rule.from_glyphs,
                &rule.to_glyphs,
                word_glyph_ranges.clone(),
                word_match_start,
                word_match_end,
            )?);
        } else {
            lookup_indices.extend(append_variable_length_lookups(
                gsub,
                &rule.from_glyphs,
                &rule.to_glyphs,
                word_glyph_ranges.clone(),
                word_match_start,
                word_match_end,
            )?);
        }
    }

    Ok(lookup_indices)
}

fn create_single_substitution_lookup(src: GlyphId16, dst: GlyphId16) -> SubstitutionLookup {
    let coverage = CoverageTable::format_1(vec![src]);
    let subtable = SingleSubst::format_2(coverage, vec![dst]);
    SubstitutionLookup::Single(Lookup::new(LookupFlag::empty(), vec![subtable]))
}

fn create_contextual_lookup(
    from_glyphs: &[GlyphId16],
    word_glyph_ranges: Vec<RangeRecord>,
    sequence_records: Vec<SequenceLookupRecord>,
    word_match_start: bool,
    word_match_end: bool,
) -> SubstitutionLookup {
    let input_coverages = exact_coverages(from_glyphs);
    let mut subtables: Vec<SubstitutionChainContext> = Vec::new();

    if word_match_start || word_match_end {
        let word_coverage = CoverageTable::format_2(word_glyph_ranges);
        if word_match_start {
            subtables.push(
                ChainedSequenceContext::format_3(
                    vec![word_coverage.clone()],
                    input_coverages.clone(),
                    Vec::new(),
                    Vec::new(),
                )
                .into(),
            );
        }
        if word_match_end {
            subtables.push(
                ChainedSequenceContext::format_3(
                    Vec::new(),
                    input_coverages.clone(),
                    vec![word_coverage],
                    Vec::new(),
                )
                .into(),
            );
        }
    }

    subtables.push(
        ChainedSequenceContext::format_3(Vec::new(), input_coverages, Vec::new(), sequence_records)
            .into(),
    );

    SubstitutionLookup::ChainContextual(Lookup::new(LookupFlag::empty(), subtables))
}

fn exact_coverages(glyphs: &[GlyphId16]) -> Vec<CoverageTable> {
    glyphs
        .iter()
        .copied()
        .map(|glyph| CoverageTable::format_1(vec![glyph]))
        .collect()
}

fn push_lookup(gsub: &mut Gsub, lookup: SubstitutionLookup) -> Result<u16, MorphError> {
    let index = u16::try_from(gsub.lookup_list.lookups.len())
        .map_err(|_| MorphError::malformed("lookup list exceeds u16::MAX"))?;
    gsub.lookup_list.lookups.push(lookup.into());
    Ok(index)
}
