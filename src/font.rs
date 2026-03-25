//! Internal helpers for reading and validating source fonts.

use std::collections::BTreeSet;

use range_set_blaze::RangeSetBlaze;
use read_fonts::{
    FontRef, TableProvider,
    tables::cmap::{CmapSubtable, PlatformId},
    types::GlyphId16,
};
use write_fonts::tables::layout::RangeRecord;

use super::MorphError;

/// The preferred order of `cmap` subtables to use when looking for a Unicode mapping. From [fonttools](https://github.com/fonttools/fonttools/blob/29a392f2b67be8ad0229a75e75893c8bd585d792/Lib/fontTools/ttLib/tables/_c_m_a_p.py#L82-L91).
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

pub fn word_to_glyphs(
    font: &FontRef<'_>,
    from_word: &str,
    to_word: &str,
) -> Result<(Vec<GlyphId16>, Vec<GlyphId16>), MorphError> {
    if from_word.is_empty() || to_word.is_empty() {
        return Err(MorphError::EmptyWord);
    }

    let cmap = best_cmap(font)?.ok_or(MorphError::MissingCmap)?;
    Ok((
        resolve_glyphs(&cmap, from_word)?,
        resolve_glyphs(&cmap, to_word)?,
    ))
}

pub fn find_unused_glyph(
    font: &FontRef<'_>,
    reserved_glyphs: &[GlyphId16],
) -> Result<Option<GlyphId16>, MorphError> {
    let cmap = best_cmap(font)?.ok_or(MorphError::MissingCmap)?;
    let num_glyphs = usize::from(font.maxp()?.num_glyphs());
    if num_glyphs <= 1 {
        return Ok(None);
    }

    let mut mapped = vec![false; num_glyphs];
    collect_mapped_glyphs(&cmap, &mut mapped);
    for glyph in reserved_glyphs {
        let index = usize::from(glyph.to_u16());
        if index < mapped.len() {
            mapped[index] = true;
        }
    }

    Ok(mapped
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, is_mapped)| !**is_mapped)
        .and_then(|(glyph_id, _)| u16::try_from(glyph_id).ok())
        .map(GlyphId16::new))
}

/// Returns the glyph ID ranges for all word characters in the font, sorted and merged. Including:
///
/// - ASCII letters (A-Z, a-z)
/// - ASCII digits (0-9)
/// - Underscore (_)
///
/// Ignores any characters that are not present in the font.
pub fn word_glyph_ranges(font: &FontRef<'_>) -> Result<Vec<RangeRecord>, MorphError> {
    let cmap = best_cmap(font)?.ok_or(MorphError::MissingCmap)?;
    let word_char_iter = ('A'..='Z')
        .chain('a'..='z')
        .chain('0'..='9')
        .chain(std::iter::once('_'));
    let word_glyph_id_iter = word_char_iter.filter_map(|ch| {
        if let Some(glyph) = cmap.map_codepoint(ch) {
            let glyph_u32 = u32::from(glyph);
            let glyph_u16 = u16::try_from(glyph_u32).ok()?; // TODO: Error on out-of-range glyph IDs?
            Some(glyph_u16)
        } else {
            None
        }
    });
    let word_glyph_ranges = RangeSetBlaze::from_iter(word_glyph_id_iter);
    let result = word_glyph_ranges
        .ranges()
        .scan(0, |coverage_index, r| {
            let range_length = r.end() - r.start() + 1;
            let record = RangeRecord {
                start_glyph_id: GlyphId16::new(*r.start()),
                end_glyph_id: GlyphId16::new(*r.end()),
                start_coverage_index: *coverage_index,
            };
            *coverage_index += range_length;
            Some(record)
        })
        .collect();
    Ok(result)
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

fn collect_mapped_glyphs(cmap: &CmapSubtable<'_>, mapped: &mut [bool]) {
    let glyph_ids = match cmap {
        CmapSubtable::Format0(_) => {
            Box::new(std::iter::empty()) as Box<dyn Iterator<Item = read_fonts::types::GlyphId>>
        }
        CmapSubtable::Format4(subtable) => Box::new(subtable.iter().map(|(_, glyph)| glyph)),
        CmapSubtable::Format6(subtable) => Box::new(subtable.iter().map(|(_, glyph)| glyph)),
        CmapSubtable::Format10(subtable) => Box::new(subtable.iter().map(|(_, glyph)| glyph)),
        CmapSubtable::Format12(subtable) => Box::new(subtable.iter().map(|(_, glyph)| glyph)),
        CmapSubtable::Format13(subtable) => Box::new(subtable.iter().map(|(_, glyph)| glyph)),
        _ => Box::new(std::iter::empty()),
    };

    let unique_glyphs = glyph_ids
        .map(|glyph: read_fonts::types::GlyphId| glyph.to_u32())
        .collect::<BTreeSet<_>>();
    for glyph in unique_glyphs {
        if let Ok(index) = usize::try_from(glyph) {
            if index < mapped.len() {
                mapped[index] = true;
            }
        }
    }
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
