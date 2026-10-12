//! Alignment of the features of a series of files in groups ("per ion" mode): port of `TP_Mine/py/tpmine/hr/features.py::align`.
//! The tables (sorted by m/z) are merged k ways in file order, which is the stable sort of the concatenation of Python. A chain of
//! features whose consecutive m/z gaps are below `ppm` is one slab (no group crosses its edges): it is cut where it spans more than
//! `max_ppm_span`, sorted by RT and cut where the RT gap is above `rt_gap`; so one slab is held at a time.

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
}

impl Default for AlignParams {
    fn default() -> Self {
        AlignParams {
            ppm: 5.0,
            rt_gap: 0.1,
            max_ppm_span: 10.0,
            max_rt_span: None,
            drop_artefacts: true,
        }
    }
}

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

/// Rows of each file in m/z order (stable), without the artefacts when asked.
fn order(f: &FileFeatures, drop: bool) -> Vec<u32> {
    let mut v: Vec<u32> = (0..f.len() as u32)
        .filter(|&i| !drop || f.flags[i as usize] & FLAG_FT_LOBE == 0)
        .collect();
    if !f.mz.windows(2).all(|w| w[0] <= w[1]) {
        v.sort_by(|&a, &b| f.mz[a as usize].total_cmp(&f.mz[b as usize]));
    }
    v
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
    let rt = |t: &Item| rt_of(files, adj, t);
    let x: Vec<f64> = items.iter().map(|t| libm::log(mz(t)) * 1e6).collect();
    let mut runs = limit_span(&x, vec![(0, items.len())], p.max_ppm_span);
    let mut groups = Vec::new();
    let mut sorted: Vec<Item> = Vec::with_capacity(items.len());
    for (a, b) in runs.drain(..) {
        let start = sorted.len();
        sorted.extend_from_slice(&items[a..b]);
        sorted[start..].sort_by(|s, t| rt(s).total_cmp(&rt(t)));
        let mut from = start;
        for i in start + 1..=sorted.len() {
            if i == sorted.len() || rt(&sorted[i]) - rt(&sorted[i - 1]) > p.rt_gap {
                groups.push((from, i));
                from = i;
            }
        }
    }
    *items = sorted;
    match p.max_rt_span {
        None => groups,
        Some(span) => {
            let xr: Vec<f64> = items.iter().map(rt).collect();
            limit_span(&xr, groups, span)
        }
    }
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
    let mut out = Grouped {
        table: Aligned {
            n_files: files.len(),
            ..Aligned::default()
        },
        group_of: if record {
            files.iter().map(|f| vec![u32::MAX; f.len()]).collect()
        } else {
            Vec::new()
        },
    };
    let ord: Vec<Vec<u32>> = files.iter().map(|f| order(f, p.drop_artefacts)).collect();
    let mut at = vec![0usize; files.len()];
    let mut heap: BinaryHeap<Head> = BinaryHeap::new();
    let push = |heap: &mut BinaryHeap<Head>, k: usize, at: &[usize]| {
        if let Some(&i) = ord[k].get(at[k]) {
            heap.push(Head {
                mz: files[k].mz[i as usize],
                file: k as u32,
                pos: i,
            });
        }
    };
    for k in 0..files.len() {
        push(&mut heap, k, &at);
    }
    let mut slab: Vec<Item> = Vec::new();
    let mut last = f64::NAN;
    while let Some(h) = heap.pop() {
        if !slab.is_empty() && h.mz - last > h.mz * p.ppm * 1e-6 {
            flush(files, &mut slab, p, adj, &mut out);
        }
        last = h.mz;
        slab.push((h.file, h.pos));
        at[h.file as usize] += 1;
        push(&mut heap, h.file as usize, &at);
    }
    flush(files, &mut slab, p, adj, &mut out);
    out
}

fn flush(
    files: &[&FileFeatures],
    slab: &mut Vec<Item>,
    p: &AlignParams,
    adj: Option<&RtMap>,
    out: &mut Grouped,
) {
    if slab.is_empty() {
        return;
    }
    for (a, b) in split_slab(files, slab, p, adj) {
        if !out.group_of.is_empty() {
            let g = out.table.len() as u32;
            for &(k, i) in &slab[a..b] {
                out.group_of[k as usize][i as usize] = g;
            }
        }
        push_group(files, &slab[a..b], &mut out.table, adj);
    }
    slab.clear();
}
