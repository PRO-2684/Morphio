use morphio::{MorphError, MorphOptions, MorphRule, Morphio};
use read_fonts::{FileRef, FontRef, TableProvider, types::Tag};
use std::fs::read;

fn msyh_bytes() -> Vec<u8> {
    read("tests/fonts/msyh.ttc").expect("msyh fixture font should exist")
}

fn impact_bytes() -> Vec<u8> {
    read("tests/fonts/IMPACT.TTF").expect("impact fixture should exist")
}

#[test]
fn rejects_empty_words() {
    let bytes = msyh_bytes();
    let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
    let err = font.morph("", "").expect_err("expected a validation error");
    assert!(matches!(err, MorphError::EmptyWord));
}

#[test]
fn rejects_missing_glyphs() {
    let bytes = msyh_bytes();
    let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
    let err = font
        .morph("abc", "😀bc")
        .expect_err("expected a missing glyph error");
    assert!(matches!(err, MorphError::MissingGlyph('😀')));
}

#[test]
fn builds_a_font_with_calt_feature() {
    let bytes = msyh_bytes();
    let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
    let morphed = font
        .morph("abc", "xyz")
        .expect("font should patch successfully");

    let rebuilt = FontRef::new(&morphed).expect("patched font should parse as sfnt");
    let gsub = rebuilt.gsub().expect("patched font should contain GSUB");
    let feature_list = gsub
        .feature_list()
        .expect("patched GSUB should contain a feature list");
    let has_calt = feature_list
        .feature_records()
        .iter()
        .any(|record| record.feature_tag() == read_fonts::types::Tag::new(b"calt"));

    assert!(has_calt, "patched font should expose a calt feature");
}

#[test]
fn supports_many_to_one_morphs() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let morphed = font
        .morph("banana", "x")
        .expect("many-to-one morph should succeed");

    assert!(
        FontRef::new(&morphed).is_ok(),
        "morphed font should remain parseable"
    );
}

#[test]
fn supports_one_to_many_morphs() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let morphed = font
        .morph("x", "orange")
        .expect("one-to-many morph should succeed");

    assert!(
        FontRef::new(&morphed).is_ok(),
        "morphed font should remain parseable"
    );
}

#[test]
fn file_ref_delegates_to_single_font() {
    let bytes = msyh_bytes();
    let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
    let file = FileRef::Font(font);
    let morphed = file
        .morph("abc", "xyz")
        .expect("single-font FileRef should patch successfully");

    assert!(
        FontRef::new(&morphed).is_ok(),
        "patched bytes should be a standalone sfnt"
    );
}

#[test]
fn file_ref_morphs_all_fonts_in_collection() {
    let bytes = msyh_bytes();
    let file = FileRef::new(&bytes).expect("fixture should parse");
    let morphed = file
        .morph("abc", "xyz")
        .expect("collection morph should patch every font");
    let rebuilt = FileRef::new(&morphed).expect("patched bytes should still be a valid file");

    let FileRef::Collection(collection) = rebuilt else {
        panic!("patched fixture should remain a collection");
    };
    assert_eq!(collection.len(), 2);

    for font in collection.iter() {
        let font = font.expect("collection member should parse");
        let gsub = font.gsub().expect("patched font should contain GSUB");
        let feature_list = gsub
            .feature_list()
            .expect("patched GSUB should contain a feature list");
        let has_calt = feature_list
            .feature_records()
            .iter()
            .any(|record| record.feature_tag() == Tag::new(b"calt"));
        assert!(
            has_calt,
            "every collection font should expose a calt feature"
        );
    }
}

#[test]
fn impact_adds_calt_to_all_latin_langsys_records() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let morphed = font
        .morph("banana", "orange")
        .expect("impact font should patch successfully");
    let rebuilt = FontRef::new(&morphed).expect("patched impact should parse");
    let gsub = rebuilt.gsub().expect("patched impact should contain GSUB");
    let feature_list = gsub
        .feature_list()
        .expect("patched GSUB should contain a feature list");
    let calt_index = feature_list
        .feature_records()
        .iter()
        .position(|record| record.feature_tag() == Tag::new(b"calt"))
        .expect("patched impact should expose calt");
    let calt_index = u16::try_from(calt_index).expect("feature index should fit into u16 for test");

    let script_list = gsub
        .script_list()
        .expect("patched GSUB should contain scripts");
    let latn = script_list
        .script_records()
        .iter()
        .find(|record| record.script_tag() == Tag::new(b"latn"))
        .expect("patched impact should keep latn script");
    let script = latn
        .script(script_list.offset_data())
        .expect("latn script should resolve");

    let default_lang_sys = script
        .default_lang_sys()
        .expect("latn script should have a default langsys")
        .expect("latn default langsys should resolve");
    assert!(
        default_lang_sys
            .feature_indices()
            .iter()
            .any(|index| index.get() == calt_index),
        "latn default langsys should include calt",
    );

    for record in script.lang_sys_records() {
        let lang_sys = record
            .lang_sys(script.offset_data())
            .expect("langsys should resolve");
        assert!(
            lang_sys
                .feature_indices()
                .iter()
                .any(|index| index.get() == calt_index),
            "every latn langsys should include calt",
        );
    }
}

#[test]
fn supports_multiple_rules_in_one_pass() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let rules = [
        MorphRule::new("banana", "orange"),
        MorphRule::new("x", "yz"),
    ];
    let morphed = font
        .morph_many(&rules)
        .expect("multi-rule morph should succeed");

    let rebuilt = FontRef::new(&morphed).expect("morphed font should parse");
    let gsub = rebuilt.gsub().expect("patched font should contain GSUB");
    let feature_list = gsub
        .feature_list()
        .expect("patched GSUB should contain a feature list");
    let has_calt = feature_list
        .feature_records()
        .iter()
        .any(|record| record.feature_tag() == Tag::new(b"calt"));

    assert!(has_calt, "patched font should expose a calt feature");
}

#[test]
fn supports_multiple_rules_in_collection_with_placeholder() {
    let bytes = msyh_bytes();
    let file = FileRef::new(&bytes).expect("fixture should parse");
    let rules = [
        MorphRule::new("banana", "orange"),
        MorphRule::new("from", "to"),
    ];
    let morphed = file
        .morph_many(&rules)
        .expect("multi-rule collection morph should succeed");

    let rebuilt = FileRef::new(&morphed).expect("morphed collection should parse");
    let FileRef::Collection(collection) = rebuilt else {
        panic!("patched fixture should remain a collection");
    };
    assert_eq!(collection.len(), 2);

    for font in collection.iter() {
        let font = font.expect("collection member should parse");
        let maxp = font.maxp().expect("patched font should contain maxp");
        assert!(
            font.table_data(Tag::new(b"LTSH")).is_none(),
            "placeholder rebuild should drop stale LTSH",
        );
        assert!(
            font.table_data(Tag::new(b"hdmx")).is_none(),
            "placeholder rebuild should drop stale hdmx",
        );

        let vhea = font.vhea().expect("patched font should contain vhea");
        let vmtx = font.vmtx().expect("patched font should contain vmtx");
        let metric_count = vmtx.v_metrics().len() + vmtx.top_side_bearings().len();
        assert_eq!(
            usize::from(vhea.number_of_long_ver_metrics()),
            vmtx.v_metrics().len(),
            "vhea and vmtx should agree on long metric count",
        );
        assert_eq!(
            usize::from(maxp.num_glyphs()),
            metric_count,
            "vmtx should cover every glyph after placeholder insertion",
        );
    }
}

#[test]
fn supports_disabling_start_word_match_only() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let morphed = font
        .morph_with_options("banana", "orange", &MorphOptions::new(false, true))
        .expect("start-only relaxed morph should succeed");

    assert!(
        FontRef::new(&morphed).is_ok(),
        "morphed font should remain parseable"
    );
}

#[test]
fn supports_disabling_end_word_match_only() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let morphed = font
        .morph_with_options("banana", "orange", &MorphOptions::new(true, false))
        .expect("end-only relaxed morph should succeed");

    assert!(
        FontRef::new(&morphed).is_ok(),
        "morphed font should remain parseable"
    );
}
