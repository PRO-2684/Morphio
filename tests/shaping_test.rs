use harfrust::{Direction, GlyphBuffer, ShapePlan, Shaper, ShaperData, UnicodeBuffer};
use read_fonts::FontRef;
use std::{fs::read, path::PathBuf, sync::LazyLock};

static FONT_DATA: LazyLock<Vec<u8>> = LazyLock::new(|| read("tests/fonts/IMPACT.TTF").unwrap());
static FONT_REF: LazyLock<FontRef<'static>> = LazyLock::new(|| FontRef::new(&FONT_DATA).unwrap());
static SHAPER_DATA: LazyLock<ShaperData> = LazyLock::new(|| ShaperData::new(&FONT_REF));
static SHAPER: LazyLock<Shaper> = LazyLock::new(|| SHAPER_DATA.shaper(&FONT_REF).build());
static SHAPE_PLAN: LazyLock<ShapePlan> =
    LazyLock::new(|| ShapePlan::new(&SHAPER, Direction::default(), None, None, &[]));

fn shape(text: &str) -> GlyphBuffer {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    SHAPER.shape_with_plan(&SHAPE_PLAN, buffer, &[])
}
