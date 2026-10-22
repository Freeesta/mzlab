//! The scan statistics of `ispezione/1`: what `Item.info()` does not carry about the scans of one file (MS1 cycle time, product-ion scans per
//! cycle, isolation windows, distinct precursors). Read from the scan table of the index, never from the peaks. Used by `inspect`.

use mzlab_core::idxread::Scan;
use mzlab_core::json::Value;
use mzlab_core::sortc;

use crate::inspect::{median, n, Obj};

/// The most isolation windows listed.
const MAX_WINDOWS: usize = 32;

/// Seconds between consecutive MS1 scans of the same polarity.
fn ms1_deltas(scans: &[Scan]) -> Vec<f64> {
    let mut deltas = Vec::new();
    for pol in [1i8, -1] {
        let rts: Vec<f64> = scans
            .iter()
            .filter(|s| s.level == 1 && s.polarity == pol)
            .map(|s| s.rt)
            .collect();
        deltas.extend(rts.windows(2).map(|w| (w[1] - w[0]) * 60.0));
    }
    deltas
}

/// Number of scans of level 2 or more between two MS1 scans.
fn per_cycle(scans: &[Scan]) -> Vec<f64> {
    let mut out = Vec::new();
    let mut open: Option<usize> = None;
    for s in scans {
        if s.level == 1 {
            if let Some(c) = open {
                out.push(c as f64);
            }
            open = Some(0);
        } else if let Some(c) = open.as_mut() {
            *c += 1;
        }
    }
    out
}

/// Distinct isolation windows `(lo, hi, count)` in tenths of m/z, sorted.
fn windows_of(scans: &[Scan]) -> Vec<(i64, i64, usize)> {
    let mut keys: Vec<(i64, i64)> = scans
        .iter()
        .filter(|s| s.level >= 2)
        .filter_map(|s| s.iso)
        .map(|(lo, hi)| ((lo * 10.0).round() as i64, (hi * 10.0).round() as i64))
        .collect();
    sortc::by(&mut keys, |a, b| a.cmp(b));
    let mut windows: Vec<(i64, i64, usize)> = Vec::new();
    for k in keys {
        match windows.last_mut() {
            Some(w) if (w.0, w.1) == k => w.2 += 1,
            _ => windows.push((k.0, k.1, 1)),
        }
    }
    windows
}

fn put_windows(o: &mut Obj, windows: &[(i64, i64, usize)]) {
    let widths = windows.iter().map(|w| (w.1 - w.0) as f64 / 10.0);
    let lo = widths.clone().fold(f64::INFINITY, f64::min);
    let hi = widths.fold(f64::NEG_INFINITY, f64::max);
    o.put("windows_distinct", Some(Value::Num(windows.len() as f64)));
    o.put("width_mz", Some(Value::Arr(vec![n(lo), n(hi)])));
    let list = windows.iter().take(MAX_WINDOWS).map(|w| {
        Value::Arr(vec![
            n(w.0 as f64 / 10.0),
            n(w.1 as f64 / 10.0),
            Value::Num(w.2 as f64),
        ])
    });
    o.put("windows", Some(Value::Arr(list.collect())));
}

/// Facts about the scans of one file: the MS1 cycle time, the number of product-ion scans per cycle, the number of distinct precursors and the
/// isolation windows (at most `MAX_WINDOWS`, rounded to 0.1, with how often each repeats).
pub fn scan_stats(scans: &[Scan]) -> Value {
    let mut deltas = ms1_deltas(scans);
    let mut cycle = per_cycle(scans);
    let mut precs: Vec<i64> = scans
        .iter()
        .filter_map(|s| s.precursor)
        .map(|p| (p * 10.0).round() as i64)
        .collect();
    sortc::by(&mut precs, |a, b| a.cmp(b));
    precs.dedup();
    let mut o = Obj(Vec::new());
    o.put("cycle_ms1_s", median(&mut deltas).map(n));
    if cycle.iter().any(|c| *c > 0.0) {
        o.put("ms2_per_cycle", median(&mut cycle).map(n));
    }
    if !precs.is_empty() {
        o.put("precursors_distinct", Some(Value::Num(precs.len() as f64)));
    }
    let windows = windows_of(scans);
    if !windows.is_empty() {
        put_windows(&mut o, &windows);
    }
    Value::Obj(o.0)
}
