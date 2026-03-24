//! Internal helpers for building and wiring GSUB tables.

use std::collections::BTreeMap;

use read_fonts::{
    FontRef, TableProvider, TopLevelTable,
    types::{GlyphId16, Tag},
};
use write_fonts::{
    from_obj::ToOwnedTable,
    tables::{
        gsub::{Gsub, SingleSubst, SubstitutionChainContext, SubstitutionLookup},
        layout::{
            ChainedSequenceContext, CoverageTable, Feature, FeatureList, FeatureRecord, LangSys,
            Lookup, LookupFlag, LookupList, RangeRecord, Script, ScriptList, ScriptRecord,
            SequenceLookupRecord,
        },
    },
};

use super::{MorphError, font::word_glyph_ranges};

const CALT_TAG: Tag = Tag::new(b"calt");
const DFLT_TAG: Tag = Tag::new(b"DFLT");
const LATN_TAG: Tag = Tag::new(b"latn");

pub fn patch_gsub(
    font: &FontRef<'_>,
    from_glyphs: &[GlyphId16],
    to_glyphs: &[GlyphId16],
) -> Result<Gsub, MorphError> {
    let mut gsub = load_gsub(font)?;
    let lookup_indices = append_word_substitution_lookups(font, &mut gsub, from_glyphs, to_glyphs)?;
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
    from_glyphs: &[GlyphId16],
    to_glyphs: &[GlyphId16],
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

    let word_glyph_ranges = word_glyph_ranges(font)?;
    let contextual_lookup =
        create_contextual_lookup(from_glyphs, word_glyph_ranges, sequence_records);
    let contextual_lookup_index = push_lookup(gsub, contextual_lookup)?;

    Ok(vec![contextual_lookup_index])
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
) -> SubstitutionLookup {
    let input_coverages = exact_coverages(from_glyphs);
    let mut subtables: Vec<SubstitutionChainContext> = Vec::new();

    if !word_glyph_ranges.is_empty() {
        let word_coverage = CoverageTable::format_2(word_glyph_ranges);
        subtables.push(
            ChainedSequenceContext::format_3(
                vec![word_coverage.clone()],
                input_coverages.clone(),
                Vec::new(),
                Vec::new(),
            )
            .into(),
        );
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

fn ensure_feature(
    gsub: &mut Gsub,
    feature_tag: Tag,
    lookup_indices: &[u16],
) -> Result<u16, MorphError> {
    if let Some((index, record)) = gsub
        .feature_list
        .feature_records
        .iter_mut()
        .enumerate()
        .find(|(_, record)| record.feature_tag == feature_tag)
    {
        let feature = record.feature.as_mut();
        for lookup_index in lookup_indices {
            if !feature.lookup_list_indices.contains(lookup_index) {
                feature.lookup_list_indices.push(*lookup_index);
            }
        }
        return u16::try_from(index)
            .map_err(|_| MorphError::malformed("feature index exceeds u16::MAX"));
    }

    let index = u16::try_from(gsub.feature_list.feature_records.len())
        .map_err(|_| MorphError::malformed("feature list exceeds u16::MAX"))?;
    gsub.feature_list.feature_records.push(FeatureRecord::new(
        feature_tag,
        Feature::new(None, lookup_indices.to_vec()),
    ));
    Ok(index)
}

fn ensure_script_feature(gsub: &mut Gsub, script_tag: Tag, feature_index: u16) {
    if let Some(record) = gsub
        .script_list
        .script_records
        .iter_mut()
        .find(|record| record.script_tag == script_tag)
    {
        ensure_langsys_features(record.script.as_mut(), feature_index);
        return;
    }

    gsub.script_list.script_records.push(ScriptRecord::new(
        script_tag,
        Script::new(Some(LangSys::new(vec![feature_index])), Vec::new()),
    ));
}

fn ensure_langsys_features(script: &mut Script, feature_index: u16) {
    let lang_sys = if let Some(lang_sys) = script.default_lang_sys.as_mut() {
        lang_sys
    } else {
        script.default_lang_sys.set(LangSys::new(Vec::new()));
        script
            .default_lang_sys
            .as_mut()
            .expect("default LangSys was just inserted")
    };

    if !lang_sys.feature_indices.contains(&feature_index) {
        lang_sys.feature_indices.push(feature_index);
    }

    for record in &mut script.lang_sys_records {
        let lang_sys = record.lang_sys.as_mut();
        if !lang_sys.feature_indices.contains(&feature_index) {
            lang_sys.feature_indices.push(feature_index);
        }
    }
}
