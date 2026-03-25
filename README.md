# Morphio

[![GitHub License](https://img.shields.io/github/license/PRO-2684/Morphio?logo=opensourceinitiative)](https://github.com/PRO-2684/Morphio/blob/main/LICENSE)
![GitHub Repo stars](https://img.shields.io/github/stars/PRO-2684/Morphio?style=flat)
![GitHub Issues or Pull Requests](https://img.shields.io/github/issues/PRO-2684/Morphio)
![GitHub Issues or Pull Requests](https://img.shields.io/github/issues-pr/PRO-2684/Morphio)
[![Crates.io Version](https://img.shields.io/crates/v/morphio?logo=rust)](https://crates.io/crates/morphio)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/morphio?logo=rust)](https://crates.io/crates/morphio)
[![docs.rs](https://img.shields.io/docsrs/morphio?logo=rust)](https://docs.rs/morphio)

Morphs the font, so it renders worda as wordb.

## 📥 Installation

> [!NOTE]
> You can try Morphio without installing it [in the browser](https://pro-2684.github.io/Morphio/).

### Using [`binstall`](https://github.com/cargo-bins/cargo-binstall)

```shell
cargo binstall morphio
```

### Downloading from Releases

Navigate to the [Releases page](https://github.com/PRO-2684/Morphio/releases) and download respective binary for your platform. Make sure to give it execute permissions.

### Compiling from Source

```shell
cargo install morphio
```

## 📖 Usage

```shell
$ morphio --help
Usage: morphio -i <input> -o <output> [-y] [--] <from> <to>

Morphio: Morphs the font, so it renders worda as wordb.

Positional Arguments:
  from              word to morph from
  to                word to morph to

Options:
  -i, --input       input font file path
  -o, --output      output font file path
  -y, --yes         allow overwrite output file if it exists
  -h, --help        display usage information
```

## ✅ TODO

- [ ] Allow morphing multiple words in one go
- [ ] Allow morphing words with different lengths
    - Possibly using edit distance algorithms, but might need to favor substitutions over insertions/deletions
- [x] ServiceWorker
- [x] Option for enabling/disabling word matching
    - Say we want to morph "banana" to "orange"
    - When word matching is enabled, `xbanana` will not be morphed, because we're matching whole words, not letters
    - When word matching is disabled, `xbanana` will be morphed to `xorange`
- [x] Optimization
    - [x] Use `CoverageTable::Format2`, which allows for more efficient storage of contiguous ranges of glyphs
- [x] Better word matching
    - Currently we consider whole words only by letters (`[a-zA-Z]+`)
    - Next stage: consider digits (`[0-9]+`) and underscores (`_`)
    - Maybe an option for toggling which characters to consider as part of words?
- [x] Bug fixes
    - [x] Might not work with fonts with multiple language records, e.g. `IMPACT.TTF`

## 🎉 Credits

- [fontations](https://github.com/googlefonts/fontations/)
- [fonttools](https://github.com/fonttools/fonttools)
- [OpenType Layout Common Table Formats](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2)
- [GSUB — Glyph Substitution Table](https://learn.microsoft.com/en-us/typography/opentype/spec/gsub)
