//! Local-minimum resolver: splits the chromatogram of an m/z trace into peaks.
//!
//! Rewritten from scratch after the algorithm of mzmine 3.9.0 `featdet_chromatogramdeconvolution/minimumsearch/MinimumSearchFeatureResolver.java`
//! (with `AbstractResolver.java`, `FeatureResolverTask.java`, `util/MathUtils.java` and `datamodel/featuredata/FeatureDataUtils.java`; MIT,
//! (c) 2004-2022 The mzmine Development Team); no code copied. Paper: Schmid R. et al., MZmine 3, Nat. Biotechnol. 41:447 (2023),
//! doi:10.1038/s41587-023-01690-2. The reference is the tag `v3.9.0`; mzmine 4.10 also widens the borders of a peak up to the zero next to it
//! (`adjustStartAndEnd`): that is [`MinParams::adjust_edges`], off by default (3.9.0).
//!
//! One function, one step: [`quantile`] (threshold), [`zero_below`], [`has_lower_within`] (is this point a local minimum), [`keep_region`]
//! (shape filter), [`regions`] (the walk along the run), [`make_feature`] (height, area, m/z, apex). Where the text of the study
//! (`docs/studi/ATTRIBUTIO.md` §5.2) is looser than the source, the source wins. A minimum closes a region only if it lies at least `search_rt` from
//! the start of the region, no point within ± `search_rt` is lower, and the apex is at least `min_ratio` times that minimum; the closed region must then
//! pass the shape filter on both edges, otherwise it is dropped and the walk goes on from the minimum.

/// Parameters of the resolver (mzmine names in the comments; values of `tools/bench/parametri/mzmine/predefinita.json`).
#[derive(Clone, Debug, PartialEq)]
pub struct MinParams {
    /// «Chromatographic threshold»: quantile of the intensities of the whole run (zeros included) under which a point becomes 0.
    pub chrom_threshold: f64,
    /// «Minimum search range RT/Mobility (absolute)», min: a minimum must be the lowest point within ± this distance.
    pub search_rt: f64,
    /// «Min ratio of peak top/edge».
    pub min_ratio: f64,
    /// «Minimum absolute height».
    pub min_abs_height: f64,
    /// «Minimum relative height»: fraction of the highest point of the run (after the threshold).
    pub min_rel_height: f64,
    /// «Min # of data points» (the name of 3.9.0; the batch of the benchmark used a name that does not exist).
    pub min_points: usize,
    /// «Peak duration range (min/mobility)», min, both ends included.
    pub duration: (f64, f64),
    /// mzmine 4.10 `adjustStartAndEnd`: an edge point that is not 0 and has a 0 next to it takes that 0 in, so the peak, its area and its range are one
    /// scan wider on that side. Off: the 3.9.0 ranges.
    pub adjust_edges: bool,
}

impl Default for MinParams {
    fn default() -> Self {
        MinParams {
            chrom_threshold: 0.85,
            search_rt: 0.1,
            min_ratio: 1.7,
            min_abs_height: 3e4,
            min_rel_height: 0.0,
            min_points: 5,
            duration: (0.0, 1.0),
            adjust_edges: false,
        }
    }
}

/// The trace on ALL the MS1 scans of the file, in scan order: `y[i]` is the intensity of its point in scan `i` (0 where the trace has none, so a
/// missing scan closes a region) and `mz[i]` its m/z (0 where `y[i]` is 0).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chrom {
    pub mz: Vec<f64>,
    pub y: Vec<f64>,
}

/// A resolved peak. RT in min; `area` in intensity·min (the unit of mzmine) and `area_s` in intensity·s (the rest of mzLab).
#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    /// Mean m/z of the points of the peak, weighted by intensity (`CenterMeasure.AVG`, `Weighting.LINEAR` of `FeatureDataUtils.recalculateIonSeriesDependingTypes`).
    pub mz: f64,
    /// RT of the most intense point (the first one if several are equal).
    pub rt_apex: f64,
    pub rt_start: f64,
    pub rt_end: f64,
    pub height: f64,
    pub area: f64,
    pub area_s: f64,
    /// First and last scan of the peak, both included.
    pub scans: (usize, usize),
}

/// Quantile as `MathUtils.calcQuantile`: sort, then the mean of the values at `floor((n-1)·q)` and `ceil((n-1)·q)` (no other interpolation);
/// `q` is clamped to [0, 1], an empty vector gives 0 and a single value gives itself.
pub fn quantile(values: &[f64], q: f64) -> f64 {
    match values.len() {
        0 => return 0.0,
        1 => return values[0],
        _ => {}
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let pos = (v.len() - 1) as f64 * q.clamp(0.0, 1.0);
    (v[pos.floor() as usize] + v[pos.ceil() as usize]) / 2.0
}

/// Puts to 0 every point under the threshold (strictly) and returns the highest value left.
fn zero_below(y: &mut [f64], threshold: f64) -> f64 {
    let mut top = 0.0f64;
    for v in y.iter_mut() {
        if *v < threshold {
            *v = 0.0;
        }
        top = top.max(*v);
    }
    top
}

/// True if a point strictly lower than `y[k]` lies within ± `width` of `x[k]`, so `k` is not a local minimum. As the source, the walk to the left stops
/// before index 0 (index 0 is never looked at) and each walk stops at the first point out of the window.
fn has_lower_within(x: &[f64], y: &[f64], k: usize, width: f64) -> bool {
    let (lo, hi) = (x[k] - width, x[k] + width);
    let inside = |i: usize| x[i] >= lo && x[i] <= hi;
    for i in (1..k).rev() {
        if !inside(i) {
            break;
        }
        if y[i] < y[k] {
            return true;
        }
    }
    for i in k + 1..x.len() {
        if !inside(i) {
            break;
        }
        if y[i] < y[k] {
            return true;
        }
    }
    false
}

/// Shape filter of a region `[s, e]` of height `top`: enough points, tall enough, the apex at least `min_ratio` times each edge, duration in range.
fn keep_region(
    x: &[f64],
    y: &[f64],
    s: usize,
    e: usize,
    top: f64,
    min_height: f64,
    p: &MinParams,
) -> bool {
    let long_enough = e - s + 1 >= p.min_points;
    let dur = x[e] - x[s];
    long_enough
        && top >= min_height
        && top >= y[s] * p.min_ratio
        && top >= y[e] * p.min_ratio
        && dur >= p.duration.0
        && dur <= p.duration.1
}

/// The walk of the source: regions run from zero to zero (a single zero closes one); inside a region, a point at least `search_rt` away from the start
/// that is the lowest within ± `search_rt` closes it too when the apex is at least `min_ratio` times that point. `y` is thresholded in place.
/// Returns the inclusive scan ranges of the peaks that pass [`keep_region`]. The loop bound `start < last - 2` is the source's: the last two
/// scans never open a region.
pub fn regions(x: &[f64], y: &mut [f64], p: &MinParams) -> Vec<(usize, usize)> {
    let n = x.len();
    let mut out = Vec::new();
    if n == 0 || y.len() != n {
        return out;
    }
    let top_all = zero_below(y, quantile(y, p.chrom_threshold));
    let min_height = p.min_abs_height.max(p.min_rel_height * top_all);
    let last = n - 1;
    let mut start = 0usize;
    'region: while (start as isize) < last as isize - 2 {
        if y[start] == 0.0 || y[start + 1] == 0.0 {
            start += 1;
            continue;
        }
        let mut top = y[start];
        for end in start + 1..n {
            top = top.max(y[end]);
            let closes_on_zero = end == last || y[end + 1] == 0.0;
            let closes_on_minimum = !closes_on_zero
                && x[end] - x[start] >= p.search_rt
                && !has_lower_within(x, y, end, p.search_rt)
                && top >= y[end] * p.min_ratio;
            if closes_on_zero || closes_on_minimum {
                if keep_region(x, y, start, end, top, min_height, p) {
                    out.push(if p.adjust_edges {
                        widen(y, start, end)
                    } else {
                        (start, end)
                    });
                }
                start = end; // the source sets `end - 1` and the `for` adds 1
                continue 'region;
            }
        }
        break; // unreachable: `end == last` always closes
    }
    out
}

/// `adjustStartAndEnd` of 4.10: a non-zero edge with a zero next to it takes the zero in (`y` is the thresholded trace of the walk).
fn widen(y: &[f64], s: usize, e: usize) -> (usize, usize) {
    let s2 = if s > 0 && y[s] != 0.0 && y[s - 1] == 0.0 {
        s - 1
    } else {
        s
    };
    let e2 = if e + 1 < y.len() && y[e] != 0.0 && y[e + 1] == 0.0 {
        e + 1
    } else {
        e
    };
    (s2, e2)
}

/// Height, area (trapezoids over the RT axis, in intensity·min), apex and m/z of the points `s..=e` of the original trace.
fn make_feature(rt: &[f64], c: &Chrom, s: usize, e: usize) -> Feature {
    let (mut apex, mut sum_w, mut sum_mw) = (s, 0.0, 0.0);
    for i in s..=e {
        if c.y[i] > c.y[apex] {
            apex = i;
        }
        sum_w += c.y[i];
        sum_mw += c.mz[i] * c.y[i];
    }
    let area: f64 = (s + 1..=e)
        .map(|i| (rt[i] - rt[i - 1]) * (c.y[i] + c.y[i - 1]) / 2.0)
        .sum();
    // all weights 0 cannot happen inside a region; as the source, fall back on the plain mean
    let mz = if sum_w == 0.0 {
        c.mz[s..=e].iter().sum::<f64>() / (e - s + 1) as f64
    } else {
        sum_mw / sum_w
    };
    Feature {
        mz,
        rt_apex: rt[apex],
        rt_start: rt[s],
        rt_end: rt[e],
        height: c.y[apex],
        area,
        area_s: area * 60.0,
        scans: (s, e),
    }
}

/// Resolves one trace: `c` covers the whole run (zeros included), `rt` is the RT of each scan in min, strictly increasing. A trace whose `y` and
/// `rt` differ in length gives nothing. The input is not modified (the threshold works on a copy; the peaks keep their original intensities).
pub fn resolve(c: &Chrom, rt: &[f64], p: &MinParams) -> Vec<Feature> {
    if c.y.len() != rt.len() || c.mz.len() != rt.len() {
        return Vec::new();
    }
    let mut y = c.y.clone();
    regions(rt, &mut y, p)
        .into_iter()
        .map(|(s, e)| make_feature(rt, c, s, e))
        .collect()
}
