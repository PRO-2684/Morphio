//! Internal helpers for building and wiring GSUB tables.

mod feature;
mod n_to_n;
mod n_to_one;
mod one_to_n;
mod one_to_one;

use super::{
    MorphError, MorphOptions,
    font::{ResolvedMorphRule, word_glyph_ranges},
};
use std::cmp::Reverse;
use feature::{ensure_all_scripts_feature, ensure_feature, ensure_script_feature};
use n_to_n::build_n_to_n_records;
use n_to_one::LigatureSubstitutionCache;
use one_to_n::MultipleSubstitutionCache;
use one_to_one::SingleSubstitutionCache;
use read_fonts::{
    FontRef, TableProvider, TopLevelTable,
    types::{GlyphId16, Tag},
};
use write_fonts::{
    from_obj::ToOwnedTable,
    tables::{
        gsub::{Gsub, SubstitutionChainContext, SubstitutionLookup},
        layout::{
            ChainedSequenceContext, CoverageTable, FeatureList, Lookup, LookupFlag, LookupList,
            RangeRecord, ScriptList, SequenceLookupRecord,
        },
    },
};

const CALT_TAG: Tag = Tag::new(b"calt");
const DFLT_TAG: Tag = Tag::new(b"DFLT");

pub fn patch_gsub(
    font: &FontRef<'_>,
    rules: &[ResolvedMorphRule],
    options: MorphOptions,
) -> Result<Gsub, MorphError> {
    let mut gsub = load_gsub(font)?;
    let lookup_index = append_word_substitution_lookups(
        font,
        &mut gsub,
        rules,
        options.word_match_start,
        options.word_match_end,
    )?;
    let Some(lookup_index) = lookup_index else {
        return Ok(gsub);
    };
    let feature_index = ensure_feature(&mut gsub, CALT_TAG, lookup_index)?;
    if gsub.script_list.script_records.is_empty() {
        ensure_script_feature(&mut gsub, DFLT_TAG, feature_index);
    } else {
        ensure_all_scripts_feature(&mut gsub, feature_index);
    }
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

/// Appends lookups for the given morph rules, and returns the index of the new chained contextual lookup.
fn append_word_substitution_lookups(
    font: &FontRef<'_>,
    gsub: &mut Gsub,
    rules: &[ResolvedMorphRule],
    word_match_start: bool,
    word_match_end: bool,
) -> Result<Option<u16>, MorphError> {
    let word_glyph_ranges = if word_match_start || word_match_end {
        word_glyph_ranges(font)?
    } else {
        Vec::new()
    };

    let mut contextual_subtables = Vec::new();
    let mut single_cache = SingleSubstitutionCache::default();
    let mut multiple_cache = MultipleSubstitutionCache::default();
    let mut ligature_cache = LigatureSubstitutionCache::default();

    let mut ordered_rules = rules.iter().enumerate().collect::<Vec<_>>();
    ordered_rules.sort_by_key(|(index, rule)| (Reverse(rule.from_glyphs.len()), *index));

    for (_, rule) in ordered_rules {
        let mut sequence_records = Vec::new();

        if rule.from_glyphs.len() == rule.to_glyphs.len() {
            sequence_records.extend(build_n_to_n_records(
                gsub,
                &rule.from_glyphs,
                &rule.to_glyphs,
                0,
                &mut single_cache,
            )?);
        } else if rule.from_glyphs.len() == 1 {
            sequence_records.push(multiple_cache.sequence_record(
                gsub,
                0,
                rule.from_glyphs[0],
                &rule.to_glyphs,
            )?);
        } else if rule.to_glyphs.len() == 1 {
            sequence_records.push(ligature_cache.sequence_record(
                gsub,
                0,
                &rule.from_glyphs,
                rule.to_glyphs[0],
            )?);
        } else if rule.from_glyphs.len() < rule.to_glyphs.len() {
            let prefix_len = rule.from_glyphs.len() - 1;
            sequence_records.extend(build_n_to_n_records(
                gsub,
                &rule.from_glyphs[..prefix_len],
                &rule.to_glyphs[..prefix_len],
                0,
                &mut single_cache,
            )?);
            sequence_records.push(multiple_cache.sequence_record(
                gsub,
                prefix_len,
                rule.from_glyphs[prefix_len],
                &rule.to_glyphs[prefix_len..],
            )?);
        } else {
            let prefix_len = rule.to_glyphs.len() - 1;
            sequence_records.extend(build_n_to_n_records(
                gsub,
                &rule.from_glyphs[..prefix_len],
                &rule.to_glyphs[..prefix_len],
                0,
                &mut single_cache,
            )?);
            sequence_records.push(ligature_cache.sequence_record(
                gsub,
                prefix_len,
                &rule.from_glyphs[prefix_len..],
                rule.to_glyphs[prefix_len],
            )?);
        }

        if sequence_records.is_empty() {
            continue;
        }

        contextual_subtables.extend(create_contextual_subtables(
            &rule.from_glyphs,
            word_glyph_ranges.clone(),
            sequence_records,
            word_match_start,
            word_match_end,
        ));
    }

    if contextual_subtables.is_empty() {
        return Ok(None);
    }

    let contextual_lookup =
        SubstitutionLookup::ChainContextual(Lookup::new(LookupFlag::empty(), contextual_subtables));
    Ok(Some(push_lookup(gsub, contextual_lookup)?))
}

fn create_contextual_subtables(
    from_glyphs: &[GlyphId16],
    word_glyph_ranges: Vec<RangeRecord>,
    sequence_records: Vec<SequenceLookupRecord>,
    word_match_start: bool,
    word_match_end: bool,
) -> Vec<SubstitutionChainContext> {
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

    subtables
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
