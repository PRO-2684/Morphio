use read_fonts::{FontRef, ReadError, TableProvider, tables::layout::LookupFlag, types::NameId};
use std::fs::{read, write};
use text_morph::{
    TextMorph,
    ext::{FontRefExt, GsubExt},
};
use write_fonts::{
    FontBuilder, NullableOffsetMarker, OffsetMarker,
    from_obj::ToOwnedTable,
    tables::{
        gsub::{Gsub, SingleSubst, SubstitutionChainContext, SubstitutionLookup},
        layout::{
            ChainedSequenceContext, ChainedSequenceContextFormat1, ChainedSequenceRule,
            ChainedSequenceRuleSet, CoverageFormat1, Lookup, SequenceLookupRecord,
        },
        name::Name,
    },
};

fn create_substitution_lookup(gsub: &mut Gsub) -> &mut Lookup<SingleSubst> {
    gsub.lookup_list
        .lookups
        .push(OffsetMarker::new(SubstitutionLookup::Single(
            Default::default(),
        )));
    let sub_lookup_table_index = gsub.lookup_list.lookups.len() - 1;
    match &mut *gsub.lookup_list.lookups[sub_lookup_table_index] {
        SubstitutionLookup::Single(table) => table,
        _ => unreachable!(),
    }
}

fn main() {
    println!("Loading font...");
    let file = read("tests/fonts/msyh.ttc").unwrap();
    let font = FontRef::from_index(&file, 0).unwrap();
    let font_name = font
        .friendly_name()
        .unwrap()
        .unwrap_or_else(|| "Unknown".into());
    println!("Font name: {font_name}");

    // Change "abc" to show up as "xyz" in the font.
    // Get glyph IDs for 'a', 'b', and 'c' from the font's cmap.
    let cmap = font.best_cmap().unwrap().unwrap();
    let from_glyphs = ['a', 'b', 'c'].map(|c| cmap.map_codepoint(c).unwrap());
    let to_glyphs = ['x', 'y', 'z'].map(|c| cmap.map_codepoint(c).unwrap());

    let mut gsub: Gsub = font.gsub().unwrap().to_owned_table();

    let coverage = CoverageFormat1 {
        glyph_array: todo!(),
    };
    let mut rules = Vec::with_capacity(3);
    for i in 0..3 {
        rules.push(ChainedSequenceRule {
            backtrack_sequence: vec![], // No backtracking
            input_sequence: vec![todo!()],
            lookahead_sequence: vec![], // No lookahead
            seq_lookup_records: vec![todo!()],
        });
    }

    let ruleset = ChainedSequenceRuleSet::new(rules);
    let ctx = ChainedSequenceContextFormat1 {
        coverage: OffsetMarker::new(coverage.into()),
        chained_seq_rule_sets: vec![NullableOffsetMarker::new(Some(ruleset))],
    };
    let ctx = ChainedSequenceContext::Format1(ctx);
    let lookup =
        SubstitutionLookup::ChainContextual(Lookup::new(LookupFlag::empty(), vec![ctx.into()])); // todo
    gsub.lookup_list.lookups.push(OffsetMarker::new(lookup));

    // Save the modified font.
    println!("Building modified font...");
    let new_font = gsub.to_font_builder(font).unwrap().build();
    write("tests/fonts/msyh_modified.ttf", new_font).unwrap();
}
