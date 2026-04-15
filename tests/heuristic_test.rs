use morphio::{MorphError, MorphOptions, MorphRule, Morphio, Recipe};
use read_fonts::{
    FileRef, FontRef, TableProvider, tables::gsub::SubstitutionSubtables, types::Tag,
};
use std::fs::read;

fn msyh_bytes() -> Vec<u8> {
    read("tests/fonts/msyh.ttc").expect("msyh fixture font should exist")
}

fn impact_bytes() -> Vec<u8> {
    read("tests/fonts/IMPACT.TTF").expect("impact fixture should exist")
}

fn recipe(path: &str) -> Recipe {
    let contents = read(path).expect("recipe fixture should exist");
    let contents = String::from_utf8(contents).expect("recipe fixture should be utf-8");
    Recipe::from_toml(&contents).expect("recipe fixture should parse")
}

#[derive(Debug, Default, PartialEq, Eq)]
struct LookupKindCounts {
    single: usize,
    multiple: usize,
    ligature: usize,
    chain_contextual: usize,
}

fn lookup_kind_counts(font: &FontRef<'_>) -> LookupKindCounts {
    let Ok(gsub) = font.gsub() else {
        return LookupKindCounts::default();
    };
    let lookup_list = gsub
        .lookup_list()
        .expect("GSUB lookup list should decode when GSUB exists");

    let mut counts = LookupKindCounts::default();
    for lookup in lookup_list.lookups().iter() {
        let lookup = lookup.expect("lookup should resolve");
        let subtables = lookup.subtables().expect("lookup subtables should resolve");
        match subtables {
            SubstitutionSubtables::Single(_) => counts.single += 1,
            SubstitutionSubtables::Multiple(_) => counts.multiple += 1,
            SubstitutionSubtables::Ligature(_) => counts.ligature += 1,
            SubstitutionSubtables::ChainContextual(_) => {
                counts.chain_contextual += 1;
            }
            _ => {}
        }
    }

    counts
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
fn can_skip_rules_with_missing_glyphs() {
    let bytes = msyh_bytes();
    let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
    let rules = [MorphRule::new("abc", "xyz"), MorphRule::new("abc", "😀bc")];
    let morphed = font
        .morph_many_with_options(&rules, MorphOptions::new(true, true, true))
        .expect("missing-glyph rules should be skipped");

    let rebuilt = FontRef::new(&morphed).expect("patched font should parse as sfnt");
    let gsub = rebuilt.gsub().expect("patched font should contain GSUB");
    let feature_list = gsub
        .feature_list()
        .expect("patched GSUB should contain a feature list");
    let has_calt = feature_list
        .feature_records()
        .iter()
        .any(|record| record.feature_tag() == Tag::new(b"calt"));
    assert!(has_calt, "valid rules should still produce a calt feature");
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
fn adds_calt_to_all_existing_scripts() {
    let bytes = msyh_bytes();
    let font = FontRef::from_index(&bytes, 0).expect("fixture should parse");
    let original_gsub = font.gsub().expect("fixture should contain GSUB");
    let original_scripts = original_gsub
        .script_list()
        .expect("fixture should contain scripts");
    let original_script_tags = original_scripts
        .script_records()
        .iter()
        .map(|record| record.script_tag())
        .collect::<Vec<_>>();
    assert!(
        !original_script_tags.is_empty(),
        "fixture should have at least one existing script"
    );

    let morphed = font
        .morph("abc", "xyz")
        .expect("font should patch successfully");
    let rebuilt = FontRef::new(&morphed).expect("patched font should parse");
    let gsub = rebuilt.gsub().expect("patched font should contain GSUB");
    let feature_list = gsub
        .feature_list()
        .expect("patched GSUB should contain a feature list");
    let calt_index = feature_list
        .feature_records()
        .iter()
        .position(|record| record.feature_tag() == Tag::new(b"calt"))
        .expect("patched font should expose calt");
    let calt_index = u16::try_from(calt_index).expect("feature index should fit into u16");

    let script_list = gsub
        .script_list()
        .expect("patched GSUB should contain scripts");
    for script_tag in original_script_tags {
        let record = script_list
            .script_records()
            .iter()
            .find(|record| record.script_tag() == script_tag)
            .expect("original script should still exist after patching");
        let script = record
            .script(script_list.offset_data())
            .expect("script should resolve");

        let default_lang_sys = script
            .default_lang_sys()
            .expect("script default langsys should decode");
        if let Ok(default_lang_sys) = default_lang_sys {
            assert!(
                default_lang_sys
                    .feature_indices()
                    .iter()
                    .any(|index| index.get() == calt_index),
                "default langsys for every existing script should include calt",
            );
        }

        for lang_record in script.lang_sys_records() {
            let lang_sys = lang_record
                .lang_sys(script.offset_data())
                .expect("langsys should resolve");
            assert!(
                lang_sys
                    .feature_indices()
                    .iter()
                    .any(|index| index.get() == calt_index),
                "every langsys for every existing script should include calt",
            );
        }
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
fn shares_primitive_lookups_until_sources_conflict() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let before = lookup_kind_counts(&font);
    let rules = [
        MorphRule::new("ab", "cd"),
        MorphRule::new("ef", "gh"),
        MorphRule::new("ax", "yz"),
        MorphRule::new("lm", "nox"),
        MorphRule::new("pq", "rsi"),
        MorphRule::new("tm", "uzz"),
        MorphRule::new("uvw", "xy"),
        MorphRule::new("rst", "pq"),
        MorphRule::new("mst", "nz"),
    ];
    let morphed = font
        .morph_many_with_options(&rules, MorphOptions::new(false, false, false))
        .expect("multi-rule morph should succeed");

    let rebuilt = FontRef::new(&morphed).expect("morphed font should parse");
    let after = lookup_kind_counts(&rebuilt);

    assert_eq!(
        after.single - before.single,
        2,
        "single substitutions should share one lookup until a source glyph conflicts",
    );
    assert_eq!(
        after.multiple - before.multiple,
        2,
        "multiple substitutions should share one lookup until a source glyph conflicts",
    );
    assert_eq!(
        after.ligature - before.ligature,
        2,
        "ligature substitutions should share one lookup until a source sequence conflicts",
    );
    assert_eq!(
        after.chain_contextual - before.chain_contextual,
        1,
        "all rules should share one chained contextual lookup",
    );
}

#[test]
fn orders_shared_ligatures_longest_first_for_same_prefix() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let rules = [
        MorphRule::new("banana", "a"),
        MorphRule::new("bananas", "b"),
    ];
    let morphed = font
        .morph_many_with_options(&rules, MorphOptions::new(false, false, false))
        .expect("multi-rule morph should succeed");

    let rebuilt = FontRef::new(&morphed).expect("morphed font should parse");
    let gsub = rebuilt.gsub().expect("patched font should contain GSUB");
    let lookup_list = gsub
        .lookup_list()
        .expect("patched GSUB should contain a lookup list");
    let lookup_count = lookup_list.lookups().len();
    let ligature_lookup = lookup_list
        .lookups()
        .iter()
        .nth(lookup_count - 2)
        .expect("ligature lookup should exist")
        .expect("ligature lookup should resolve");
    let read_fonts::tables::gsub::SubstitutionSubtables::Ligature(ligature_lookup) =
        ligature_lookup
            .subtables()
            .expect("ligature lookup subtables should resolve")
    else {
        panic!("expected shared ligature lookup before the contextual lookup");
    };
    let ligature_subtable = ligature_lookup
        .iter()
        .next()
        .expect("ligature lookup should contain a subtable")
        .expect("ligature subtable should resolve");
    let ligature_set = ligature_subtable
        .ligature_sets()
        .iter()
        .next()
        .expect("ligature subtable should contain a ligature set")
        .expect("ligature set should resolve");
    let component_lengths = ligature_set
        .ligatures()
        .iter()
        .map(|ligature| {
            ligature
                .expect("ligature should resolve")
                .component_glyph_ids()
                .len()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        component_lengths,
        vec![6, 5],
        "shared ligatures should prefer the longer source before its prefix",
    );
}

#[test]
fn orders_prefix_overlaps_longest_first_in_shared_contextual_lookup() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let rules = [MorphRule::new("abc", "x"), MorphRule::new("abcd", "y")];
    let morphed = font
        .morph_many_with_options(&rules, MorphOptions::new(false, false, false))
        .expect("multi-rule morph should succeed");

    let rebuilt = FontRef::new(&morphed).expect("morphed font should parse");
    let gsub = rebuilt.gsub().expect("patched font should contain GSUB");
    let feature_list = gsub
        .feature_list()
        .expect("patched GSUB should contain a feature list");
    let calt_record = feature_list
        .feature_records()
        .iter()
        .find(|record| record.feature_tag() == Tag::new(b"calt"))
        .expect("patched font should expose calt");
    let calt_feature = calt_record
        .feature(feature_list.offset_data())
        .expect("calt feature should resolve");
    let contextual_lookup_index = calt_feature
        .lookup_list_indices()
        .last()
        .expect("calt should reference at least one lookup")
        .get();
    let lookup_list = gsub
        .lookup_list()
        .expect("patched GSUB should contain a lookup list");
    let chain_lookup = lookup_list
        .lookups()
        .iter()
        .nth(usize::from(contextual_lookup_index))
        .expect("calt lookup index should exist")
        .expect("lookup should resolve")
        .subtables()
        .expect("lookup subtables should resolve");
    let SubstitutionSubtables::ChainContextual(chain_lookup) = chain_lookup else {
        panic!("calt lookup should be chained contextual");
    };
    let subtable_lengths = chain_lookup
        .iter()
        .map(|subtable| {
            let subtable = subtable.expect("subtable should resolve");
            let read_fonts::tables::layout::ChainedSequenceContext::Format3(subtable) = subtable
            else {
                panic!("expected format 3 chained contextual subtables");
            };
            subtable.input_coverages().len()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        subtable_lengths,
        vec![4, 3],
        "longer prefix rule should be ordered before the shorter one",
    );
}

#[test]
fn supports_multiple_rules_in_collection_with_unequal_lengths() {
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
fn supports_disabling_start_word_match_only() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let morphed = font
        .morph_with_options("banana", "orange", MorphOptions::new(false, true, false))
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
        .morph_with_options("banana", "orange", MorphOptions::new(true, false, false))
        .expect("end-only relaxed morph should succeed");

    assert!(
        FontRef::new(&morphed).is_ok(),
        "morphed font should remain parseable"
    );
}

#[test]
fn parses_simple_recipe_fixture() {
    let recipe = recipe("tests/recipes/simple.toml");

    assert_eq!(recipe.options, MorphOptions::new(true, true, false));
    assert_eq!(recipe.rules.len(), 1);
    assert_eq!(recipe.rules[0].from, "Microsoft");
    assert_eq!(recipe.rules[0].to, "Microslop");
}

#[test]
fn recipe_fixture_morphs_font() {
    let bytes = impact_bytes();
    let font = FontRef::new(&bytes).expect("impact fixture should parse");
    let recipe = recipe("tests/recipes/coverage.toml");

    let morphed = font
        .morph_with_recipe(&recipe)
        .expect("recipe-based morph should succeed");

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
