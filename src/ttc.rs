/// Build a TTC from the given font bytes, rebasing all internal offsets by the appropriate amount. Implemented here because `write-fonts` [doesn't support TTC yet](https://github.com/googlefonts/fontations/blob/423de8c29d960f1d2dd691c325a1bf41dda8513e/write-fonts/src/font_builder.rs#L265).
pub fn build_ttc(mut fonts: Vec<Vec<u8>>) -> Vec<u8> {
    let header_len = 12 + fonts.len() * 4;
    let mut offsets = Vec::with_capacity(fonts.len());
    let mut offset = header_len as u32;

    for font in &fonts {
        offsets.push(offset);
        offset += align_4(font.len()) as u32;
    }

    for (font, offset) in fonts.iter_mut().zip(offsets.iter().copied()) {
        rebase_sfnt_offsets(font, offset);
    }

    let mut ttc = Vec::with_capacity(offset as usize);
    ttc.extend_from_slice(b"ttcf");
    ttc.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    ttc.extend_from_slice(&(fonts.len() as u32).to_be_bytes());
    for offset in offsets {
        ttc.extend_from_slice(&offset.to_be_bytes());
    }

    for font in fonts {
        ttc.extend_from_slice(&font);
        let padding = align_4(font.len()) - font.len();
        ttc.resize(ttc.len() + padding, 0);
    }

    ttc
}

fn align_4(len: usize) -> usize {
    (len + 3) & !3
}

fn rebase_sfnt_offsets(font: &mut [u8], delta: u32) {
    if font.len() < 12 {
        return;
    }

    let num_tables = u16::from_be_bytes([font[4], font[5]]) as usize;
    let records_start = 12;

    for i in 0..num_tables {
        let record_offset = records_start + i * 16;
        if record_offset + 12 > font.len() {
            return;
        }

        let table_offset = u32::from_be_bytes([
            font[record_offset + 8],
            font[record_offset + 9],
            font[record_offset + 10],
            font[record_offset + 11],
        ]);
        let rebased = table_offset.saturating_add(delta).to_be_bytes();
        font[record_offset + 8..record_offset + 12].copy_from_slice(&rebased);
    }
}
