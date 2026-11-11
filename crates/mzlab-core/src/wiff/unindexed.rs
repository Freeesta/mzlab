//! Sections of a `.wiff.scan` that the `.wiff` index does not list: a copy made while Analyst was still acquiring (the index is saved when a sample ends,
//! the data grow all the time). Only MRM layouts are recovered: their blocks have one fixed size, so the geometry follows from the nearest indexed sample
//! whose blocks fit; the name, the method and the times of such a sample are inferred, never read, and the sample says so (`Wiff::inferred_from`).

use super::{Record, Wiff};
use crate::idxread::ReadAt;

/// Eight bytes that start every section of the `.wiff.scan`, then the u32 sample number; the first block lies `LEAD` bytes after the marker.
const MARKER: [u8; 8] = [0x11, 0x11, 0x11, 0x11, 0x82, 0x05, 0, 0];
const LEAD: u64 = 24;
const CHUNK: u64 = 1 << 20;

/// What was found after the last indexed section: the samples rebuilt (in file order) and how many sections were left out because their blocks could not be checked.
pub(super) struct Found {
    pub(super) samples: Vec<Wiff>,
    pub(super) skipped: usize,
}

fn read(scan: &mut dyn ReadAt, at: u64, n: u64) -> Option<Vec<u8>> {
    let mut b = vec![0u8; usize::try_from(n).ok()?];
    scan.read_at(at, &mut b).ok()?;
    Some(b)
}

/// The sample number of the section whose marker is at `at`, if there is one.
fn marker_at(scan: &mut dyn ReadAt, scan_len: u64, at: u64) -> Option<u32> {
    if at.checked_add(12)? > scan_len {
        return None;
    }
    marker_id(&read(scan, at, 12)?)
}

/// The sample number of a marker at the start of `b` (12 bytes: marker and u32), if it is one.
fn marker_id(b: &[u8]) -> Option<u32> {
    let id = u32::from_le_bytes(b.get(8..12)?.try_into().ok()?);
    (b.get(..8)? == MARKER).then_some(id)
}

/// The position of the next marker at or after `from` (a marker of a sample number above `above`), else the end of the file.
fn next_marker(scan: &mut dyn ReadAt, scan_len: u64, from: u64, above: u32) -> u64 {
    let mut at = from;
    while at + 12 <= scan_len {
        let n = (CHUNK + 11).min(scan_len - at);
        let Some(b) = read(scan, at, n) else { break };
        for (i, w) in b.windows(12).enumerate() {
            if marker_id(w).is_some_and(|id| id > above) {
                return at + i as u64;
            }
        }
        if at + n >= scan_len {
            break;
        }
        at += n - 11;
    }
    scan_len
}

/// The block size of the indexed sample `w` when its layout lets a later section be read: one fixed size (a multiple of 4: f32 per transition), MRM only.
fn block_size(w: &Wiff) -> Option<u64> {
    let size = u64::from(w.records.first()?.size);
    let mrm = !w.experiments.is_empty() && (0..w.experiments.len()).all(|e| !w.scan_experiment(e));
    (mrm && size >= 4
        && size.is_multiple_of(4)
        && size <= CHUNK
        && w.records.iter().all(|r| u64::from(r.size) == size))
    .then_some(size)
}

/// Every f32 of the `rows` blocks of `size` bytes from `first` is a finite number of counts (not below zero): the blocks are really the ones of that layout.
fn plausible(scan: &mut dyn ReadAt, first: u64, rows: u64, size: u64) -> bool {
    let per = (CHUNK / size).max(1);
    let mut row = 0;
    while row < rows {
        let n = per.min(rows - row);
        let Some(b) = read(scan, first + row * size, n * size) else {
            return false;
        };
        if !b.chunks_exact(4).all(|c| {
            let v = c.try_into().map_or(f32::NAN, f32::from_le_bytes);
            v.is_finite() && v >= 0.0
        }) {
            return false;
        }
        row += n;
    }
    true
}

/// The time of block `k` of the section, inferred from the rows of the donor: the same method has the same cycle, so its times are continued.
fn time_of(donor: &Wiff, cycle: usize, n_exp: usize) -> f64 {
    let t = |c: usize| donor.records.get(c * n_exp).map(|r| r.time_min);
    let last = donor.records.len().saturating_sub(1) / n_exp;
    match (t(cycle), t(0), t(last)) {
        (Some(x), ..) => x,
        (None, Some(a), Some(b)) if last > 0 => a + (b - a) / last as f64 * cycle as f64,
        _ => 0.0,
    }
}

/// The sections after the last block the index knows (`indexed`, in file order; `end_of_index`: where the indexed data end), rebuilt as samples when their blocks
/// check out. Nothing when the `.wiff.scan` ends there or what follows is not a section marker.
pub(super) fn recover(
    scan: &mut dyn ReadAt,
    scan_len: u64,
    indexed: &[&Wiff],
    end_of_index: u64,
    last_id: u32,
) -> Found {
    let mut out = Found {
        samples: Vec::new(),
        skipped: 0,
    };
    let (mut at, mut above) = (end_of_index, last_id);
    while let Some(id) = marker_at(scan, scan_len, at).filter(|id| *id > above) {
        let first = at + LEAD;
        let end = next_marker(scan, scan_len, first, id);
        above = id;
        let donor = indexed
            .iter()
            .rev()
            .find_map(|w| Some((*w, block_size(w)?)));
        let built = donor.and_then(|(d, size)| {
            let rows = end.saturating_sub(first) / size;
            // between two markers the blocks fill the section exactly; at the end of the file the last one may be cut
            let whole = end == scan_len || (end - first).is_multiple_of(size);
            if rows > 0 && whole && plausible(scan, first, rows, size) {
                rebuild(d, id, first, rows, u32::try_from(size).ok()?)
            } else {
                None
            }
        });
        match built {
            Some(w) => out.samples.push(w),
            None => out.skipped += 1,
        }
        at = end;
    }
    out
}

fn rebuild(donor: &Wiff, id: u32, first: u64, rows: u64, size: u32) -> Option<Wiff> {
    let n_exp = donor.experiments.len().max(1);
    let records = (0..rows)
        .map(|k| {
            let (cycle, at) = (usize::try_from(k).ok()?, first + k * u64::from(size));
            Some(Record {
                at,
                end: at + u64::from(size),
                size,
                time_min: time_of(donor, cycle / n_exp, n_exp),
                tic: 0.0,
                cycle: cycle / n_exp,
                experiment: cycle % n_exp,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Wiff {
        sample: id,
        records,
        experiments: donor.experiments.clone(),
        polarity: donor.polarity,
        method: donor.method.clone(),
        dad: None,
        model: donor.model.clone(),
        serial: donor.serial.clone(),
        source_type: donor.source_type.clone(),
        created: donor.created.clone(),
        wiff_sha1: donor.wiff_sha1.clone(),
        trim_scheduled: false,
        inferred_from: Some(donor.sample),
        scan_len: donor.scan_len,
        failure: None,
    })
}
