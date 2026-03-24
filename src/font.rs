//! Internal helpers for reading and validating source fonts.

use read_fonts::{
    FontRef, TableProvider,
    tables::cmap::{CmapSubtable, PlatformId},
    types::GlyphId16,
};

use crate::MorphError;

/// The preferred order of `cmap` subtables to use when looking for a Unicode mapping.
const CMAP_PREFERENCES: &[(PlatformId, u16)] = &[
    (PlatformId::Windows, 10),
    (PlatformId::Unicode, 6),
    (PlatformId::Unicode, 4),
    (PlatformId::Windows, 1),
    (PlatformId::Unicode, 3),
    (PlatformId::Unicode, 2),
    (PlatformId::Unicode, 1),
    (PlatformId::Unicode, 0),
];

pub fn validate_words(
    font: &FontRef<'_>,
    from_word: &str,
    to_word: &str,
) -> Result<(Vec<GlyphId16>, Vec<GlyphId16>), MorphError> {
    if from_word.chars().count() != to_word.chars().count() {
        return Err(MorphError::DifferentLengths);
    }
    if from_word.is_empty() {
        return Err(MorphError::EmptyWord);
    }

    let cmap = best_cmap(font)?.ok_or(MorphError::MissingCmap)?;
    Ok((
        resolve_glyphs(&cmap, from_word)?,
        resolve_glyphs(&cmap, to_word)?,
    ))
}

pub fn ascii_letter_glyphs(font: &FontRef<'_>) -> Result<Vec<GlyphId16>, MorphError> {
    let cmap = best_cmap(font)?.ok_or(MorphError::MissingCmap)?;
    let mut glyphs = Vec::new();

    for ch in ('A'..='Z').chain('a'..='z') {
        if let Some(glyph) = cmap.map_codepoint(ch) {
            let glyph_u32 = u32::from(glyph);
            let glyph_u16 =
                u16::try_from(glyph_u32).map_err(|_| MorphError::GlyphIdOutOfRange(glyph_u32))?;
            let glyph = GlyphId16::new(glyph_u16);
            if !glyphs.contains(&glyph) {
                glyphs.push(glyph);
            }
        }
    }

    glyphs.sort_unstable();
    Ok(glyphs)
}

fn best_cmap<'a>(font: &'a FontRef<'a>) -> Result<Option<CmapSubtable<'a>>, MorphError> {
    let cmap = font.cmap()?;
    let records = cmap.encoding_records();

    for (platform_id, encoding_id) in CMAP_PREFERENCES {
        if let Some(record) = records.iter().find(|record| {
            record.platform_id() == *platform_id && record.encoding_id() == *encoding_id
        }) {
            let subtable = record.subtable(cmap.offset_data())?;
            return Ok(Some(subtable));
        }
    }

    Ok(None)
}

fn resolve_glyphs(cmap: &CmapSubtable<'_>, word: &str) -> Result<Vec<GlyphId16>, MorphError> {
    word.chars()
        .map(|ch| {
            let Some(glyph) = cmap.map_codepoint(ch) else {
                return Err(MorphError::MissingGlyph(ch));
            };
            let glyph_u32 = u32::from(glyph);
            let glyph_u16 =
                u16::try_from(glyph_u32).map_err(|_| MorphError::GlyphIdOutOfRange(glyph_u32))?;
            Ok(GlyphId16::new(glyph_u16))
        })
        .collect()
}
