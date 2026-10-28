//! Deconvolution of chimeric MS2 spectra of co-eluting isomers via MCR-ALS.
#![allow(clippy::needless_range_loop)]

mod mcr;
use mcr::{interp_linear, mcr_als};

#[cfg(test)]
mod old;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub struct DeconvolveSpectrum {
    pub mz: Vec<f64>,
    pub rel: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeconvolveComponent {
    pub rt_apex: Option<f64>,
    pub spectrum: DeconvolveSpectrum,
    pub profile: Option<Vec<f64>>,
    pub xic_corr: Option<f64>,
    pub n_scans_main: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeconvolveResult {
    pub k: usize,
    pub components: Vec<DeconvolveComponent>,
    pub scans: usize,
    pub fragments: usize,
    pub lof: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct DeconvolveParams {
    pub min_scans: usize,
    pub max_components: usize,
    pub ppm: f64,
    pub min_frac_sig: f64,
    pub subpeak_tol: f64,
}

impl Default for DeconvolveParams {
    fn default() -> Self {
        Self {
            min_scans: 4,
            max_components: 3,
            ppm: 8.0,
            min_frac_sig: 0.02,
            subpeak_tol: 0.07,
        }
    }
}

fn argmax(v: &[f64]) -> usize {
    v.iter()
        .enumerate()
        .fold((0, f64::NEG_INFINITY), |(bi, bv), (i, &x)| {
            if x > bv {
                (i, x)
            } else {
                (bi, bv)
            }
        })
        .0
}

fn pav(v: &[f64]) -> Vec<f64> {
    let (mut vals, mut wts): (Vec<f64>, Vec<usize>) =
        (Vec::with_capacity(v.len()), Vec::with_capacity(v.len()));
    for &x in v {
        vals.push(x);
        wts.push(1);
        while vals.len() > 1 && vals[vals.len() - 2] > vals[vals.len() - 1] {
            let l = vals.len();
            let (tot, wt) = (
                vals[l - 2] * wts[l - 2] as f64 + vals[l - 1] * wts[l - 1] as f64,
                wts[l - 2] + wts[l - 1],
            );
            vals.pop();
            vals.pop();
            wts.pop();
            wts.pop();
            vals.push(tot / wt as f64);
            wts.push(wt);
        }
    }
    let mut out = Vec::with_capacity(v.len());
    for (val, wt) in vals.into_iter().zip(wts) {
        out.extend(std::iter::repeat_n(val, wt));
    }
    out
}

fn isotonic_up_down(c: &[f64]) -> Vec<f64> {
    if c.is_empty() {
        return Vec::new();
    }
    let k = argmax(c);
    let left = pav(&c[..=k]);
    let right_rev: Vec<f64> = c[k..].iter().copied().rev().collect();
    let right: Vec<f64> = pav(&right_rev).into_iter().rev().collect();
    let mut out = Vec::with_capacity(c.len());
    out.extend_from_slice(&left[..k]);
    out.push(if let (Some(&l), Some(&r)) = (left.get(k), right.first()) {
        l.max(r)
    } else {
        c[k]
    });
    if right.len() > 1 {
        out.extend_from_slice(&right[1..]);
    }
    out
}

fn cluster_peaks(peaks: &mut [(f64, f64, usize)], ppm: f64) -> (Vec<usize>, usize) {
    if peaks.is_empty() {
        return (Vec::new(), 0);
    }
    crate::sortc::by(peaks, |a, b| a.0.total_cmp(&b.0));
    let mut lab = vec![0usize; peaks.len()];
    for i in 1..peaks.len() {
        let diff = peaks[i].0 - peaks[i - 1].0;
        lab[i] = if diff > peaks[i].0 * ppm * 1e-6 {
            lab[i - 1] + 1
        } else {
            lab[i - 1]
        };
    }
    let ncol = match lab.last() {
        Some(&l) => l + 1,
        None => 0,
    };
    (lab, ncol)
}

/// Row-major matrix D (`n` scans x `ncol` kept fragments) and the centroid m/z of each kept fragment.
struct FragmentMatrix {
    d: Vec<f64>,
    n: usize,
    ncol: usize,
    cm: Vec<f64>,
}

fn fragment_matrix_flat<S: AsRef<[f64]>>(
    mzs: &[S],
    ints: &[S],
    ppm: f64,
    min_scans: usize,
    row_norm: bool,
) -> FragmentMatrix {
    let n = mzs.len();
    let mut peaks: Vec<(f64, f64, usize)> = Vec::new();
    for (i, (m, it)) in mzs.iter().zip(ints).enumerate() {
        let (m, it) = (m.as_ref(), it.as_ref());
        let max_it = it.iter().fold(0.0f64, |acc, &x| acc.max(x)).max(1e-12);
        for (&mz_val, &it_val) in m.iter().zip(it) {
            peaks.push((mz_val, it_val / max_it, i));
        }
    }
    let (lab, ncol) = cluster_peaks(&mut peaks, ppm);
    if ncol == 0 {
        return FragmentMatrix {
            d: Vec::new(),
            n,
            ncol: 0,
            cm: Vec::new(),
        };
    }
    // The peaks of one cluster are contiguous: accumulate its column in a scratch row, count the scans
    // it fills and keep it (column-major) only when it reaches `min_scans`.
    let mut col = vec![0.0; n];
    let mut seen = vec![usize::MAX; n];
    let mut touched: Vec<usize> = Vec::new();
    let (mut kept, mut cm): (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
    let mut start = 0;
    while start < peaks.len() {
        let l = lab[start];
        let mut end = start;
        let (mut sum_it, mut sum_itmz) = (0.0, 0.0);
        while end < peaks.len() && lab[end] == l {
            let p = &peaks[end];
            if p.2 < n {
                if seen[p.2] != l {
                    seen[p.2] = l;
                    touched.push(p.2);
                }
                col[p.2] += p.1;
                sum_it += p.1;
                sum_itmz += p.1 * p.0;
            }
            end += 1;
        }
        let count = touched.iter().filter(|&&r| col[r] > 0.0).count();
        if count >= min_scans {
            kept.extend_from_slice(&col);
            cm.push(sum_itmz / f64::max(sum_it, 1e-12));
        }
        for &r in &touched {
            col[r] = 0.0;
        }
        touched.clear();
        start = end;
    }
    let kcols = cm.len();
    let mut d = vec![0.0; n * kcols];
    for j in 0..kcols {
        for r in 0..n {
            d[r * kcols + j] = kept[j * n + r];
        }
    }
    if row_norm && kcols > 0 {
        for row in d.chunks_exact_mut(kcols) {
            let row_max = row.iter().fold(0.0f64, |acc, &x| acc.max(x)).max(1e-12);
            for v in row.iter_mut() {
                *v /= row_max;
            }
        }
    }
    FragmentMatrix {
        d,
        n,
        ncol: kcols,
        cm,
    }
}

/// Matrix D (scans x fragments) and centroid m/z values of fragments appearing in at least `min_scans`.
pub fn fragment_matrix(
    mzs: &[Vec<f64>],
    ints: &[Vec<f64>],
    ppm: f64,
    min_scans: usize,
    row_norm: bool,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let f = fragment_matrix_flat(mzs, ints, ppm, min_scans, row_norm);
    let rows = (0..f.n)
        .map(|r| f.d[r * f.ncol..(r + 1) * f.ncol].to_vec())
        .collect();
    (rows, f.cm)
}

/// Consensus MS2 of a feature: peaks present in >= 40% of scans and >= 0.5% base peak.
pub fn ms2_consensus(mzs: &[Vec<f64>], ints: &[Vec<f64>], ppm: f64) -> DeconvolveSpectrum {
    ms2_consensus_of(mzs, ints, ppm)
}

fn ms2_consensus_of<S: AsRef<[f64]>>(mzs: &[S], ints: &[S], ppm: f64) -> DeconvolveSpectrum {
    let n = mzs.len();
    let empty = || DeconvolveSpectrum {
        mz: Vec::new(),
        rel: Vec::new(),
    };
    if n == 0 {
        return empty();
    }
    let mut peaks: Vec<(f64, f64, usize)> = Vec::new();
    for (i, (m, it)) in mzs.iter().zip(ints).enumerate() {
        let (m, it) = (m.as_ref(), it.as_ref());
        let max_it = it.iter().fold(0.0f64, |acc, &x| acc.max(x));
        if max_it > 0.0 {
            for (&mz_val, &it_val) in m.iter().zip(it) {
                if it_val > 0.0 {
                    peaks.push((mz_val, it_val / max_it, i));
                }
            }
        }
    }
    let (lab, ncol) = cluster_peaks(&mut peaks, ppm);
    if ncol == 0 {
        return empty();
    }
    // Clusters are contiguous runs of the sorted peaks, so one stamp per scan counts its unique scans.
    let mut stamp = vec![usize::MAX; n];
    let threshold_nsc = (0.4 * n as f64).max(1.0);
    let (mut mz_out, mut rel_out) = (Vec::new(), Vec::new());
    let mut start = 0;
    while start < peaks.len() {
        let l = lab[start];
        let (mut sum_w, mut sum_wmz, mut nsc) = (0.0, 0.0, 0usize);
        let mut end = start;
        while end < peaks.len() && lab[end] == l {
            let p = &peaks[end];
            sum_w += p.1;
            sum_wmz += p.1 * p.0;
            if stamp[p.2] != l {
                stamp[p.2] = l;
                nsc += 1;
            }
            end += 1;
        }
        let mean = sum_w / (n as f64);
        if nsc as f64 >= threshold_nsc && mean >= 0.005 {
            mz_out.push(sum_wmz / f64::max(sum_w, 1e-12));
            rel_out.push(mean);
        }
        start = end;
    }
    if !rel_out.is_empty() {
        let max_rel = rel_out.iter().fold(0.0f64, |acc, &x| acc.max(x)).max(1e-12);
        for v in &mut rel_out {
            *v = 100.0 * *v / max_rel;
        }
    }
    DeconvolveSpectrum {
        mz: mz_out,
        rel: rel_out,
    }
}

fn pearson_corr(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.is_empty() {
        return None;
    }
    let n = x.len() as f64;
    let (mean_x, mean_y) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let (mut num, mut den_x, mut den_y) = (0.0, 0.0, 0.0);
    for (&xi, &yi) in x.iter().zip(y) {
        let (dx, dy) = (xi - mean_x, yi - mean_y);
        num += dx * dy;
        den_x += dx * dx;
        den_y += dy * dy;
    }
    let (sd_x, sd_y) = ((den_x / n).sqrt(), (den_y / n).sqrt());
    if sd_x > 0.0 && sd_y > 0.0 {
        Some((num / (den_x.sqrt() * den_y.sqrt())).clamp(-1.0, 1.0))
    } else {
        None
    }
}

/// Seed spectra (`m x k`, row-major) from the sub-peaks of the XIC; `d` is the `n x m` matrix.
fn find_seeds(
    xr: &[f64],
    xy: &[f64],
    rts: &[f64],
    d: &[f64],
    (n, m): (usize, usize),
    max_components: usize,
    subpeak_tol: f64,
) -> Option<(usize, Vec<f64>)> {
    if xr.len() <= 3 || xy.len() <= 3 || n == 0 {
        return None;
    }
    let peaks = mzlab_core::subpeaks::subpeaks(xr, xy, 0.15, 0.03, 0.08);
    let mut seeds: Vec<(f64, Vec<usize>)> = Vec::new();
    for p in peaks {
        let ix: Vec<usize> = rts
            .iter()
            .enumerate()
            .filter(|(_, &rt)| (rt - p).abs() <= subpeak_tol)
            .map(|(i, _)| i)
            .collect();
        if !ix.is_empty() {
            seeds.push((p, ix));
        }
    }
    if seeds.len() < 2 {
        return None;
    }
    let k = seeds.len().min(max_components);
    crate::sortc::any_by(&mut seeds, |a, b| b.1.len().cmp(&a.1.len()));
    seeds.truncate(k);
    crate::sortc::any_by(&mut seeds, |a, b| a.0.total_cmp(&b.0));
    let mut s0 = vec![0.0; m * k];
    for (col, (_, ix)) in seeds.iter().enumerate() {
        let count = ix.len() as f64;
        for f in 0..m {
            s0[f * k + col] = ix.iter().map(|&r| d[r * m + f]).sum::<f64>() / count;
        }
    }
    Some((k, s0))
}

/// Components from the profiles `c` (`n x k`) and spectra `s` (`m x k`), ordered by retention time.
fn build_components(
    (c, s): (&[f64], &[f64]),
    k: usize,
    cm: &[f64],
    rts: &[f64],
    xic: Option<(&[f64], &[f64])>,
    min_frac_sig: f64,
) -> Vec<DeconvolveComponent> {
    let (n, m) = (c.len() / k.max(1), cm.len());
    // The XIC resampled on the scan times does not depend on the component.
    let xic_on_rts = match xic {
        Some((xr, xy)) if xr.len() > 3 && xy.len() > 3 => Some(interp_linear(rts, xr, xy)),
        _ => None,
    };
    let mut comps = Vec::with_capacity(k);
    for j in 0..k {
        let col: Vec<f64> = (0..n).map(|i| c[i * k + j]).collect();
        let rt_apex = rts.get(argmax(&col)).copied();
        let s_col: Vec<f64> = (0..m).map(|i| s[i * k + j]).collect();
        let s_max = s_col.iter().fold(0.0f64, |acc, &x| acc.max(x)).max(1e-12);
        let (mut spec_mz, mut spec_rel) = (Vec::new(), Vec::new());
        for (f, &val) in s_col.iter().enumerate() {
            if val >= min_frac_sig * s_max {
                spec_mz.push(cm[f]);
                spec_rel.push(100.0 * val / s_max);
            }
        }
        let max_c = col.iter().fold(0.0f64, |acc, &x| acc.max(x)).max(1e-12);
        let prof: Vec<f64> = col.iter().map(|&v| v / max_c).collect();
        let n_scans_main = Some(prof.iter().filter(|&&v| v >= 0.5).count());
        let xic_corr = xic_on_rts.as_deref().and_then(|y| pearson_corr(&prof, y));
        comps.push(DeconvolveComponent {
            rt_apex,
            spectrum: DeconvolveSpectrum {
                mz: spec_mz,
                rel: spec_rel,
            },
            profile: Some(prof),
            xic_corr,
            n_scans_main,
        });
    }
    crate::sortc::any_by(&mut comps, |a, b| match (a.rt_apex, b.rt_apex) {
        (Some(ra), Some(rb)) => ra.total_cmp(&rb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    comps
}

fn single_comp_res(
    mzs: &[&[f64]],
    ints: &[&[f64]],
    rts: &[f64],
    rt_apex: Option<f64>,
    ppm: f64,
    frags: usize,
) -> DeconvolveResult {
    let cons = ms2_consensus_of(mzs, ints, ppm);
    let comp = DeconvolveComponent {
        rt_apex,
        spectrum: cons,
        profile: None,
        xic_corr: None,
        n_scans_main: None,
    };
    DeconvolveResult {
        k: 1,
        components: vec![comp],
        scans: rts.len(),
        fragments: frags,
        lof: None,
    }
}

/// Deconvolve chimeric MS2 spectra of co-eluting isomers across chromatographic scans.
pub fn deconvolve_isomers(
    mzs: &[Vec<f64>],
    ints: &[Vec<f64>],
    rts: &[f64],
    xic: Option<(&[f64], &[f64])>,
    params: &DeconvolveParams,
) -> DeconvolveResult {
    let n_in = rts.len().min(mzs.len()).min(ints.len());
    let mut order: Vec<usize> = (0..n_in).collect();
    crate::sortc::by(&mut order, |&a, &b| rts[a].total_cmp(&rts[b]));
    let rts_sorted: Vec<f64> = order.iter().map(|&i| rts[i]).collect();
    let mzs_sorted: Vec<&[f64]> = order.iter().map(|&i| mzs[i].as_slice()).collect();
    let ints_sorted: Vec<&[f64]> = order.iter().map(|&i| ints[i].as_slice()).collect();

    let FragmentMatrix { d, cm, .. } =
        fragment_matrix_flat(&mzs_sorted, &ints_sorted, params.ppm, 2, true);
    let (n, m) = (rts_sorted.len(), cm.len());

    if n < params.min_scans || m < 3 {
        let sums: Vec<f64> = ints_sorted.iter().map(|row| row.iter().sum()).collect();
        let rt_apex = if n > 0 {
            rts_sorted.get(argmax(&sums)).copied()
        } else {
            None
        };
        return single_comp_res(
            &mzs_sorted,
            &ints_sorted,
            &rts_sorted,
            rt_apex,
            params.ppm,
            m,
        );
    }

    let est = mzlab_core::components::estimate_components(&d, n, m, params.max_components, 0.002);
    let mut k = est.k;
    let mut s0: Option<Vec<f64>> = None;

    if let Some((xr, xy)) = xic {
        if let Some((k_seed, s0_seed)) = find_seeds(
            xr,
            xy,
            &rts_sorted,
            &d,
            (n, m),
            params.max_components,
            params.subpeak_tol,
        ) {
            k = k_seed;
            s0 = Some(s0_seed);
        }
    }

    if k < 2 {
        let sums: Vec<f64> = d.chunks_exact(m).map(|row| row.iter().sum()).collect();
        let rt_apex = rts_sorted.get(argmax(&sums)).copied();
        return single_comp_res(
            &mzs_sorted,
            &ints_sorted,
            &rts_sorted,
            rt_apex,
            params.ppm,
            m,
        );
    }

    let (c, s, lof) = mcr_als(&d, n, m, k, true, s0);
    let comps = build_components((&c, &s), k, &cm, &rts_sorted, xic, params.min_frac_sig);
    DeconvolveResult {
        k,
        components: comps,
        scans: n,
        fragments: m,
        lof: Some(lof),
    }
}
