//! Dumps diagnostic information for a font file or TTC.
use read_fonts::{FileRef, FontRef, TableProvider};
use std::{
    collections::BTreeMap,
    env::args,
    fs::{File, read},
    io::Write,
};
use write_fonts::{from_obj::ToOwnedTable, tables::gsub::Gsub, types::Tag};

fn main() {
    let path = args()
        .nth(1)
        .expect("Usage: cargo run --example dump -- <font-file> [output-file]");
    let output_path = args().nth(2).unwrap_or_else(|| "font_dump.txt".to_string());

    let data = read(&path).expect("Failed to read font file");
    let file = FileRef::new(&data).expect("Failed to parse font file");

    let mut output = String::new();
    output.push_str(&format!("Path: {path}\n"));
    output.push_str(&format!("File bytes: {}\n", data.len()));

    match file {
        FileRef::Font(font) => {
            output.push_str("Kind: single font\n\n");
            dump_font(&mut output, &font, 0);
        }
        FileRef::Collection(collection) => {
            output.push_str(&format!("Kind: TTC ({} fonts)\n\n", collection.len()));
            let mut shared_payloads = BTreeMap::<Vec<u8>, Vec<(usize, String)>>::new();
            for (index, font) in collection.iter().enumerate() {
                let font = font.expect("Collection member should parse");
                dump_font(&mut output, &font, index);
                collect_payloads(&font, index, &mut shared_payloads);
                output.push('\n');
            }
            dump_shared_payloads(&mut output, &shared_payloads);
        }
    }

    let mut file = File::create(&output_path).expect("Failed to create output file");
    file.write_all(output.as_bytes())
        .expect("Failed to write diagnostic dump");
}

fn dump_font(output: &mut String, font: &FontRef<'_>, index: usize) {
    output.push_str(&format!("Font #{index}\n"));
    output.push_str(&format!(
        "TTC index: {}\n",
        font.ttc_index()
            .map_or_else(|| "standalone".to_string(), |idx| idx.to_string())
    ));
    output.push_str(&format!(
        "Table count: {}\n",
        font.table_directory().table_records().len()
    ));
    output.push_str("Tables:\n");

    for record in font.table_directory().table_records() {
        output.push_str(&format!(
            "  {} checksum={:#010x} offset={} length={}\n",
            tag_to_string(record.tag()),
            record.checksum(),
            record.offset(),
            record.length(),
        ));
    }

    match font.gsub() {
        Ok(gsub) => {
            let gsub: Gsub = gsub.to_owned_table();
            output.push_str("\nGSUB:\n");
            output.push_str(&format!("{gsub:#?}\n"));
        }
        Err(_) => output.push_str("\nGSUB: <missing>\n"),
    }
}

fn collect_payloads(
    font: &FontRef<'_>,
    font_index: usize,
    shared_payloads: &mut BTreeMap<Vec<u8>, Vec<(usize, String)>>,
) {
    for record in font.table_directory().table_records() {
        let tag = record.tag();
        if let Some(data) = font.table_data(tag) {
            shared_payloads
                .entry(data.as_bytes().to_vec())
                .or_default()
                .push((font_index, tag_to_string(tag)));
        }
    }
}

fn dump_shared_payloads(
    output: &mut String,
    shared_payloads: &BTreeMap<Vec<u8>, Vec<(usize, String)>>,
) {
    output.push_str("Shared table payloads across TTC members:\n");
    let mut found_any = false;
    for (payload, locations) in shared_payloads {
        if locations.len() < 2 {
            continue;
        }
        found_any = true;
        output.push_str(&format!("  {} bytes shared by ", payload.len()));
        for (idx, (font_index, tag)) in locations.iter().enumerate() {
            if idx > 0 {
                output.push_str(", ");
            }
            output.push_str(&format!("font #{font_index}:{tag}"));
        }
        output.push('\n');
    }
    if !found_any {
        output.push_str("  <none>\n");
    }
}

fn tag_to_string(tag: Tag) -> String {
    let bytes = tag.to_be_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}
