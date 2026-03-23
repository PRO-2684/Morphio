use argh::FromArgs;
use morphio::TextMorph;
use read_fonts::FileRef;
use std::{
    fs::{read, write},
    path::PathBuf,
};

/// Morphio: Morphs the font, so it shows worda as wordb.
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
    /// allow overwrite output file if it exists
    #[argh(switch, short = 'y')]
    yes: bool,
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
        .morph(&args.from, &args.to)
        .expect("Failed to morph font");
    write(&args.output, morphed).expect("Failed to write output font file");
}
