//! Module for handling GSUB features.

use read_fonts::types::Tag;
use write_fonts::tables::{
    gsub::Gsub,
    layout::{Feature, FeatureRecord, LangSys, Script, ScriptRecord},
};

use super::MorphError;

pub fn ensure_feature(
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

pub fn ensure_script_feature(gsub: &mut Gsub, script_tag: Tag, feature_index: u16) {
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

pub fn ensure_all_scripts_feature(gsub: &mut Gsub, feature_index: u16) {
    for record in &mut gsub.script_list.script_records {
        ensure_langsys_features(record.script.as_mut(), feature_index);
    }
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
