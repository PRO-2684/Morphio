#![allow(
    clippy::cast_possible_truncation,
    reason = "TTC offsets and lengths are u32-based."
)]
//! Build TTC files while deduplicating identical table payloads.

use std::collections::HashMap;

use read_fonts::{FontRef, types::Tag};

/// Build a TTC from standalone sfnt font binaries.
///
/// Identical table payloads are shared across collection members, which keeps
/// rebuilt TTCs much closer to the size characteristics of the original file.
pub fn build_ttc(fonts: &[Vec<u8>]) -> Vec<u8> {
    let fonts = fonts
        .iter()
        .map(|font| parse_font(font))
        .collect::<Vec<_>>();

    let ttc_header_len = 12 + fonts.len() * 4;
    let mut directory_offsets = Vec::with_capacity(fonts.len());
    let mut cursor = ttc_header_len;
    for font in &fonts {
        directory_offsets.push(cursor as u32);
        cursor += 12 + font.tables.len() * 16;
    }
    cursor = align_4(cursor);

    let mut payload_offsets = HashMap::<Vec<u8>, u32>::new();
    let mut payloads = Vec::<(u32, Vec<u8>)>::new();
    for font in &fonts {
        for table in &font.tables {
            if payload_offsets.contains_key(&table.data) {
                continue;
            }
            let offset = cursor as u32;
            payload_offsets.insert(table.data.clone(), offset);
            payloads.push((offset, table.data.clone()));
            cursor += align_4(table.data.len());
        }
    }

    let mut ttc = vec![0; cursor];
    ttc[0..4].copy_from_slice(b"ttcf");
    ttc[4..8].copy_from_slice(&0x0001_0000_u32.to_be_bytes());
    ttc[8..12].copy_from_slice(&(fonts.len() as u32).to_be_bytes());
    for (index, offset) in directory_offsets.iter().enumerate() {
        let start = 12 + index * 4;
        ttc[start..start + 4].copy_from_slice(&offset.to_be_bytes());
    }

    for (font, directory_offset) in fonts.iter().zip(directory_offsets) {
        write_font_directory(&mut ttc, directory_offset as usize, font, &payload_offsets);
    }

    for (offset, payload) in payloads {
        let start = offset as usize;
        ttc[start..start + payload.len()].copy_from_slice(&payload);
    }

    ttc
}

struct ParsedFont {
    sfnt_version: u32,
    search_range: u16,
    entry_selector: u16,
    range_shift: u16,
    tables: Vec<ParsedTable>,
}

struct ParsedTable {
    tag: Tag,
    checksum: u32,
    data: Vec<u8>,
}

fn parse_font(font_data: &[u8]) -> ParsedFont {
    let font = FontRef::new(font_data).expect("standalone font should parse before TTC assembly");
    let directory = font.table_directory();
    let tables = directory
        .table_records()
        .iter()
        .map(|record| {
            let tag = record.tag();
            let table = font
                .table_data(tag)
                .expect("table record should resolve to data")
                .as_bytes()
                .to_vec();
            ParsedTable {
                tag,
                checksum: record.checksum(),
                data: table,
            }
        })
        .collect();

    ParsedFont {
        sfnt_version: directory.sfnt_version(),
        search_range: directory.search_range(),
        entry_selector: directory.entry_selector(),
        range_shift: directory.range_shift(),
        tables,
    }
}

fn write_font_directory(
    ttc: &mut [u8],
    offset: usize,
    font: &ParsedFont,
    payload_offsets: &HashMap<Vec<u8>, u32>,
) {
    ttc[offset..offset + 4].copy_from_slice(&font.sfnt_version.to_be_bytes());
    ttc[offset + 4..offset + 6].copy_from_slice(&(font.tables.len() as u16).to_be_bytes());
    ttc[offset + 6..offset + 8].copy_from_slice(&font.search_range.to_be_bytes());
    ttc[offset + 8..offset + 10].copy_from_slice(&font.entry_selector.to_be_bytes());
    ttc[offset + 10..offset + 12].copy_from_slice(&font.range_shift.to_be_bytes());

    let mut record_offset = offset + 12;
    for table in &font.tables {
        let payload_offset = payload_offsets
            .get(&table.data)
            .expect("payload offset should have been assigned");
        ttc[record_offset..record_offset + 4].copy_from_slice(&table.tag.to_be_bytes());
        ttc[record_offset + 4..record_offset + 8].copy_from_slice(&table.checksum.to_be_bytes());
        ttc[record_offset + 8..record_offset + 12].copy_from_slice(&payload_offset.to_be_bytes());
        ttc[record_offset + 12..record_offset + 16]
            .copy_from_slice(&(table.data.len() as u32).to_be_bytes());
        record_offset += 16;
    }
}

const fn align_4(len: usize) -> usize {
    (len + 3) & !3
}
