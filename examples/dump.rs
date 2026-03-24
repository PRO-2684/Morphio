//! Dumps the font's gsub table.
use read_fonts::{FontRef, TableProvider};
use std::{
    env::args,
    fs::{File, read},
    io::Write,
};
use write_fonts::{from_obj::ToOwnedTable, tables::gsub::Gsub};

fn main() {
    let path = args()
        .nth(1)
        .expect("Usage: gsub_dump <font-file> [output-file]");
    let output_path = args().nth(2).unwrap_or_else(|| "gsub_dump.txt".to_string());

    let data = read(path).expect("Failed to read font file");
    let font = FontRef::new(&data).expect("Failed to parse font file");

    let gsub = font.gsub().expect("Font does not have a gsub table");
    let gsub: Gsub = gsub.to_owned_table();

    let mut file = File::create(&output_path).expect("Failed to create output file");
    write!(file, "{gsub:#?}").expect("Failed to write gsub table to file");
}
