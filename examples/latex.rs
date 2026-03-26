//! Generate recipe for LaTeX.
use morphio::{MorphOptions, OwnedMorphRule, Recipe};
use serde_json::from_reader;
use std::{collections::HashMap, fs::File};

fn main() {
    // Get from https://github.com/ViktorQvarfordt/unicode-latex/blob/master/latex-unicode.json
    let file = File::open("doc/latex-unicode.json").expect("Failed to open file");
    let mappings: HashMap<String, String> = from_reader(file).expect("Failed to parse JSON");

    // Retain mappings that:
    // - Start with a backslash
    // - Consist entirely of letters or digits (excluding the backslash)
    // - The value is a single Unicode character
    // - The value is not an ASCII character
    let filtered_mappings: HashMap<String, String> = mappings
        .into_iter()
        .filter(|(key, value)| {
            key.starts_with('\\')
                && key[1..].chars().all(|c| c.is_alphanumeric())
                && value.chars().count() == 1
                && !value.chars().any(|c| c.is_ascii())
        })
        .collect();

    // Collect the filtered mappings into a vector of OwnedMorphRule
    let rules: Vec<OwnedMorphRule> = filtered_mappings
        .into_iter()
        .map(|(from, to)| OwnedMorphRule { from, to })
        .collect();
    println!("Collected {} rules", rules.len());

    // Create a Recipe with the collected rules
    let options = MorphOptions {
        word_match_start: false, // We start with a backslash, so we don't need to match the start of a word
        word_match_end: true,
        skip_missing_glyphs: false,
    };
    let recipe = Recipe { rules, options };

    // Save the recipe to a file
    let string = recipe
        .to_toml()
        .expect("Failed to serialize recipe to TOML");
    std::fs::write("tests/recipes/latex_unicode.toml", string)
        .expect("Failed to write recipe to file");
}
