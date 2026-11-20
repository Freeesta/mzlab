//! What a `.wiff` says of each sample from the container and the `Idx` rows alone (`Reader::listing`): counts, times, TIC, section bytes, experiment kinds,
//! polarity and the method file. The `.wiff.scan` is never read here; a copy cut short shows as index rows lying past its end (`missing`).

use super::modes::{mode, Output};
use super::reader::{member, wiff_member, WiffReader, SCAN};
use super::Wiff;
use crate::error::{Error, Result};
use crate::ole::Ole;
use crate::readers::{FileSet, Limits, Listing, Summary};

/// Scans the index places in the data file, and the bytes of the section: the numbers the sample list holds before any block is read. A block counts as a scan
/// when its row holds points, or (IDA: the rows hold no count) when its experiment acquires scans.
pub(super) fn counts(w: &Wiff) -> (usize, u64) {
    let scan = |r: &super::Record| {
        !r.cut(w.scan_len)
            && (r.points > 0.0
                || (r.points.is_nan() && r.size > 0 && w.scan_experiment(r.experiment)))
    };
    let size = w.records.iter().map(|r| u64::from(r.size)).sum();
    (w.records.iter().filter(|r| scan(r)).count(), size)
}

/// The scan count and the index bytes of a sample as `open_with` lists it: the index's for a listing, the built scans' (no size) otherwise.
pub(super) fn counts_of(listing: bool, part: &super::reader::Part) -> (usize, Option<u64>) {
    if listing {
        let (n, size) = counts(&part.w);
        (n, Some(size))
    } else {
        (part.scans.len(), None)
    }
}

fn summary(w: &Wiff, method: Option<String>) -> Summary {
    let mut present = vec![false; w.experiments.len()];
    for r in &w.records {
        if let Some(p) = present.get_mut(r.experiment) {
            *p = true;
        }
    }
    let used: Vec<_> = w
        .experiments
        .iter()
        .zip(&present)
        .filter(|x| *x.1)
        .map(|x| x.0)
        .collect();
    let has = |f: &dyn Fn(&super::Experiment) -> bool| used.iter().any(|e| f(e));
    let out = |e: &super::Experiment| e.code.and_then(mode).map(|m| (m.output, m.level));
    let scan2 = |e: &super::Experiment| {
        out(e).is_some_and(|(o, l)| o == Output::Spettro && l.max(e.level) >= 2)
    };
    let kinds = [
        (
            "ms",
            has(&|e| out(e).is_none_or(|(o, _)| o == Output::Spettro) && !scan2(e)),
        ),
        ("ms2", has(&scan2)),
        ("sim", has(&|e| out(e).is_some_and(|x| x.0 == Output::Sim))),
        ("mrm", has(&|e| out(e).is_some_and(|x| x.0 == Output::Srm))),
        ("dad", w.dad.is_some()),
    ];
    let signs: Vec<i8> = used
        .iter()
        .map(|e| w.polarity_of(e))
        .filter(|p| *p != 0)
        .collect();
    let polarity = match (signs.contains(&1), signs.contains(&-1)) {
        (true, true) => Some("both"),
        (true, false) => Some("+"),
        (false, true) => Some("-"),
        _ => None,
    };
    let rows = w.records.iter();
    let apex = rows.clone().fold(None::<(f64, f64)>, |a, r| match a {
        Some((_, t)) if t >= r.tic => a,
        _ => Some((r.time_min, r.tic)),
    });
    Summary {
        kinds: kinds.iter().filter(|k| k.1).map(|k| k.0).collect(),
        polarity,
        rt: rows.clone().map(|r| r.time_min).fold(None, |a, t| match a {
            None => Some((t, t)),
            Some((lo, hi)) => Some((lo.min(t), hi.max(t))),
        }),
        tic: apex.map(|(rt, top)| (rt, top, rows.map(|r| r.tic).sum())),
        method,
        missing: w.records.iter().filter(|r| r.cut(w.scan_len)).count(),
        empty: Some(w.records.is_empty() && w.dad.is_none()),
    }
}

/// Where the data the index knows end in the `.wiff.scan`: the last block any row of any sample places (refused samples included: their rows are still the
/// index's), or the head of the last section when no row places a block. Bytes after it are sections the index lacks (a copy made while acquiring) or padding.
fn indexed_end(ole: &crate::ole::Ole) -> u64 {
    let table = ole
        .read(&format!("{}SampleIdxTable", super::SAMPLES))
        .map(|t| super::idx::sections(&t))
        .unwrap_or_default();
    let end = |(id, at): &(u32, Option<u64>)| {
        let at = (*at)?;
        let rows = ole
            .read(&format!("{}Sample{id}/Idx", super::SAMPLES))
            .map(|b| super::idx::parse(&b))
            .unwrap_or_default();
        let blocks = rows
            .iter()
            .filter(|e| !e.zeroed())
            .map(|e| at + u64::from(e.offset) + super::scan::PAYLOAD_SKIP + u64::from(e.size));
        Some(blocks.max().unwrap_or(0).max(at + 20 + 24))
    };
    table.iter().filter_map(end).max().unwrap_or(0)
}

/// `(id, start, end)` of every sample's section in a `.wiff.scan` of `scan_len` bytes, ascending by start: a section runs to the start of the next one, the last
/// to `end_of_index`. A sample with no section (nothing acquired) gets an empty range at 0.
fn section_ranges(ole: &crate::ole::Ole, scan_len: u64, end_of_index: u64) -> Vec<(u32, u64, u64)> {
    let table = ole
        .read(&format!("{}SampleIdxTable", super::SAMPLES))
        .map(|t| super::idx::sections(&t))
        .unwrap_or_default();
    let mut starts: Vec<(u64, u32)> = table
        .iter()
        .map(|(id, at)| (at.map_or(0, |a| a.min(scan_len)), *id))
        .collect();
    crate::sortc::slice_ord(&mut starts);
    let mut out = Vec::with_capacity(starts.len());
    for (i, (at, id)) in starts.iter().enumerate() {
        let none = table.iter().any(|(t, a)| t == id && a.is_none());
        let next = starts.iter().skip(i + 1).map(|x| x.0).find(|n| n > at);
        let end = next.unwrap_or_else(|| end_of_index.min(scan_len)).max(*at);
        out.push((*id, *at, if none { *at } else { end }));
    }
    out
}

impl WiffReader {
    pub(super) fn listing_of(&self) -> Listing {
        let end = indexed_end(&self.ole);
        let sections = self.parts.iter().map(|p| &p.w).collect::<Vec<_>>();
        Listing {
            bytes: self.ole.byte_len() + self.scan_len,
            scan: Some((self.scan_len, end)),
            sections: section_ranges(&self.ole, self.scan_len, end),
            samples: sections
                .iter()
                .map(|w| summary(w, super::method_name::dam_of(&self.ole, w.sample)))
                .collect(),
        }
    }
}

/// The name of the `.wiff` of `fs`, its container read within `limits` and the index of its `.wiff.scan` member.
pub(super) fn container_of(fs: &mut FileSet, limits: &Limits) -> Result<(String, Ole, usize)> {
    let wi = wiff_member(fs).ok_or_else(|| Error::new("err.file.notWiff").with("name", "?"))?;
    let name = member(fs, wi)?
        .nome
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("x.wiff")
        .to_string();
    let si = fs
        .find(SCAN)
        .ok_or_else(|| Error::new("err.file.wiffNoScan").with("name", &name))?;
    let m = member(fs, wi)?;
    let ole = super::container::open_member(m, &name, limits)?;
    Ok((name, ole, si))
}
