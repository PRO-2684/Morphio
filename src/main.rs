use argh::FromArgs;
use morphio::{MorphOptions, Morphio, OwnedMorphRule, Recipe};
use read_fonts::FileRef;
use std::{
    fs::{File, read, write},
    io::read_to_string,
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
    /// load morph rules and word matching options from a TOML recipe file, ignoring command-line options
    #[argh(option, short = 'r')]
    recipe: Option<PathBuf>,
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
    fn get_morph_options(&self) -> MorphOptions {
        MorphOptions {
            word_match_start: !(self.no_word_match || self.no_word_match_start),
            word_match_end: !(self.no_word_match || self.no_word_match_end),
        }
    }

    fn get_morph_rules(&self) -> Result<Vec<OwnedMorphRule>, String> {
        if !self.pairs.chunks_exact(2).remainder().is_empty() {
            return Err(
                "Expected an even number of positional words: FROM TO [FROM TO ...]".into(),
            );
        }

        Ok(self
            .pairs
            .chunks_exact(2)
            .map(|pair| OwnedMorphRule::new(&pair[0], &pair[1]))
            .collect())
    }

    fn to_recipe(&self) -> Result<Recipe, String> {
        if let Some(path) = &self.recipe {
            let file =
                File::open(path).map_err(|err| format!("Failed to open recipe file: {err}"))?;
            let data =
                read_to_string(file).map_err(|err| format!("Failed to read recipe file: {err}"))?;
            Recipe::from_toml(&data).map_err(|err| format!("Failed to parse recipe file: {err}"))
        } else {
            Ok(Recipe::new(
                self.get_morph_options(),
                self.get_morph_rules()?,
            ))
        }
    }
}

fn main() {
    let args: Args = argh::from_env();
    if args.output.exists() && !args.yes {
        eprintln!("Output file already exists. Use -y to overwrite.");
        std::process::exit(1);
    }
    let recipe = match args.to_recipe() {
        Ok(recipe) => recipe,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let data = read(&args.input).expect("Failed to read input font file");
    let font = FileRef::new(&data).expect("Failed to parse font file");
    let morphed = font
        .morph_with_recipe(&recipe)
        .expect("Failed to morph font");
    write(&args.output, morphed).expect("Failed to write output font file");
}
