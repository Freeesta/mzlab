//! Sections of a `.wiff.scan` that the `.wiff` index does not list: a copy made while Analyst was still acquiring (the index is saved when a sample ends,
//! the data grow all the time). Only MRM layouts are recovered: their blocks have one fixed size, so the geometry follows from the nearest indexed sample
//! whose blocks fit; the name, the method and the times of such a sample are inferred, never read, and the sample says so (`Wiff::inferred_from`).

use super::reader::{build, Part};
use super::{Record, Wiff};
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::ole::Ole;
use crate::readers::{Limits, Sample};

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

/// The sections of the `.wiff.scan` the `.wiff` index does not list (the copy was made while the acquisition ran), rebuilt as samples after the indexed ones
/// with an inferred name and method; `(recovered, left out)`.
pub(super) fn add(
    parti: &mut Vec<Part>,
    campioni: &mut Vec<Sample>,
    raw_names: &mut Vec<(u32, Option<String>)>,
    (scan, scan_len, membro): (&mut Box<dyn ReadAt>, u64, u16),
    (ole, limits, only): (&Ole, &Limits, Option<usize>),
    total: &mut u64,
) -> Result<(usize, usize)> {
    let known: Vec<&Wiff> = parti.iter().map(|p| &p.w).collect();
    let (end, last_id) = index_end(ole, &known);
    let found = recover(scan, scan_len, &known, end, last_id);
    let counts = (found.samples.len(), found.skipped);
    for w in found.samples {
        let (i, s) = (parti.len(), w.sample);
        *total += w.records.len() as u64;
        limits.ok_entries(*total)?;
        let mut part = Part::new(w);
        if only.is_none_or(|c| c == i) && part.w.readable().is_ok() {
            build(&mut part, scan, scan_len, membro)?;
        }
        raw_names.push((s, None));
        campioni.push(Sample {
            id: s,
            nome: String::new(),
            scansioni: part.scans.len(),
            size: None,
        });
        parti.push(part);
    }
    Ok(counts)
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
                points: f64::NAN,
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
        devices: Vec::new(),
        model: donor.model.clone(),
        serial: donor.serial.clone(),
        source_type: donor.source_type.clone(),
        created: donor.created.clone(),
        wiff_sha1: donor.wiff_sha1.clone(),
        trim_scheduled: false,
        inferred_from: Some(donor.sample),
        scan_len: donor.scan_len,
        unplaced: 0,
        failure: None,
    })
}

/// Where the data the index knows end in the `.wiff.scan`: after the last block of a sample, or after the head of the last section of the sample table (an empty
/// injection has no blocks); and the highest sample number of the table.
pub(super) fn index_end(ole: &Ole, indexed: &[&Wiff]) -> (u64, u32) {
    let table = ole
        .read(&format!("{}SampleIdxTable", super::SAMPLES))
        .map(|t| super::idx::sections(&t))
        .unwrap_or_default();
    let heads = table
        .iter()
        .filter_map(|s| Some(s.1? + 20 + 24))
        .max()
        .unwrap_or(0);
    let blocks = indexed
        .iter()
        .flat_map(|w| w.records.iter())
        .map(|r| r.at + u64::from(r.size))
        .max()
        .unwrap_or(0);
    (
        heads.max(blocks),
        table.iter().map(|s| s.0).max().unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::error::Error;

    fn sample(times: &[f64], size: u32, codes: &[u32]) -> Wiff {
        let mut w = Wiff::refused(1, 0, Error::new("x"));
        w.records = times
            .iter()
            .map(|&time_min| Record {
                at: 0,
                end: 0,
                size,
                time_min,
                tic: 0.0,
                cycle: 0,
                experiment: 0,
                points: 0.0,
            })
            .collect();
        w.experiments = codes
            .iter()
            .map(|&c| super::super::Experiment {
                code: Some(c),
                ..Default::default()
            })
            .collect();
        w
    }

    #[test]
    fn a_cycle_past_the_donor_continues_its_mean_step() {
        let d = sample(&[1.0, 2.0, 3.0, 4.0], 8, &[4]);
        assert_eq!(time_of(&d, 2, 1), 3.0);
        // three steps of 1 min from 1.0: cycle 6 is at 7.0
        assert_eq!(time_of(&d, 6, 1), 7.0);
        // two experiments per cycle: the cycles are every other row, the last one is cycle 3
        let two = sample(&[1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5], 8, &[4, 4]);
        assert_eq!(time_of(&two, 1, 2), 2.0);
        assert_eq!(time_of(&two, 6, 2), 7.0);
        assert_eq!(time_of(&sample(&[], 8, &[4]), 3, 1), 0.0);
    }

    #[test]
    fn only_an_mrm_sample_of_one_block_size_can_be_continued() {
        assert_eq!(block_size(&sample(&[0.0, 1.0], 8, &[4])), Some(8));
        // a full scan (EMS) or no experiment at all: the blocks have no fixed size
        assert_eq!(block_size(&sample(&[0.0, 1.0], 8, &[15])), None);
        assert_eq!(block_size(&sample(&[0.0, 1.0], 8, &[])), None);
        assert_eq!(block_size(&sample(&[0.0, 1.0], 6, &[4])), None);
    }
}
