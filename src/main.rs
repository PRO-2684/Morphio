use std::fs::{read, write};
use text_morph::TextMorph;

fn main() {
    let mut data = read("./tests/fonts/msyh.ttc").unwrap();
    data.morph("banana", "orange").unwrap();
    write("./tests/fonts/msyh-morphed.ttc", data).unwrap();
}
