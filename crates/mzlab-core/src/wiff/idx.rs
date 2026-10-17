//! The index streams of a `.wiff`: `SampleSubtree/SampleIdxTable` (where each sample's section starts in the `.wiff.scan`) and
//! `SampleSubtree/SampleN/Idx` (one 54-byte row per scan block after a 32-byte header). Layout found on the 3200 QTRAP files
//! (the OpenSXRaw layout, Apache-2.0, does not fit them): u32 offset from the section start, u32 size, f64 time in ms (+8), u8 flag (+16),
//! f64 total ion current in cps (+18), f64 (+26, the number of points of a scan, 0 for an empty one).

const HEADER: usize = 32;
const ROW: usize = 54;
const TABLE_ROW: usize = 16;
/// The table gives the position of the first record; the section starts 20 bytes before.
const TABLE_LEAD: u64 = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entry {
    pub offset: u32,
    pub size: u32,
    pub time_ms: f64,
    pub tic: f64,
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn f64_at(b: &[u8], at: usize) -> f64 {
    let mut t = [0u8; 8];
    t.copy_from_slice(&b[at..at + 8]);
    f64::from_le_bytes(t)
}

/// `(sample number, start of its section in the .scan)` for each row of `SampleIdxTable`.
pub fn sections(table: &[u8]) -> Vec<(u32, u64)> {
    let rows = table.get(HEADER..).unwrap_or(&[]);
    rows.chunks_exact(TABLE_ROW)
        .filter_map(|r| Some((u32_at(r, 0), (u32_at(r, 4) as u64).checked_sub(TABLE_LEAD)?)))
        .collect()
}

pub fn parse(data: &[u8]) -> Vec<Entry> {
    let body = data.get(HEADER..).unwrap_or(&[]);
    body.chunks_exact(ROW)
        .map(|r| Entry {
            offset: u32_at(r, 0),
            size: u32_at(r, 4),
            time_ms: f64_at(r, 8),
            tic: f64_at(r, 18),
        })
        .collect()
}
