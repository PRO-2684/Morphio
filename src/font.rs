//! Internal helpers for reading and validating source fonts.

use range_set_blaze::RangeSetBlaze;
use read_fonts::{
    FontRef, TableProvider,
    tables::cmap::{CmapSubtable, PlatformId},
    types::GlyphId16,
};
use write_fonts::tables::layout::RangeRecord;

use super::MorphError;

/// A morph rule resolved to glyph IDs for a particular font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMorphRule {
    /// Source glyph sequence to match.
    pub from_glyphs: Vec<GlyphId16>,
    /// Target glyph sequence to substitute.
    pub to_glyphs: Vec<GlyphId16>,
    /// Placeholder glyph assigned to this rule when a many-to-many variable-length
    /// rewrite requires an intermediate glyph.
    pub placeholder: Option<GlyphId16>,
}

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

/// Resolve a list of text rules into glyph rules for the provided font.
pub fn resolve_rules(
    font: &FontRef<'_>,
    rules: &[super::MorphRule<'_>],
) -> Result<Vec<ResolvedMorphRule>, MorphError> {
    let cmap = best_cmap(font)?.ok_or(MorphError::MissingCmap)?;
    rules
        .iter()
        .map(|rule| {
            if rule.from.is_empty() || rule.to.is_empty() {
                return Err(MorphError::EmptyWord);
            }
            Ok(ResolvedMorphRule {
                from_glyphs: resolve_glyphs(&cmap, rule.from)?,
                to_glyphs: resolve_glyphs(&cmap, rule.to)?,
                placeholder: None,
            })
        })
        .collect()
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
    let word_glyph_ranges: RangeSetBlaze<_> = word_glyph_id_iter.collect();
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
