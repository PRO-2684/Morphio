//! # Morphio
//!
//! Morphs the font, so it renders worda as wordb.
//!
//! ## Usage
//!
//! ### Loading and parsing a font
//!
//! You should load the font file as bytes (e.g. using [`std::fs::read`]) and parse it using [`read_fonts::FontRef`].
//!
//! ```rust
//! use read_fonts::FontRef;
//!
//! let path_to_font = "tests/fonts/INKFREE.TTF";
//! let font_data = std::fs::read(path_to_font).unwrap();
//! let font = FontRef::new(&font_data).unwrap();
//! ```
//!
//! ### Morphing the font
//!
//! Then, call the [`Morphio::morph`] method on the parsed font, passing in the two words you want to morph between.
//!
//! ```rust
//! # use read_fonts::FontRef;
//! use morphio::Morphio;
//!
//! # let path_to_font = "tests/fonts/INKFREE.TTF";
//! # let font_data = std::fs::read(path_to_font).unwrap();
//! # let font = FontRef::new(&font_data).unwrap();
//! let morphed_font_data = font.morph("worda", "wordb").unwrap(); // Of type `Vec<u8>`, containing the bytes of the morphed font
//! ```
//!
//! ### Verifying the morphed font
//!
//! We can verify the result by trying to parse the morphed font and check the expected GSUB feature is present.
//!
//! ```rust
//! # use read_fonts::FontRef;
//! # use morphio::Morphio;
//! use read_fonts::{TableProvider, types::Tag};
//!
//! # let path_to_font = "tests/fonts/INKFREE.TTF";
//! # let font_data = std::fs::read(path_to_font).unwrap();
//! # let font = FontRef::new(&font_data).unwrap();
//! # let morphed_font_data = font.morph("worda", "wordb").unwrap();
//! let morphed_font = FontRef::new(&morphed_font_data).unwrap();
//! let gsub = morphed_font.gsub().unwrap();
//! let feature_list = gsub.feature_list().unwrap();
//! let has_calt = feature_list.feature_records().iter().any(|record| record.feature_tag() == Tag::new(b"calt"));
//! assert!(has_calt, "morphed font should expose a calt feature for the morphing substitution");
//! ```
//!
//! ### Font collection support
//!
//! [`Morphio::morph`] can also be called on a font collection (e.g. a TTC). In this case, the morphing will be applied to every font in the collection, and the result will still be a valid font collection containing the morphed fonts.
//!
//! To load a font collection, use [`read_fonts::FileRef`] instead of [`read_fonts::FontRef`].
//!
//! ```rust
//! use read_fonts::FileRef;
//! use morphio::Morphio;
//!
//! let path_to_collection = "tests/fonts/msyh.ttc";
//! let collection_data = std::fs::read(path_to_collection).unwrap();
//! let collection = FileRef::new(&collection_data).unwrap();
//! let morphed_collection_data = collection.morph("abc", "xyz").unwrap();
//! let morphed_collection = FileRef::new(&morphed_collection_data).unwrap(); // Verify that it can be parsed
//! ```
//!
//! ## WebAssembly
//!
//! This library can also be compiled to WebAssembly, providing the [`morph_font_wasm`] function as the entry point (the JavaScript name is `morphFont`). See it in action in the [demo](https://pro-2684.github.io/Morphio/), and the source code is available in [the GitHub repo](https://github.com/PRO-2684/Morphio).

#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

mod error;
mod font;
mod gsub;
mod ttc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

pub use error::MorphError;
use read_fonts::{FileRef, FontRef, TableProvider, types::GlyphId16};
use ttc::build_ttc;
use write_fonts::{
    FontBuilder,
    from_obj::{FromTableRef, ToOwnedTable},
    tables::{
        glyf::{Glyf, GlyfLocaBuilder, Glyph},
        head::Head,
        hhea::Hhea,
        hmtx::{Hmtx, LongMetric},
        loca::{Loca, LocaFormat},
        maxp::Maxp,
    },
    types::GlyphId,
};

/// Options for morphing a font.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct MorphOptions {
    /// Whether to enable word matching.
    ///
    /// ## Example
    ///
    /// Say we want to morph "banana" to "orange". With word match enabled, "bananas" won't be affected; with it disabled, "bananas" will be rendered as "oranges".
    pub word_match: bool,
}

impl Default for MorphOptions {
    fn default() -> Self {
        Self { word_match: true }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl MorphOptions {
    /// Creates a new [`MorphOptions`].
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    #[must_use]
    #[allow(
        clippy::missing_const_for_fn,
        reason = "wasm_bindgen doesn't support const fns"
    )]
    pub fn new(word_match: bool) -> Self {
        Self { word_match }
    }
}

/// The main trait for "morphing" text.
pub trait Morphio {
    /// Patch the font so it renders `from_word` as `to_word`, returning the rebuilt font bytes. Note that the two words:
    ///
    /// - Must be non-empty
    /// - Must be fully supported by the font (i.e. all glyphs must be present)
    ///
    /// If multiple fonts are present (e.g. in a TTC), all fonts will be patched.
    ///
    /// ## Note
    ///
    /// If the `from_word` and `to_word` have different numbers of glyphs, and none of the numbers is 1, an empty placeholder glyph will be appended to the font.
    ///
    /// ## Errors
    ///
    /// See the [`MorphError`] enum for possible error cases.
    fn morph(&self, from_word: &str, to_word: &str) -> Result<Vec<u8>, MorphError> {
        self.morph_with_options(from_word, to_word, &MorphOptions::default())
    }

    /// Patch the font with options, so it renders `from_word` as `to_word`, returning the rebuilt font bytes. Note that the two words:
    ///
    /// - Must be non-empty
    /// - Must be fully supported by the font (i.e. all glyphs must be present)
    ///
    /// If multiple fonts are present (e.g. in a TTC), all fonts will be patched.
    ///
    /// ## Note
    ///
    /// If the `from_word` and `to_word` have different numbers of glyphs, and none of the numbers is 1, an empty placeholder glyph will be appended to the font.
    ///
    /// ## Errors
    ///
    /// See the [`MorphError`] enum for possible error cases.
    fn morph_with_options(
        &self,
        from_word: &str,
        to_word: &str,
        options: &MorphOptions,
    ) -> Result<Vec<u8>, MorphError>;
}

impl Morphio for FontRef<'_> {
    fn morph_with_options(
        &self,
        from_word: &str,
        to_word: &str,
        options: &MorphOptions,
    ) -> Result<Vec<u8>, MorphError> {
        morph_font(self.clone(), from_word, to_word, options)
    }
}

impl Morphio for FileRef<'_> {
    fn morph_with_options(
        &self,
        from_word: &str,
        to_word: &str,
        options: &MorphOptions,
    ) -> Result<Vec<u8>, MorphError> {
        match self {
            Self::Font(font) => font.morph_with_options(from_word, to_word, options),
            Self::Collection(collection) => {
                let fonts = collection
                    .iter()
                    .map(|font| font.map_err(MorphError::Read))
                    .map(|font| {
                        font.and_then(|font| font.morph_with_options(from_word, to_word, options))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(build_ttc(fonts))
            }
        }
    }
}

fn morph_font(
    font: FontRef<'_>,
    from_word: &str,
    to_word: &str,
    options: &MorphOptions,
) -> Result<Vec<u8>, MorphError> {
    let (from_glyphs, to_glyphs) = font::word_to_glyphs(&font, from_word, to_word)?;
    let glyph_patch = if from_glyphs.len() == to_glyphs.len() {
        None
    } else {
        Some(append_empty_placeholder_glyph(&font)?)
    };
    let placeholder = glyph_patch.as_ref().map(|patch| patch.placeholder);
    let gsub = gsub::patch_gsub(&font, &from_glyphs, &to_glyphs, placeholder, options)?;

    let mut builder = FontBuilder::new();
    builder.add_table(&gsub)?;
    if let Some(patch) = glyph_patch.and_then(|patch| patch.inserted_tables) {
        builder
            .add_table(&patch.head)?
            .add_table(&patch.hhea)?
            .add_table(&patch.hmtx)?
            .add_table(&patch.maxp)?
            .add_table(&patch.glyf)?
            .add_table(&patch.loca)?;
    }
    builder.copy_missing_tables(font);

    Ok(builder.build())
}

struct GlyphPatch {
    placeholder: GlyphId16,
    inserted_tables: Option<InsertedGlyphTables>,
}

struct InsertedGlyphTables {
    head: Head,
    hhea: Hhea,
    hmtx: Hmtx,
    maxp: Maxp,
    glyf: Glyf,
    loca: Loca,
}

fn append_empty_placeholder_glyph(font: &FontRef<'_>) -> Result<GlyphPatch, MorphError> {
    let mut head: Head = font.head()?.to_owned_table();
    let mut hhea: Hhea = font.hhea()?.to_owned_table();
    let mut hmtx: Hmtx = font.hmtx()?.to_owned_table();
    let mut maxp: Maxp = font.maxp()?.to_owned_table();
    let read_loca = font.loca(None)?;
    let read_glyf = font.glyf()?;

    let num_glyphs = maxp.num_glyphs;
    let placeholder = read_fonts::types::GlyphId16::new(num_glyphs);

    let mut glyf_builder = GlyfLocaBuilder::new();
    for glyph_id in 0..num_glyphs {
        let glyph = read_loca.get_glyf(GlyphId::new(u32::from(glyph_id)), &read_glyf)?;
        let glyph = glyph.as_ref().map_or(Glyph::Empty, Glyph::from_table_ref);
        glyf_builder.add_glyph(&glyph)?;
    }
    glyf_builder.add_glyph(&Glyph::Empty)?;
    let (glyf, loca, loca_format) = glyf_builder.build();

    maxp.num_glyphs = maxp
        .num_glyphs
        .checked_add(1)
        .ok_or(MorphError::UnsupportedPlaceholderGlyph)?;
    hmtx.h_metrics.push(LongMetric::new(0, 0));
    hhea.number_of_h_metrics = hhea
        .number_of_h_metrics
        .checked_add(1)
        .ok_or(MorphError::UnsupportedPlaceholderGlyph)?;
    head.index_to_loc_format = i16::from(matches!(loca_format, LocaFormat::Long));

    Ok(GlyphPatch {
        placeholder,
        inserted_tables: Some(InsertedGlyphTables {
            head,
            hhea,
            hmtx,
            maxp,
            glyf,
            loca,
        }),
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = morphFont)]
/// WebAssembly entry point that morphs the provided font bytes and returns rebuilt font data.
pub fn morph_font_wasm(
    font_data: &[u8],
    from_word: &str,
    to_word: &str,
    options: MorphOptions,
) -> Result<Vec<u8>, JsValue> {
    let file = FileRef::new(font_data).map_err(|err| JsValue::from_str(&err.to_string()))?;
    file.morph_with_options(from_word, to_word, &options)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}
