//! Peaks that hide two ions: a trace with several maxima is cut at its valleys.
//!
//! A feature is looked at again when its trace is long (more scans than `Params::long_trace`) or its shape is poor (below
//! `ResolveParams::max_shape`): the trace within `window_ppm` is searched for maxima with `peakq::detect_prominence`. Two neighbouring maxima
//! are two ions when the valley between them is lower than `valley_frac` of the lower maximum and their apexes are at least `min_sep` minutes
//! apart; the feature becomes one row per ion (`FLAG_RESOLVED`; area, height, edges and quality come from the trace of the piece). Maxima
//! that are not separated stay one feature; `mark_mixtures` sets `FLAG_MIXTURE` on it when the MS2 spectra before and after the deepest valley
//! differ (cosine of the square-rooted spectra binned at `bin` m/z below `mix_cosine`). Runs after `quality::assess` and before the
//! isotope groups and the MS2 links, which refer to the rows of the new table.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::peakq::{detect_prominence, ProminenceParams};

use crate::ms2::Ms2Links;
use crate::quality::{around_apex, assess_with, fill_row, QParams, AREA_HALF_WINDOW};
use crate::table::{FileFeatures, FLAG_MIXTURE, FLAG_RESOLVED};
use crate::trace::Params;

#[derive(Debug, Clone)]
pub struct ResolveParams {
    pub window_ppm: f64,
    pub pad: usize,
    /// Features with a shape correlation below this are searched for several maxima (long traces always).
    pub max_shape: f64,
    /// The valley must be lower than this fraction of the lower of the two maxima.
    pub valley_frac: f64,
    /// Minimum distance between the apexes of two ions, minutes.
    pub min_sep: f64,
    pub prom: ProminenceParams,
    /// Width of the m/z bins of the MS2 comparison.
    pub bin: f64,
    pub mix_cosine: f64,
}

impl Default for ResolveParams {
    fn default() -> Self {
        ResolveParams {
            window_ppm: 6.0,
            pad: 15,
            max_shape: 0.9,
            valley_frac: 0.5,
            min_sep: 0.1,
            prom: ProminenceParams::default(),
            bin: 0.05,
            mix_cosine: 0.7,
        }
    }
}

/// A feature with several maxima that were not separated: its deepest valley, minutes.
#[derive(Debug, Clone, PartialEq)]
pub struct Multi {
    pub row: usize,
    pub valley_rt: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Resolved {
    /// Features that were cut and the rows they became.
    pub split: usize,
    pub pieces: usize,
    pub multi: Vec<Multi>,
}

/// Cheap necessary condition of a split, on the trace between the edges of a feature smoothed with three points: a point lower than 0.9 times
/// the lower of the highest points on both sides, the lower of which is at least a twentieth of the highest of the trace.
fn may_split(y: &[f64]) -> bool {
    let n = y.len();
    if n < 7 {
        return false;
    }
    let sm: Vec<f64> = (0..n)
        .map(|i| (y[i.saturating_sub(1)] + y[i] + y[(i + 1).min(n - 1)]) / 3.0)
        .collect();
    let top = sm.iter().fold(0.0f64, |m, &v| m.max(v));
    let mut right = vec![0.0f64; n];
    for i in (0..n).rev() {
        right[i] = sm[i].max(if i + 1 < n { right[i + 1] } else { 0.0 });
    }
    let mut left = 0.0f64;
    for i in 0..n {
        left = left.max(sm[i]);
        let lower = left.min(right[i]);
        if lower >= 0.05 * top && sm[i] < 0.9 * lower {
            return true;
        }
    }
    false
}

/// Pieces and unseparated maxima found while the traces go by.
#[derive(Default)]
struct Found {
    pieces: Vec<(usize, FileFeatures)>,
    multi: Vec<(usize, usize)>,
}

/// Look at row `i` (finished, with its trace `y` from scan `w0`): if it hides several ions, the pieces or the deepest valley go to `out`.
#[allow(clippy::too_many_arguments)]
fn look(
    f: &FileFeatures,
    i: usize,
    rt: &[f64],
    y: &[f64],
    w0: usize,
    p: &Params,
    q: &QParams,
    rp: &ResolveParams,
    out: &mut Found,
) {
    let (slo, shi) = (
        f.scan_lo[i] as usize,
        (f.scan_hi[i] as usize).min(rt.len() - 1),
    );
    if f.npts[i] < 2 * p.min_points as u16
        || !(f.scan_hi[i] - f.scan_lo[i] + 1 > p.long_trace || (f.shape[i] as f64) < rp.max_shape)
    {
        return;
    }
    if !may_split(&y[slo - w0..=shi - w0]) {
        return;
    }
    let w1 = w0 + y.len() - 1;
    let mut prom = rp.prom;
    prom.min_scans = p.min_points as usize;
    let mut peaks: Vec<_> = detect_prominence(&rt[w0..=w1], y, prom)
        .into_iter()
        .filter(|pk| pk.apex_i + w0 >= slo && pk.apex_i + w0 <= shi)
        .collect();
    if peaks.len() < 2 {
        return;
    }
    peaks.sort_by_key(|pk| pk.apex_i);
    // consecutive maxima: separated by a valley or part of the same feature
    let mut cuts: Vec<usize> = Vec::new();
    let mut deepest: Option<(f64, usize)> = None;
    for w in peaks.windows(2) {
        let (x, z) = (w[0].apex_i, w[1].apex_i);
        let mut v = x;
        for t in x..=z {
            if y[t] < y[v] {
                v = t;
            }
        }
        let ratio = y[v] / y[x].min(y[z]).max(1e-300);
        if ratio < rp.valley_frac && rt[w0 + z] - rt[w0 + x] >= rp.min_sep {
            cuts.push(v);
        } else if deepest.is_none_or(|d| ratio < d.0) {
            deepest = Some((ratio, v));
        }
    }
    if cuts.is_empty() {
        if let Some((_, v)) = deepest {
            out.multi.push((i, w0 + v));
        }
        return;
    }
    // pieces: from the edge of the feature to the first cut, between cuts, from the last cut to the other edge
    let mut bounds = vec![slo - w0];
    bounds.extend(cuts.iter().copied());
    bounds.push(shi - w0);
    for g in 0..bounds.len() - 1 {
        let (lo, hi) = (bounds[g], bounds[g + 1]);
        let mut k2 = lo;
        for t in lo..=hi {
            if y[t] > y[k2] {
                k2 = t;
            }
        }
        let mut tmp = FileFeatures {
            dt: f.dt,
            ..Default::default()
        };
        tmp.push_row(f, i);
        tmp.scan_lo[0] = (w0 + lo) as u32;
        tmp.scan_hi[0] = (w0 + hi) as u32;
        tmp.apex[0] = (w0 + k2) as u32;
        tmp.rt[0] = rt[w0 + k2] as f32;
        tmp.left[0] = rt[w0 + lo] as f32;
        tmp.right[0] = rt[w0 + hi] as f32;
        tmp.height[0] = y[k2] as f32;
        tmp.area[0] = y[lo..=hi].iter().sum::<f64>() * f.dt;
        tmp.npts[0] = y[lo..=hi]
            .iter()
            .filter(|&&v| v > 0.0)
            .count()
            .min(u16::MAX as usize) as u16;
        tmp.flags[0] |= FLAG_RESOLVED;
        let (wa, wb) = around_apex(rt, w0 + k2, AREA_HALF_WINDOW);
        fill_row(&mut tmp, 0, rt, y, (w0, wa.max(w0), wb.min(w1)), p.floor, q);
        out.pieces.push((i, tmp));
    }
}

/// `quality::assess` and, in the same walk through the peak table, the cut of the features with several separated maxima. Rows keep the
/// order by m/z (the pieces of one ion are consecutive, by time). Run before the isotope groups and the MS2 links.
pub fn assess_and_resolve<R: ReadAt>(
    index: &mut Index<R>,
    f: &mut FileFeatures,
    p: &Params,
    q: &QParams,
    rp: &ResolveParams,
) -> Result<Resolved> {
    let mut found = Found::default();
    assess_with(index, f, p, q, |f, i, rt, y, w0| {
        look(f, i, rt, y, w0, p, q, rp, &mut found)
    })?;
    let mut out = Resolved::default();
    let Some(si) = index.section_of(1, p.polarity) else {
        return Ok(out);
    };
    if found.pieces.is_empty() && found.multi.is_empty() {
        return Ok(out);
    }
    let rt: Vec<f64> = index.arrays(si)?.rt.clone();
    let (mut pieces, mut multi) = (found.pieces, found.multi);
    pieces.sort_by_key(|x| x.0);
    multi.sort();
    let mut g = FileFeatures {
        dt: f.dt,
        ..Default::default()
    };
    let (mut pi, mut mi) = (0, 0);
    for i in 0..f.len() {
        if pi < pieces.len() && pieces[pi].0 == i {
            out.split += 1;
            while pi < pieces.len() && pieces[pi].0 == i {
                g.push_row(&pieces[pi].1, 0);
                out.pieces += 1;
                pi += 1;
            }
            continue;
        }
        if mi < multi.len() && multi[mi].0 == i {
            out.multi.push(Multi {
                row: g.len(),
                valley_rt: rt[multi[mi].1],
            });
            mi += 1;
        }
        g.push_row(f, i);
    }
    *f = g;
    Ok(out)
}

/// Square-rooted spectra, binned at `bin` m/z, of the MS2 scans `want` (global scan numbers) from one pass over the MS2 section.
fn spectra<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
    want: &[u32],
    bin: f64,
) -> Result<Vec<Vec<(i64, f64)>>> {
    let ids = index.arrays(si)?.scan_ids.clone();
    let mut slot = vec![u32::MAX; ids.len()];
    for (k, &g) in ids.iter().enumerate() {
        if let Ok(j) = want.binary_search(&(g.max(0) as u32)) {
            if g >= 0 {
                slot[k] = j as u32;
            }
        }
    }
    let mut raw: Vec<Vec<(i64, f64)>> = vec![Vec::new(); want.len()];
    let n = index.sections[si].n_peaks;
    index.each_peak(si, 0, n, |pos, mz, it| {
        if let Some(&j) = slot.get(pos) {
            if j != u32::MAX {
                raw[j as usize].push(((mz / bin).floor() as i64, it));
            }
        }
    })?;
    Ok(raw
        .into_iter()
        .map(|mut v| {
            v.sort_by_key(|x| x.0);
            let mut out: Vec<(i64, f64)> = Vec::new();
            for (k, w) in v {
                match out.last_mut() {
                    Some(l) if l.0 == k => l.1 += w,
                    _ => out.push((k, w)),
                }
            }
            for o in &mut out {
                o.1 = libm::sqrt(o.1);
            }
            out
        })
        .collect())
}

/// Binned spectrum: (bin, weight) ascending by bin.
type Spec = Vec<(i64, f64)>;

fn add_into(acc: &mut Vec<(i64, f64)>, s: &[(i64, f64)]) {
    let mut m: Vec<(i64, f64)> = Vec::with_capacity(acc.len() + s.len());
    let (mut x, mut y) = (0, 0);
    while x < acc.len() || y < s.len() {
        if y >= s.len() || (x < acc.len() && acc[x].0 < s[y].0) {
            m.push(acc[x]);
            x += 1;
        } else if x >= acc.len() || s[y].0 < acc[x].0 {
            m.push(s[y]);
            y += 1;
        } else {
            m.push((acc[x].0, acc[x].1 + s[y].1));
            x += 1;
            y += 1;
        }
    }
    *acc = m;
}

fn cosine(a: &[(i64, f64)], b: &[(i64, f64)]) -> f64 {
    let (mut dot, mut na, mut nb) = (0.0, 0.0, 0.0);
    let (mut x, mut y) = (0, 0);
    while x < a.len() && y < b.len() {
        if a[x].0 < b[y].0 {
            x += 1;
        } else if a[x].0 > b[y].0 {
            y += 1;
        } else {
            dot += a[x].1 * b[y].1;
            x += 1;
            y += 1;
        }
    }
    for v in a {
        na += v.1 * v.1;
    }
    for v in b {
        nb += v.1 * v.1;
    }
    if na <= 0.0 || nb <= 0.0 {
        0.0
    } else {
        dot / libm::sqrt(na * nb)
    }
}

/// `FLAG_MIXTURE` on the unseparated features whose MS2 spectra change across the deepest valley (they need an MS2 scan on each side).
/// Returns how many were marked.
pub fn mark_mixtures<R: ReadAt>(
    index: &mut Index<R>,
    f: &mut FileFeatures,
    links: &Ms2Links,
    multi: &[Multi],
    p: &Params,
    rp: &ResolveParams,
) -> Result<usize> {
    let Some(s2) = index.section_of(2, p.polarity) else {
        return Ok(0);
    };
    let mut want: Vec<u32> = multi
        .iter()
        .flat_map(|m| links.of(m.row).0.iter().copied())
        .collect();
    want.sort_unstable();
    want.dedup();
    let sp = spectra(index, s2, &want, rp.bin)?;
    let mut marked = 0;
    for m in multi {
        let (scans, _) = links.of(m.row);
        let (mut left, mut right) = (Spec::new(), Spec::new());
        let (mut nl, mut nr) = (0, 0);
        for &g in scans {
            let sp = &sp[want.binary_search(&g).unwrap_or(0)];
            if index.scans[g as usize].rt < m.valley_rt {
                add_into(&mut left, sp);
                nl += 1;
            } else {
                add_into(&mut right, sp);
                nr += 1;
            }
        }
        if nl > 0 && nr > 0 && cosine(&left, &right) < rp.mix_cosine {
            f.flags[m.row] |= FLAG_MIXTURE;
            marked += 1;
        }
    }
    Ok(marked)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gauss(n: usize, c: f64, h: f64, s: f64) -> impl Fn(usize) -> f64 {
        move |i| {
            let d = (i as f64 - c) / s;
            let _ = n;
            h * libm::exp(-0.5 * d * d)
        }
    }

    /// One feature spanning the whole trace `y` (scans every 0.01 min).
    fn row(y: &[f64]) -> (FileFeatures, Vec<f64>) {
        let n = y.len();
        let mut f = FileFeatures {
            dt: 0.6,
            ..Default::default()
        };
        macro_rules! one {
            ($($c:ident = $v:expr),*) => { $( f.$c.push($v); )* };
        }
        one!(
            mz = 300.0,
            rt = 0.0,
            left = 0.0,
            right = 0.0,
            apex = 0,
            scan_lo = 0,
            scan_hi = (n - 1) as u32,
            area = 0.0,
            height = 0.0
        );
        one!(
            npts = n as u16,
            flags = 0,
            iso_group = 0,
            charge = 0,
            shape = 0.5,
            snr = 10.0,
            fwhm = 0.0,
            asym = 0.0,
            gaps = 0,
            qreason = 0
        );
        one!(ms2_n = 0, purity = -1.0, area_win = 0.0);
        (f, (0..n).map(|i| i as f64 * 0.01).collect())
    }

    fn run(y: &[f64]) -> Found {
        let (f, rt) = row(y);
        let mut out = Found::default();
        look(
            &f,
            0,
            &rt,
            y,
            0,
            &Params::default(),
            &QParams::default(),
            &ResolveParams::default(),
            &mut out,
        );
        out
    }

    fn two(sep: f64, h2: f64, s: f64) -> Vec<f64> {
        let (a, b) = (gauss(0, 40.0, 1000.0, s), gauss(0, 40.0 + sep, h2, s));
        (0..100 + sep as usize).map(|i| a(i) + b(i) + 3.0).collect()
    }

    #[test]
    fn separated_ions_become_two_rows() {
        let f = run(&two(30.0, 800.0, 4.0));
        assert_eq!(f.pieces.len(), 2);
        assert!(f.multi.is_empty());
        let (a, b) = (&f.pieces[0].1, &f.pieces[1].1);
        assert!(a.flags[0] & FLAG_RESOLVED != 0 && a.apex[0] < b.apex[0]);
        assert!((a.apex[0] as i64 - 40).abs() <= 1 && (b.apex[0] as i64 - 70).abs() <= 1);
        assert!(a.scan_hi[0] <= b.scan_lo[0] + 1);
    }

    #[test]
    fn apexes_closer_than_the_limit_stay_one() {
        // 8 scans = 0.08 min apart with a deep valley: not two ions
        let f = run(&two(8.0, 900.0, 1.5));
        assert!(f.pieces.is_empty());
    }

    #[test]
    fn shallow_valley_is_a_possible_mixture() {
        // two maxima 0.16 min apart whose valley stays above half of the lower one
        let f = run(&two(16.0, 950.0, 5.0));
        assert!(f.pieces.is_empty());
        assert_eq!(f.multi.len(), 1);
    }

    #[test]
    fn one_gaussian_is_left_alone() {
        let g = gauss(0, 50.0, 1000.0, 4.0);
        let y: Vec<f64> = (0..100).map(|i| g(i) + 3.0).collect();
        let f = run(&y);
        assert!(f.pieces.is_empty() && f.multi.is_empty());
    }

    #[test]
    fn cosine_of_equal_spectra_is_one() {
        let a = vec![(1, 4.0), (5, 3.0)];
        assert!((cosine(&a, &a) - 1.0).abs() < 1e-12);
        assert_eq!(cosine(&a, &[(2, 1.0)]), 0.0);
        let mut m = vec![(1, 1.0)];
        add_into(&mut m, &[(0, 2.0), (1, 1.0), (7, 1.0)]);
        assert_eq!(m, vec![(0, 2.0), (1, 2.0), (7, 1.0)]);
    }
}
