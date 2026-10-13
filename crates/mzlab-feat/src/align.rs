//! Alignment of the features of a series of files in groups ("per ion" mode): port of `TP_Mine/py/tpmine/hr/features.py::align`.
//! The tables (sorted by m/z) are merged k ways in file order, which is the stable sort of the concatenation of Python. A chain of
//! features whose consecutive m/z gaps are below `ppm` is one slab (no group crosses its edges): it is cut where it spans more than
//! `max_ppm_span`, sorted by RT and cut where the RT gap is above `rt_gap`; so one slab is held at a time.

use crate::axis::half;
use crate::sortc;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::rtmap::RtMap;
use crate::table::{Aligned, FileFeatures, CELL_ABSENT, CELL_DETECTED, FLAG_FT_LOBE};

/// Parameters of `align`.
#[derive(Debug, Clone, Copy)]
pub struct AlignParams {
    pub ppm: f64,
    pub rt_gap: f64,
    pub max_ppm_span: f64,
    pub max_rt_span: Option<f64>,
    pub drop_artefacts: bool,
    /// A group with two features of the same file (two close isomers) is cut at the RT gaps above this, minutes (None: never).
    pub split_gap: Option<f64>,
}

impl Default for AlignParams {
    fn default() -> Self {
        AlignParams {
            ppm: 5.0,
            rt_gap: 0.1,
            max_ppm_span: 10.0,
            max_rt_span: None,
            drop_artefacts: true,
            split_gap: None,
        }
    }
}

/// Files that must show two features on both sides of a cut for `split_duplicates`.
const MIN_SPLIT_FILES: i32 = 9;

/// A feature of the slab: (file, row in the file).
type Item = (u32, u32);

/// RT of a feature (minutes): the one of the table, or the reference RT of the map when one is given.
fn rt_of(files: &[&FileFeatures], adj: Option<&RtMap>, t: &Item) -> f64 {
    let r = files[t.0 as usize].rt[t.1 as usize] as f64;
    adj.map_or(r, |m| m.reference(r, t.0 as usize))
}

/// Alignment with the group of every feature: `group_of[file][row]` (`u32::MAX` for the rows left out).
pub struct Grouped {
    pub table: Aligned,
    pub group_of: Vec<Vec<u32>>,
}

/// Entry of the merge heap: smallest m/z first, then smallest file.
struct Head {
    mz: f64,
    file: u32,
    pos: u32,
}

impl PartialEq for Head {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Head {}
impl PartialOrd for Head {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Head {
    fn cmp(&self, o: &Self) -> Ordering {
        o.mz.total_cmp(&self.mz).then(o.file.cmp(&self.file))
    }
}

/// Walk through the rows of a file in m/z order (stable), skipping the artefacts when asked. A table that is already sorted is walked in place.
struct Cursor {
    order: Option<Vec<u32>>,
    at: usize,
}

impl Cursor {
    fn new(f: &FileFeatures) -> Cursor {
        let sorted = f.mz.windows(2).all(|w| w[0] <= w[1]);
        let order = (!sorted).then(|| {
            let mut v: Vec<u32> = (0..f.len() as u32).collect();
            sortc::by(&mut v, |&a, &b| {
                f.mz[a as usize].total_cmp(&f.mz[b as usize])
            });
            v
        });
        Cursor { order, at: 0 }
    }

    fn next(&mut self, f: &FileFeatures, drop: bool) -> Option<u32> {
        while self.at < f.len() {
            let i = self.order.as_ref().map_or(self.at as u32, |o| o[self.at]);
            self.at += 1;
            if !drop || f.flags[i as usize] & FLAG_FT_LOBE == 0 {
                return Some(i);
            }
        }
        None
    }
}

/// Cuts every run of `x` (sorted inside the run) wider than `max_span` at its largest internal gap, repeatedly (at most 32 passes, as Python).
/// `runs` are half-open ranges of `x`; the result keeps their order.
fn limit_span(x: &[f64], runs: Vec<(usize, usize)>, max_span: f64) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(runs.len());
    let mut stack: Vec<(usize, usize, u32)> =
        runs.into_iter().rev().map(|(a, b)| (a, b, 0)).collect();
    while let Some((a, b, depth)) = stack.pop() {
        if depth >= 32 || b - a < 2 || x[b - 1] - x[a] <= max_span {
            out.push((a, b));
            continue;
        }
        let gap = |i: usize| x[i] - x[i - 1];
        let top = (a + 1..b).map(gap).fold(f64::MIN, f64::max);
        if top <= 0.0 {
            out.push((a, b));
            continue;
        }
        let mut cuts: Vec<usize> = (a + 1..b).filter(|&i| gap(i) == top).collect();
        cuts.push(b);
        let mut from = a;
        let mut pieces = Vec::with_capacity(cuts.len());
        for c in cuts {
            pieces.push((from, c, depth + 1));
            from = c;
        }
        stack.extend(pieces.into_iter().rev());
    }
    out
}

/// Groups of one slab (`items` in m/z order): the ranges of the slab sorted by RT, in `items` order after the sort.
fn split_slab(
    files: &[&FileFeatures],
    items: &mut Vec<Item>,
    p: &AlignParams,
    adj: Option<&RtMap>,
) -> Vec<(usize, usize)> {
    let mz = |t: &Item| files[t.0 as usize].mz[t.1 as usize];
    // log-ppm axis, or the m/z itself when the tolerances are in Da
    let unit = p.max_ppm_span < 0.0;
    let x: Vec<f64> = items
        .iter()
        .map(|t| if unit { mz(t) } else { libm::log(mz(t)) * 1e6 })
        .collect();
    let mut runs = limit_span(&x, vec![(0, items.len())], p.max_ppm_span.abs());
    // the RT of every feature of the slab, computed once (the corrected RT interpolates a curve)
    let rts: Vec<f64> = items.iter().map(|t| rt_of(files, adj, t)).collect();
    let mut groups = Vec::new();
    let mut sorted: Vec<(f64, Item)> = Vec::with_capacity(items.len());
    for (a, b) in runs.drain(..) {
        let start = sorted.len();
        sorted.extend((a..b).map(|i| (rts[i], items[i])));
        sortc::by(&mut sorted[start..], |s, t| s.0.total_cmp(&t.0));
        let mut from = start;
        for i in start + 1..=sorted.len() {
            if i == sorted.len() || sorted[i].0 - sorted[i - 1].0 > p.rt_gap {
                groups.push((from, i));
                from = i;
            }
        }
    }
    *items = sorted.iter().map(|x| x.1).collect();
    if let Some(gap) = p.split_gap {
        groups = split_duplicates(files.len(), &sorted, groups, gap);
    }
    match p.max_rt_span {
        None => groups,
        Some(span) => {
            let xr: Vec<f64> = sorted.iter().map(|x| x.0).collect();
            limit_span(&xr, groups, span)
        }
    }
}

/// Cuts the groups where an RT gap above `gap` lies between two features of one file in at least `MIN_SPLIT_FILES` files: two ions (close
/// isomers) that the RT gap of the alignment fused; a double peak seen in one or two files only is not a second ion.
fn split_duplicates(
    nf: usize,
    sorted: &[(f64, Item)],
    groups: Vec<(usize, usize)>,
    gap: f64,
) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(groups.len());
    for (a, b) in groups {
        let mut by_file: Vec<(u32, usize)> = (a..b).map(|i| (sorted[i].1 .0, i)).collect();
        sortc::ord(&mut by_file);
        // one counter per feature of a single group (a group never spans more than the files and the gap allow), not the table
        let mut straddle = vec![0i32; b - a + 1];
        for w in by_file.chunk_by(|x, y| x.0 == y.0) {
            let (lo, hi) = (w[0].1, w[w.len() - 1].1);
            if hi > lo {
                straddle[lo + 1 - a] += 1;
                straddle[hi + 1 - a] -= 1;
            }
        }
        let (mut open, mut cuts) = (0, Vec::new());
        for i in a + 1..b {
            open += straddle[i - a];
            if open >= MIN_SPLIT_FILES && sorted[i].0 - sorted[i - 1].0 > gap {
                cuts.push(i);
            }
        }
        // one cut = two ions; more cuts are a chain of noise, left fused
        if cuts.len() != 1 || b - a > 2 * nf {
            out.push((a, b));
            continue;
        }
        out.push((a, cuts[0]));
        let from = cuts[0];
        out.push((from, b));
    }
    out
}

/// Appends one group to `out` (largest area per file gives apex and limits; weighted m/z and RT).
fn push_group(files: &[&FileFeatures], items: &[Item], out: &mut Aligned, adj: Option<&RtMap>) {
    let nf = out.n_files;
    let base = out.area.len();
    out.area.resize(base + nf, 0.0);
    out.height.resize(base + nf, 0.0);
    out.apex_rt.resize(base + nf, f32::NAN);
    out.width.resize(base + nf, f32::NAN);
    out.state.resize(base + nf, CELL_ABSENT);
    let mut best: Vec<Option<u32>> = vec![None; nf];
    for &(k, i) in items {
        let (f, k) = (files[k as usize], k as usize);
        let i = i as usize;
        if best[k].is_none_or(|b| f.area[i] > f.area[b as usize]) {
            best[k] = Some(i as u32);
        }
        out.area[base + k] = out.area[base + k].max(f.area[i]);
        out.height[base + k] = out.height[base + k].max(f.height[i]);
    }
    let (mut w, mut sm, mut sr) = (0.0f64, 0.0f64, 0.0f64);
    for (k, b) in best.iter().enumerate() {
        let Some(b) = *b else { continue };
        let (f, b) = (files[k], b as usize);
        out.apex_rt[base + k] = f.rt[b];
        out.width[base + k] = f.right[b] - f.left[b];
        out.state[base + k] = CELL_DETECTED;
        w += f.area[b];
        sm += f.mz[b] * f.area[b];
        sr += rt_of(files, adj, &(k as u32, b as u32)) * f.area[b];
    }
    out.mz.push(sm / w);
    out.rt.push(sr / w);
}

/// Aligns the files in "per ion" mode. Cells without a feature have area 0, NaN apex and width, state `CELL_ABSENT`.
pub fn align(files: &[&FileFeatures], p: &AlignParams) -> Aligned {
    align_grouped(files, p, None, false).table
}

/// `align` with the RT of every feature corrected by `adj` (the reference RT of the map) and, when `record`, the group of every feature.
pub fn align_grouped(
    files: &[&FileFeatures],
    p: &AlignParams,
    adj: Option<&RtMap>,
    record: bool,
) -> Grouped {
    let mut n = 0usize;
    merge_groups(files, p, adj, &mut |_| n += 1);
    let mut out = Grouped {
        table: Aligned::with_capacity(files.len(), n),
        group_of: if record {
            files.iter().map(|f| vec![u32::MAX; f.len()]).collect()
        } else {
            Vec::new()
        },
    };
    merge_groups(files, p, adj, &mut |items| {
        if record {
            let g = out.table.len() as u32;
            for &(k, i) in items {
                out.group_of[k as usize][i as usize] = g;
            }
        }
        push_group(files, items, &mut out.table, adj);
    });
    out
}

/// Calls `sink` with the features of every group, in the order of the table (the slabs are made one at a time).
fn merge_groups(
    files: &[&FileFeatures],
    p: &AlignParams,
    adj: Option<&RtMap>,
    sink: &mut dyn FnMut(&[Item]),
) {
    let mut cur: Vec<Cursor> = files.iter().map(|f| Cursor::new(f)).collect();
    let mut heap: BinaryHeap<Head> = BinaryHeap::new();
    let refill = |heap: &mut BinaryHeap<Head>, cur: &mut [Cursor], k: usize| {
        if let Some(i) = cur[k].next(files[k], p.drop_artefacts) {
            heap.push(Head {
                mz: files[k].mz[i as usize],
                file: k as u32,
                pos: i,
            });
        }
    };
    for k in 0..files.len() {
        refill(&mut heap, &mut cur, k);
    }
    let mut slab: Vec<Item> = Vec::new();
    let mut last = f64::NAN;
    while let Some(h) = heap.pop() {
        if !slab.is_empty() && h.mz - last > half(h.mz, p.ppm) {
            flush(files, &mut slab, p, adj, sink);
        }
        last = h.mz;
        slab.push((h.file, h.pos));
        refill(&mut heap, &mut cur, h.file as usize);
    }
    flush(files, &mut slab, p, adj, sink);
}

fn flush(
    files: &[&FileFeatures],
    slab: &mut Vec<Item>,
    p: &AlignParams,
    adj: Option<&RtMap>,
    sink: &mut dyn FnMut(&[Item]),
) {
    if slab.is_empty() {
        return;
    }
    for (a, b) in split_slab(files, slab, p, adj) {
        sink(&slab[a..b]);
    }
    slab.clear();
}
