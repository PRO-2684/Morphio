use argh::FromArgs;
use morphio::{MorphOptions, MorphRule, Morphio};
use read_fonts::FileRef;
use std::{
    fs::{read, write},
    path::PathBuf,
};

/// Morphio: Morphs the font, so it renders worda as wordb.
#[derive(FromArgs)]
#[argh(help_triggers("-h", "--help"))]
struct Args {
    /// input font file path
    #[argh(option, short = 'i')]
    input: PathBuf,
    /// output font file path
    #[argh(option, short = 'o')]
    output: PathBuf,
    /// disable both start and end word matching
    #[argh(switch, short = 'm')]
    no_word_match: bool,
    /// disable word matching at the start of the source word
    #[argh(switch)]
    no_word_match_start: bool,
    /// disable word matching at the end of the source word
    #[argh(switch)]
    no_word_match_end: bool,
    /// allow overwrite output file if it exists
    #[argh(switch, short = 'y')]
    yes: bool,
    /// pairs of words to morph
    #[argh(positional, greedy)]
    pairs: Vec<String>,
}

impl Args {
    fn to_morph_options(&self) -> MorphOptions {
        MorphOptions {
            word_match_start: !(self.no_word_match || self.no_word_match_start),
            word_match_end: !(self.no_word_match || self.no_word_match_end),
        }
    }
}

fn main() {
    let args: Args = argh::from_env();
    if args.output.exists() && !args.yes {
        eprintln!("Output file already exists. Use -y to overwrite.");
        std::process::exit(1);
    }
    let data = read(&args.input).expect("Failed to read input font file");
    let font = FileRef::new(&data).expect("Failed to parse font file");
    let mut rules = Vec::new();
    for pair in args.pairs.chunks_exact(2) {
        let from = &pair[0];
        let to = &pair[1];
        rules.push(MorphRule { from, to });
    }
    let morphed = font
        .morph_many_with_options(&rules, &args.to_morph_options())
        .expect("Failed to morph font");
    write(&args.output, morphed).expect("Failed to write output font file");
}
