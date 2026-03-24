use argh::FromArgs;
use morphio::{MorphOptions, Morphio};
use read_fonts::FileRef;
use std::{
    fs::{read, write},
    path::PathBuf,
};

/// Morphio: Morphs the font, so it renders worda as wordb.
#[derive(FromArgs)]
#[argh(help_triggers("-h", "--help"))]
struct Args {
    /// word to morph from
    #[argh(positional)]
    from: String,
    /// word to morph to
    #[argh(positional)]
    to: String,
    /// input font file path
    #[argh(option, short = 'i')]
    input: PathBuf,
    /// output font file path
    #[argh(option, short = 'o')]
    output: PathBuf,
    /// disable word match
    #[argh(switch, short = 'm')]
    no_word_match: bool,
    /// allow overwrite output file if it exists
    #[argh(switch, short = 'y')]
    yes: bool,
}

impl Args {
    fn to_morph_options(&self) -> MorphOptions {
        MorphOptions {
            word_match: !self.no_word_match,
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
    let morphed = font
        .morph_with_options(&args.from, &args.to, &args.to_morph_options())
        .expect("Failed to morph font");
    write(&args.output, morphed).expect("Failed to write output font file");
}
