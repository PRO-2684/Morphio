use harfrust::{Direction, GlyphBuffer, ShapePlan, Shaper, ShaperData, UnicodeBuffer};
use morphio::Morphio;
use read_fonts::{FontRef, TableProvider, types::GlyphId};
use std::{collections::BTreeMap, fs::read, sync::LazyLock};

const DIRECTION: Direction = Direction::LeftToRight;

static FONT_DATA: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let font_data = read("tests/fonts/IMPACT.TTF").unwrap();
    let font = FontRef::new(&font_data).unwrap();
    let morphed_font_data = font.morph("from", "to").unwrap();
    morphed_font_data
});
static FONT_REF: LazyLock<FontRef<'static>> = LazyLock::new(|| FontRef::new(&FONT_DATA).unwrap());
static SHAPER_DATA: LazyLock<ShaperData> = LazyLock::new(|| ShaperData::new(&FONT_REF));
static SHAPER: LazyLock<Shaper> = LazyLock::new(|| SHAPER_DATA.shaper(&FONT_REF).build());
static SHAPE_PLAN: LazyLock<ShapePlan> =
    LazyLock::new(|| ShapePlan::new(&SHAPER, DIRECTION, None, None, &[]));
static GLYPH_ID_TO_CODEPOINT: LazyLock<BTreeMap<GlyphId, u32>> = LazyLock::new(|| {
    let mut map = BTreeMap::new();
    let cmap = FONT_REF.cmap().unwrap();
    for (codepoint, glyph_id) in cmap
        .encoding_records()
        .iter()
        .filter_map(|record| record.subtable(cmap.offset_data()).ok())
        .flat_map(|subtable| subtable.iter())
    {
        map.insert(glyph_id, codepoint);
    }
    map
});

fn shape(text: &str) -> GlyphBuffer {
    let mut buffer = UnicodeBuffer::new();
    buffer.set_direction(DIRECTION);
    buffer.push_str(text);
    SHAPER.shape_with_plan(&SHAPE_PLAN, buffer, &[])
}

fn glyph_id_to_char(glyph_id: GlyphId) -> Option<char> {
    GLYPH_ID_TO_CODEPOINT
        .get(&glyph_id)
        .and_then(|&codepoint| char::from_u32(codepoint))
}

fn buffer_to_string(buffer: &GlyphBuffer) -> Option<String> {
    buffer
        .glyph_infos()
        .iter()
        .map(|info| glyph_id_to_char(GlyphId::new(info.glyph_id)))
        .collect()
}

fn test_match(orig: &str, morphed: &str) {
    let buffer = shape(orig);
    let rendered = buffer_to_string(&buffer).unwrap();
    assert_eq!(rendered, morphed);
}

#[test]
fn test_unmatched() {
    test_match("Hello, world!", "Hello, world!");
}

#[test]
fn test_morph_simple() {
    test_match("from", "to");
}

#[test]
fn test_morph_match() {
    test_match("a from b", "a to b");
}

#[test]
fn test_morph_unmatched() {
    // We've got word match enabled, so `1from` should not match
    test_match("1from", "1from");
}
