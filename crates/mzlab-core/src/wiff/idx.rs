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
    /// The point count of a scan block; 0 for an empty scan and for an MRM block.
    pub points: f64,
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn f64_at(b: &[u8], at: usize) -> f64 {
    let mut t = [0u8; 8];
    t.copy_from_slice(&b[at..at + 8]);
    f64::from_le_bytes(t)
}

/// `(sample number, start of its section in the .scan)` for each row of `SampleIdxTable`; `None` for a row whose position is before any section
/// (an injection that acquired nothing writes 0): the sample is kept, with no blocks to place.
pub fn sections(table: &[u8]) -> Vec<(u32, Option<u64>)> {
    let rows = table.get(HEADER..).unwrap_or(&[]);
    rows.chunks_exact(TABLE_ROW)
        .map(|r| (u32_at(r, 0), (u32_at(r, 4) as u64).checked_sub(TABLE_LEAD)))
        .collect()
}

impl Entry {
    /// Every field of the row is zero: it places no block (its offset 0 would read the first block of the section again, and the last row would run
    /// to the end of the file), so it is left out and counted, never read as a scan at rt 0.
    pub fn zeroed(&self) -> bool {
        self.offset == 0
            && self.size == 0
            && self.time_ms == 0.0
            && self.tic == 0.0
            && self.points == 0.0
    }
}

pub fn parse(data: &[u8]) -> Vec<Entry> {
    let body = data.get(HEADER..).unwrap_or(&[]);
    body.chunks_exact(ROW)
        .map(|r| Entry {
            offset: u32_at(r, 0),
            size: u32_at(r, 4),
            time_ms: f64_at(r, 8),
            tic: f64_at(r, 18),
            points: f64_at(r, 26),
        })
        .collect()
}

/// The blocks the rows of a sample place in a section that starts at `section`: each runs to the start of the next placed row, the last to `scan_len`.
/// A zeroed row places none, and no row of a sample without a section does; they are counted (`unplaced`, `warn.wiff.damaged`), except a zeroed first
/// row before a placed one (an empty first scan at time 0 holds nothing to lose).
pub(super) fn place(
    entries: &[Entry],
    section: Option<u64>,
    scan_len: u64,
    n_exp: usize,
    ida: bool,
) -> (Vec<super::Record>, usize) {
    let Some(section) = section else {
        return (Vec::new(), entries.len());
    };
    let kept: Vec<(usize, &Entry)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.zeroed())
        .collect();
    let empty_first =
        entries.first().is_some_and(Entry::zeroed) && entries.get(1).is_some_and(|e| !e.zeroed());
    let start = |e: &Entry| section + u64::from(e.offset) + super::scan::PAYLOAD_SKIP;
    let records = kept
        .iter()
        .enumerate()
        .map(|(i, &(k, e))| super::Record {
            at: start(e),
            end: kept.get(i + 1).map_or(scan_len, |n| start(n.1)),
            size: e.size,
            time_min: e.time_ms / 60000.0,
            tic: e.tic,
            cycle: k / n_exp,
            experiment: k % n_exp,
            points: if ida { f64::NAN } else { e.points },
        })
        .collect();
    (
        records,
        entries.len() - kept.len() - usize::from(empty_first),
    )
}
