use read_fonts::FontRef;
use std::fs::{read, write};
use text_morph::TextMorph;

fn main() {
    let data = read("./tests/fonts/msyh.ttc").unwrap();
    let font = FontRef::from_index(&data, 0).unwrap();
    let morphed = font.morph("banana", "orange").unwrap();
    write("./tests/fonts/msyh-morphed.ttc", morphed).unwrap();
}
