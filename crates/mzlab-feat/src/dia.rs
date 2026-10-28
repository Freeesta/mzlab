//! Pseudo MS2 spectra from a DIA run (SWATH, MSE): for every precursor, the fragments whose ion trace follows the trace of the precursor.
//! Pure functions on slices (no files, no threads, sums in a fixed order); used by `spectra.ms2` of the CLI with `dia_pseudo`. `docs/agenti/dia.md`.

/// One scan of the run. `iso` is the isolation window `(low, high)` of an MS2; `None` on an MS2 means all ions (MSE). Peaks sorted by m/z.
pub struct DiaScan<'a> {
    pub rt_s: f64,
    pub level: u8,
    pub iso: Option<(f64, f64)>,
    pub mz: &'a [f64],
    pub int: &'a [f64],
}

/// A precursor and the RT range (seconds) of its peak.
pub struct Prec {
    pub id: String,
    pub mz: f64,
    pub rt_lo_s: f64,
    pub rt_hi_s: f64,
}

pub struct DiaParams {
    /// Least Pearson correlation between the trace of a fragment and that of the precursor.
    pub min_corr: f64,
    /// Fewest cycles of the RT range.
    pub min_points: usize,
    /// Largest distance between the apexes, in cycles.
    pub max_shift: usize,
    pub frag_ppm: f64,
    /// Candidate fragments: peaks of the scan at the apex above this fraction of its base peak.
    pub min_rel: f64,
}

impl Default for DiaParams {
    fn default() -> Self {
        DiaParams {
            min_corr: 0.9,
            min_points: 5,
            max_shift: 1,
            frag_ppm: 10.0,
            min_rel: 0.01,
        }
    }
}

/// The pseudo spectrum of a precursor: `peaks` are `(m/z, intensity at the apex, correlation)` in m/z order.
#[derive(Debug, Clone, PartialEq)]
pub struct Pseudo {
    pub id: String,
    pub apex_rt_s: f64,
    pub peaks: Vec<(f64, f64, f64)>,
}

/// Highest intensity of the scan within `ppm` of `mz` (0 when none).
fn max_near(s: &DiaScan, mz: f64, ppm: f64) -> f64 {
    let tol = mz * ppm * 1e-6;
    let from = s.mz.partition_point(|m| *m < mz - tol);
    let mut best = 0.0f64;
    for k in from..s.mz.len() {
        if s.mz[k] > mz + tol {
            break;
        }
        best = best.max(s.int[k]);
    }
    best
}

fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        let (dx, dy) = (x - ma, y - mb);
        sab += dx * dy;
        saa += dx * dx;
        sbb += dy * dy;
    }
    if saa <= 0.0 || sbb <= 0.0 {
        return 0.0;
    }
    sab / (saa * sbb).sqrt()
}

/// Index of the first maximum.
fn apex(v: &[f64]) -> usize {
    let mut at = 0;
    for (k, x) in v.iter().enumerate() {
        if *x > v[at] {
            at = k;
        }
    }
    at
}

/// The window of the MS2 scans that holds `mz`: the one whose centre is the closest (a scan without window holds every ion, last choice).
fn window_of(scans: &[DiaScan], mz: f64) -> Option<Option<(f64, f64)>> {
    let mut best: Option<(f64, Option<(f64, f64)>)> = None;
    for s in scans.iter().filter(|s| s.level == 2) {
        let dist = match s.iso {
            Some((lo, hi)) if lo <= mz && mz <= hi => ((lo + hi) / 2.0 - mz).abs(),
            Some(_) => continue,
            None => f64::MAX,
        };
        if best.is_none_or(|b| dist < b.0) {
            best = Some((dist, s.iso));
        }
    }
    best.map(|b| b.1)
}

/// The trace of the precursor in the MS1 scans, interpolated linearly to the times of the cycles.
fn precursor_trace(scans: &[DiaScan], mz: f64, ppm: f64, times: &[f64]) -> Vec<f64> {
    let mut ms1: Vec<(f64, f64)> = scans
        .iter()
        .filter(|s| s.level == 1)
        .map(|s| (s.rt_s, max_near(s, mz, ppm)))
        .collect();
    ms1.sort_by(|a, b| a.0.total_cmp(&b.0));
    times
        .iter()
        .map(|t| {
            let at = ms1.partition_point(|p| p.0 < *t);
            match (at.checked_sub(1).map(|k| ms1[k]), ms1.get(at)) {
                (Some(a), Some(b)) if b.0 > a.0 => a.1 + (b.1 - a.1) * (t - a.0) / (b.0 - a.0),
                (Some(a), None) => a.1,
                (_, Some(b)) => b.1,
                (None, None) => 0.0,
            }
        })
        .collect()
}

fn one(scans: &[DiaScan], prec: &Prec, p: &DiaParams) -> Option<Pseudo> {
    let win = window_of(scans, prec.mz)?;
    let mut cycles: Vec<&DiaScan> = scans
        .iter()
        .filter(|s| s.level == 2 && s.iso == win)
        .filter(|s| s.rt_s >= prec.rt_lo_s && s.rt_s <= prec.rt_hi_s)
        .collect();
    if cycles.len() < p.min_points.max(2) {
        return None;
    }
    cycles.sort_by(|a, b| a.rt_s.total_cmp(&b.rt_s));
    let times: Vec<f64> = cycles.iter().map(|s| s.rt_s).collect();
    let xic = precursor_trace(scans, prec.mz, p.frag_ppm, &times);
    let top = apex(&xic);
    if xic[top] <= 0.0 {
        return None;
    }
    let at = cycles[top];
    let base = at.int.iter().fold(0.0f64, |m, v| m.max(*v));
    let mut peaks = Vec::new();
    for (k, mz) in at.mz.iter().enumerate() {
        if at.int[k] <= 0.0 || at.int[k] < p.min_rel * base {
            continue;
        }
        let trace: Vec<f64> = cycles
            .iter()
            .map(|s| max_near(s, *mz, p.frag_ppm))
            .collect();
        let corr = pearson(&trace, &xic);
        if corr >= p.min_corr && apex(&trace).abs_diff(top) <= p.max_shift {
            peaks.push((*mz, trace[top], corr));
        }
    }
    if peaks.is_empty() {
        return None;
    }
    Some(Pseudo {
        id: prec.id.clone(),
        apex_rt_s: times[top],
        peaks,
    })
}

/// The pseudo spectra of the precursors that have one, in the order of `precs`. A precursor with fewer than `min_points` cycles, no trace in the
/// MS1 scans or no correlated fragment has none. Isomers that elute together share their fragments: the traces cannot tell them apart.
pub fn pseudo_ms2(scans: &[DiaScan], precs: &[Prec], p: &DiaParams) -> Vec<Pseudo> {
    precs
        .iter()
        .filter_map(|prec| one(scans, prec, p))
        .collect()
}
