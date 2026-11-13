//! Background of a TIC/BPC by ion: the m/z bins present in most scans (chemical background: solvent, plasticisers, clusters) are found,
//! and each one's slow time baseline (median of block medians, a local robust baseline) is subtracted from the trace, so a real peak that
//! shares an m/z with a background line keeps its signal. Other modes: a quiet RT window or a blank sample give the baseline; manual m/z
//! windows and `Action::Exclude` drop whole ions. Pure functions on peaks streamed in m/z order (the order of the index); `ticbg_route` reads the index.
use crate::signal::median as median_of;

/// Where the baseline of a background ion comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Rolling baseline of the ion over the run (default).
    Auto,
    /// The median of the ion in the RT window `quiet` (constant).
    Quiet,
    /// The median of the ion in a blank sample (constant); the blank also names the ions.
    Blank,
}

/// What happens to a background ion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// Its baseline is subtracted, its peaks stay (default).
    Subtract,
    /// The whole ion leaves the trace.
    Exclude,
}

/// Knobs; `Default` is the automatic removal. Thresholds and their sources: `docs/agenti/tic-fondo.md`.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    pub mode: Mode,
    pub action: Action,
    /// Fraction of the scans an ion must be in to be background, default 0.8.
    pub persist: f64,
    /// Bin width at unit resolution (Da), default 1.
    pub bin_da: f64,
    /// Bin width at high resolution (ppm of the lower edge of the m/z octave), default 10.
    pub bin_ppm: f64,
    /// High-resolution binning (from the file unless the request says).
    pub hr: bool,
    /// Block of the baseline as a fraction of the scans, default 0.01 (at least 3 scans).
    pub block: f64,
    /// Blocks in the median window of the baseline (odd), default 15: a peak narrower than about a third of it stays.
    pub window: usize,
    /// RT window (min) of `Mode::Quiet`.
    pub quiet: Option<(f64, f64)>,
    /// Manual exclusion windows (m/z, ± tolerance in Da).
    pub windows: Vec<(f64, f64)>,
    /// m/z of detected ions the user put back (left raw).
    pub keep: Vec<f64>,
    /// `Mode::Auto`: after the ions, the rolling baseline of the TIC itself goes too: the floor of the sparse chemical noise no single ion explains. Default on.
    pub floor: bool,
}

impl Default for Params {
    fn default() -> Self {
        Params { mode: Mode::Auto, action: Action::Subtract, persist: 0.8, bin_da: 1.0, bin_ppm: 10.0, hr: false, block: 0.01, window: 15, quiet: None, windows: Vec::new(), keep: Vec::new(), floor: true }
    }
}

impl Params {
    /// The bin of an m/z. Unit resolution: nearest multiple of `bin_da`. High resolution: `bin_ppm` of the lower power of two of the m/z,
    /// so the width is 1-2x `bin_ppm` and the bin needs no logarithm (same bits in the `.wasm` and native).
    pub fn bin(&self, mz: f64) -> i64 {
        if !self.hr {
            return (mz / self.bin_da).round() as i64;
        }
        let e = ((mz.to_bits() >> 52) & 0x7ff) as i64;
        let w = f64::from_bits(((e as u64) << 52) & 0x7ff0_0000_0000_0000) * self.bin_ppm * 1e-6;
        (e << 40) + (mz / w).floor() as i64
    }
    fn dropped(&self, mz: f64) -> bool {
        self.windows.iter().any(|&(c, t)| (mz - c).abs() <= t)
    }
    fn kept(&self, lo: f64, hi: f64) -> bool {
        let pad = if self.hr { 0.0 } else { self.bin_da / 2.0 };
        self.keep.iter().any(|&m| m >= lo - pad - 1e-9 && m <= hi + pad + 1e-9)
    }
}

/// The blank of `Mode::Blank`: per bin, its median over the blank's scans (only bins with `persist`).
pub type BlankProfile = Vec<(i64, f64)>;

/// One background ion (adjacent bins merged), with the measures behind the choice.
#[derive(Clone, Debug, PartialEq)]
pub struct Ion {
    /// Intensity-weighted m/z and the m/z span of its peaks.
    pub mz: f64,
    pub lo: f64,
    pub hi: f64,
    /// Fraction of scans with the ion.
    pub persistence: f64,
    /// Robust coefficient of variation (1.4826 MAD / median) where present.
    pub cv: f64,
    /// Median of its baseline (intensity).
    pub level: f64,
    /// Its removed part over the raw TIC (sum over the run).
    pub share: f64,
    /// persistence / (1 + cv), 0..1.
    pub confidence: f64,
}

/// The cleaned traces and what was removed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cleaned {
    pub tic: Vec<f64>,
    pub bpc: Vec<f64>,
    pub ions: Vec<Ion>,
    /// Removed signal over the raw TIC (sum over the run), all ions and windows.
    pub removed: f64,
    /// The part of `removed` from the manual windows.
    pub windows: f64,
    /// The part of `removed` from the floor of the TIC.
    pub floor: f64,
}

/// Running state of one bin while its peaks stream by.
struct Bin {
    id: i64,
    pts: Vec<(usize, f64, f64)>,
}

/// Per bin: the number of distinct scans it is in (first pass). `peaks` streams (scan, m/z, intensity) in m/z order.
#[inline(never)]
pub fn presence(n: usize, p: &Params, peaks: &mut dyn FnMut(&mut dyn FnMut(usize, f64, f64))) -> Vec<(i64, u32)> {
    let mut seen = vec![i64::MIN; n];
    let mut out: Vec<(i64, u32)> = Vec::new();
    bins(n, p, peaks, &mut |b| {
        let c = b.pts.iter().filter(|q| std::mem::replace(&mut seen[q.0], b.id) != b.id).count();
        out.push((b.id, c as u32));
    });
    out
}

/// The peaks grouped by bin, one bin at a time in m/z order; peaks in the manual windows are left out. Returns (raw total, windows total).
#[inline(never)]
fn bins(n: usize, p: &Params, peaks: &mut dyn FnMut(&mut dyn FnMut(usize, f64, f64)), f: &mut dyn FnMut(Bin)) -> (f64, f64) {
    let mut cur = Bin { id: i64::MIN, pts: Vec::new() };
    let (mut raw, mut win) = (0.0, 0.0);
    peaks(&mut |s, mz, y| {
        if s >= n {
            return;
        }
        raw += y;
        if p.dropped(mz) {
            win += y;
            return;
        }
        let b = p.bin(mz);
        if cur.id != b {
            let done = std::mem::replace(&mut cur, Bin { id: b, pts: Vec::new() });
            if !done.pts.is_empty() {
                f(done);
            }
        }
        cur.pts.push((s, mz, y));
    });
    if !cur.pts.is_empty() {
        f(cur);
    }
    (raw, win)
}

/// The bins treated as background: persistence over `persist` (high resolution: the bin and its two neighbours together, a line that
/// straddles a bin edge counts whole), or the bins of the blank; minus the ions the user kept.
#[inline(never)]
pub fn flagged(n: usize, p: &Params, pres: &[(i64, u32)], blank: Option<&BlankProfile>) -> Vec<i64> {
    if let Some(b) = blank {
        return b.iter().map(|x| x.0).collect();
    }
    let need = p.persist * n as f64;
    let count = |b: i64| pres.binary_search_by_key(&b, |x| x.0).map_or(0, |i| pres[i].1);
    pres.iter()
        .filter(|&&(b, c)| {
            let c = if p.hr { count(b - 1) + c + count(b + 1) } else { c };
            n > 0 && c as f64 >= need
        })
        .map(|x| x.0)
        .collect()
}

/// Baseline of one ion trace: medians of blocks of `block` scans, a running median of `window` blocks, linear between block centres;
/// then once more without the points above it by more than 3 robust sigma (the peaks), as sigma clipping does.
#[inline(never)]
pub fn rolling_baseline(x: &[f64], p: &Params) -> Vec<f64> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let m = ((n as f64 * p.block).round() as usize).max(3).min(n);
    let first = block_baseline(x, None, m, p.window.max(1) / 2);
    let r: Vec<f64> = x.iter().zip(&first).map(|(a, b)| a - b).collect();
    let rm = median_of(&r);
    let sigma = 1.4826 * median_of(&r.iter().map(|v| (v - rm).abs()).collect::<Vec<_>>());
    let peak: Vec<bool> = r.iter().map(|v| *v - rm > 3.0 * sigma && *v > 0.0).collect();
    block_baseline(x, Some(&peak), m, p.window.max(1) / 2)
}

#[inline(never)]
fn block_baseline(x: &[f64], masked: Option<&[bool]>, m: usize, h: usize) -> Vec<f64> {
    let n = x.len();
    let meds: Vec<Option<f64>> = (0..n.div_ceil(m))
        .map(|k| {
            let v: Vec<f64> = (k * m..((k + 1) * m).min(n)).filter(|&j| !masked.is_some_and(|ms| ms[j])).map(|j| x[j]).collect();
            (!v.is_empty()).then(|| median_of(&v))
        })
        .collect();
    let all: Vec<f64> = meds.iter().flatten().copied().collect();
    let base: Vec<f64> = (0..meds.len())
        .map(|k| {
            let w: Vec<f64> = meds[k.saturating_sub(h)..(k + h + 1).min(meds.len())].iter().flatten().copied().collect();
            median_of(if w.is_empty() { &all } else { &w })
        })
        .collect();
    let centre = |k: usize| (k * m) as f64 + ((m.min(n - k * m)) as f64 - 1.0) / 2.0;
    let mut out = Vec::with_capacity(n);
    let mut k = 0;
    for j in 0..n {
        let t = j as f64;
        while k + 1 < base.len() && centre(k + 1) <= t {
            k += 1;
        }
        let v = if k + 1 >= base.len() || t <= centre(k) {
            base[k]
        } else {
            let (c0, c1) = (centre(k), centre(k + 1));
            base[k] + (base[k + 1] - base[k]) * (t - c0) / (c1 - c0)
        };
        out.push(v);
    }
    out
}

/// The cleaned TIC and BPC (second pass). `rt` gives the scans (the quiet window); `raw_total` is the sum of the raw TIC.
#[inline(never)]
pub fn clean(rt: &[f64], p: &Params, flags: &[i64], blank: Option<&BlankProfile>, peaks: &mut dyn FnMut(&mut dyn FnMut(usize, f64, f64))) -> Cleaned {
    let n = rt.len();
    let mut out = Cleaned { tic: vec![0.0; n], bpc: vec![0.0; n], ..Default::default() };
    let mut parts: Vec<(i64, Ion, f64)> = Vec::new();
    let (raw_total, win) = bins(n, p, peaks, &mut |b| finish(b, rt, p, flags, blank, &mut out, &mut parts));
    let share = |v: f64| if raw_total > 0.0 { v / raw_total } else { 0.0 };
    out.ions = merge(parts, raw_total);
    out.windows = share(win);
    if p.floor && p.mode == Mode::Auto {
        let f = rolling_baseline(&out.tic, p);
        out.tic.iter_mut().zip(&f).for_each(|(y, b)| *y -= b);
        out.floor = share(f.iter().sum());
    }
    out.removed = out.windows + out.floor + out.ions.iter().map(|i| i.share).sum::<f64>();
    // largest share first; equal shares keep the m/z order (a stable sort)
    let order = crate::specpeaks::order_by_f64(&out.ions.iter().map(|i| i.share).collect::<Vec<_>>(), true);
    out.ions = order.into_iter().map(|k| out.ions[k].clone()).collect();
    out
}

/// One bin done: its trace, its baseline, its part of the cleaned TIC and BPC, and its measures when it is background.
#[inline(never)]
fn finish(b: Bin, rt: &[f64], p: &Params, flags: &[i64], blank: Option<&BlankProfile>, out: &mut Cleaned, parts: &mut Vec<(i64, Ion, f64)>) {
    let n = rt.len();
    let mut x = vec![0.0; n];
    let (mut sw, mut swm, mut lo, mut hi) = (0.0, 0.0, f64::INFINITY, f64::NEG_INFINITY);
    for &(s, mz, y) in &b.pts {
        x[s] += y;
        sw += y;
        swm += y * mz;
        lo = lo.min(mz);
        hi = hi.max(mz);
    }
    let bg = flags.binary_search(&b.id).is_ok() && !p.kept(lo, hi);
    let base = if !bg {
        vec![0.0; n]
    } else if p.action == Action::Exclude {
        x.clone()
    } else {
        match p.mode {
            Mode::Auto => rolling_baseline(&x, p),
            Mode::Quiet => {
                let (a, z) = p.quiet.unwrap_or((f64::NEG_INFINITY, f64::INFINITY));
                let q: Vec<f64> = x.iter().zip(rt).filter(|(_, t)| **t >= a && **t <= z).map(|(v, _)| *v).collect();
                vec![if q.is_empty() { 0.0 } else { median_of(&q) }; n]
            }
            Mode::Blank => {
                let v = blank.and_then(|bl| bl.binary_search_by_key(&b.id, |x| x.0).ok().map(|i| bl[i].1)).unwrap_or(0.0);
                vec![v; n]
            }
        }
    };
    let mut removed = 0.0;
    for j in 0..n {
        let v = x[j] - base[j];
        out.tic[j] += v;
        removed += base[j];
        if v > out.bpc[j] {
            out.bpc[j] = v;
        }
    }
    if bg {
        let present: Vec<f64> = x.iter().copied().filter(|v| *v > 0.0).collect();
        let med = if present.is_empty() { 0.0 } else { median_of(&present) };
        let mad = if present.is_empty() { 0.0 } else { median_of(&present.iter().map(|v| (v - med).abs()).collect::<Vec<_>>()) };
        let cv = if med > 0.0 { 1.4826 * mad / med } else { 0.0 };
        let persistence = present.len() as f64 / n.max(1) as f64;
        let ion = Ion { mz: if sw > 0.0 { swm / sw } else { (lo + hi) / 2.0 }, lo, hi, persistence, cv, level: median_of(&base), share: 0.0, confidence: persistence / (1.0 + cv) };
        parts.push((b.id, ion, removed));
    }
}

/// Adjacent bins become one ion (a high-resolution line split by a bin edge): weighted m/z, widest span, highest persistence, summed share.
#[inline(never)]
fn merge(parts: Vec<(i64, Ion, f64)>, total: f64) -> Vec<Ion> {
    let mut ions: Vec<(i64, Ion, f64, f64)> = Vec::new();
    for (b, ion, removed) in parts {
        let w = removed.max(1e-300);
        match ions.last_mut() {
            Some(last) if last.0 + 1 == b => {
                let (l, wl) = (&mut last.1, last.3);
                l.mz = (l.mz * wl + ion.mz * w) / (wl + w);
                l.lo = l.lo.min(ion.lo);
                l.hi = l.hi.max(ion.hi);
                if ion.persistence > l.persistence {
                    l.persistence = ion.persistence;
                    l.cv = ion.cv;
                    l.confidence = ion.confidence;
                }
                l.level += ion.level;
                last.0 = b;
                last.2 += removed;
                last.3 += w;
            }
            _ => ions.push((b, ion, removed, w)),
        }
    }
    ions.into_iter()
        .map(|(_, mut i, removed, _)| {
            i.share = if total > 0.0 { removed / total } else { 0.0 };
            i
        })
        .collect()
}

/// The blank profile of `Mode::Blank` from the blank's peaks: median per bin of the bins present in `persist` of its scans.
#[inline(never)]
pub fn blank_profile(n: usize, p: &Params, peaks: &mut dyn FnMut(&mut dyn FnMut(usize, f64, f64))) -> BlankProfile {
    let pres = presence(n, p, peaks);
    let keep: Vec<i64> = flagged(n, &Params { hr: p.hr, persist: p.persist, ..Params::default() }, &pres, None);
    let mut out: BlankProfile = Vec::new();
    bins(n, p, peaks, &mut |b| {
        if keep.binary_search(&b.id).is_ok() {
            let mut x = vec![0.0; n];
            b.pts.iter().for_each(|q| x[q.0] += q.2);
            out.push((b.id, median_of(&x)));
        }
    });
    out
}
