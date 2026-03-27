# Morphio

[![GitHub License](https://img.shields.io/github/license/PRO-2684/Morphio?logo=opensourceinitiative)](https://github.com/PRO-2684/Morphio/blob/main/LICENSE)
![GitHub Repo stars](https://img.shields.io/github/stars/PRO-2684/Morphio?style=flat)
![GitHub Issues or Pull Requests](https://img.shields.io/github/issues/PRO-2684/Morphio)
![GitHub Issues or Pull Requests](https://img.shields.io/github/issues-pr/PRO-2684/Morphio)
[![Crates.io Version](https://img.shields.io/crates/v/morphio?logo=rust)](https://crates.io/crates/morphio)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/morphio?logo=rust)](https://crates.io/crates/morphio)
[![docs.rs](https://img.shields.io/docsrs/morphio?logo=rust)](https://docs.rs/morphio)

Morphs the font, so one word renders as another.

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

### 💻 CLI

```shell
Morphio: Morphs the font, so one word renders as another.

Options:
  -i, --input       input font file path
  -o, --output      output font file path
  -r, --recipe      load morph rules and word matching options from a TOML
                    recipe file, ignoring command-line options
  -m, --no-word-match
                    disable both start and end word matching
  --no-word-match-start
                    disable word matching at the start of the source word
  --no-word-match-end
                    disable word matching at the end of the source word
  --skip-missing-glyphs
                    skip rules that reference missing glyphs instead of failing
  -y, --yes         allow overwrite output file if it exists
  -h, --help        display usage information
```

Without a recipe file, pass rules as positional `FROM TO` pairs:

```shell
morphio -i input.ttf -o output.ttf banana orange from to
```

To relax word-boundary matching or skip unsupported rules:

```shell
morphio -i input.ttf -o output.ttf --no-word-match-end --skip-missing-glyphs banana orange
```

To drive the CLI from a [recipe file](#-recipes):

```shell
morphio -i input.ttf -o output.ttf -r tests/recipes/simple.toml
```

### 🌐 Web Interface

The browser demo supports:

- Uploading a font file
- Adding multiple morph rules
- Toggling advanced options:
  - Match word start
  - Match word end
  - Skip missing glyphs
- Previewing original and morphed text side by side
- Downloading the morphed font
- Importing and exporting recipe TOML files

The web UI uses the same recipe format as the CLI and library.

## 🧾 Recipes

Recipes are TOML files that store:

- Morph rules
- Word-boundary matching options
- Whether rules with missing glyphs should be skipped

Example:

```toml
[options]
word_match_start = true
word_match_end = true
skip_missing_glyphs = false

[[rules]]
from = "banana"
to = "orange"

[[rules]]
from = "from"
to = "to"
```

Notes:

- `word_match_start = false` allows matches inside a longer word prefix, such as morphing `xbanana` to `xorange`.
- `word_match_end = false` allows matches before a longer word suffix, such as morphing `bananas` to `oranges`.
- `skip_missing_glyphs = true` ignores rules whose source or target characters are not present in the font instead of aborting the whole morph.

You can find example recipes under [`tests/recipes`](tests/recipes).

## ✅ TODO

- [ ] Actual shaping tests via [`harfrust`](https://github.com/harfbuzz/harfrust) (when it updates on crates.io to [use `read-fonts` 0.38.0](https://github.com/harfbuzz/harfrust/pull/348/changes#diff-40a9b26e08998f84bfd1780aef2fd166dc3c24c0ece4834f49622aedc2f0c1d7))
    - [`UnicodeBuffer`](https://docs.rs/harfrust/latest/harfrust/struct.UnicodeBuffer.html)
    - [`Shaper::shape`](https://docs.rs/harfrust/latest/harfrust/struct.Shaper.html#method.shape)
    - [`GlyphBuffer`](https://docs.rs/harfrust/latest/harfrust/struct.GlyphBuffer.html)
- [ ] Replace our rough implementation of TTC support after [`write-fonts` adds support for that](https://github.com/googlefonts/fontations/blob/2dec98a9d308c7b449a89eb49d3169464d6707c1/write-fonts/src/font_builder.rs#L265).
- [x] Recipe (configuration) support
    - Advanced settings
    - Morph rules
- [x] Skip rules with missing glyphs instead of failing
- [x] Configure word matching of start and end of words separately
- [x] Reduce TTC font sizes (table sharing?)
- [x] Allow morphing multiple words in one go
- [x] Allow morphing words with different lengths
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

The following is of low priority, and may not get implemented:

- Reuse 1 -> N and N -> 1 mappings
- Determine "best split point", instead of just split in the end

## 🎉 Credits

- [fontations](https://github.com/googlefonts/fontations/)
- [fonttools](https://github.com/fonttools/fonttools)
- [OpenType Layout Common Table Formats](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2)
- [GSUB — Glyph Substitution Table](https://learn.microsoft.com/en-us/typography/opentype/spec/gsub)
