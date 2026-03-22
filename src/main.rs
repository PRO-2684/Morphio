use read_fonts::{FontRef, ReadError, TableProvider};
use std::fs::read;
use write_fonts::{FontBuilder, from_obj::ToOwnedTable, tables::head::Head, types::LongDateTime};

fn get_font_name(font_ref: &FontRef) -> Result<Option<String>, ReadError> {
    let name = font_ref.name()?;
    let Some(record) = name.name_record().get(0) else {
        return Ok(None);
    };
    let string = record.string(name.string_data())?;
    let string = string
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>();
    Ok(Some(string))
}

fn main() {
    println!("Loading font...");
    let file = read("tests/fonts/msyh.ttc").unwrap();
    let font_ref = FontRef::from_index(&file, 0).unwrap();
    let font_name = get_font_name(&font_ref)
        .unwrap()
        .unwrap_or_else(|| "Unknown".into());
    println!("Font name: {font_name}");

    // let mut gsub = font_ref.gsub().unwrap();
}
