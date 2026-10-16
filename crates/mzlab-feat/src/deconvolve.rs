//! Deconvolution of chimeric MS2 spectra of co-eluting isomers via MCR-ALS.
#![allow(clippy::needless_range_loop)]

mod mcr;
use mcr::{interp_linear, mcr_als};

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
    let (mut vals, mut wts): (Vec<f64>, Vec<f64>) =
        (Vec::with_capacity(v.len()), Vec::with_capacity(v.len()));
    for &x in v {
        vals.push(x);
        wts.push(1.0);
        while vals.len() > 1 && vals[vals.len() - 2] > vals[vals.len() - 1] {
            let l = vals.len();
            let (tot, wt) = (
                vals[l - 2] * wts[l - 2] + vals[l - 1] * wts[l - 1],
                wts[l - 2] + wts[l - 1],
            );
            vals.pop();
            vals.pop();
            wts.pop();
            wts.pop();
            vals.push(tot / wt);
            wts.push(wt);
        }
    }
    let mut out = Vec::with_capacity(v.len());
    for (val, wt) in vals.into_iter().zip(wts) {
        for _ in 0..wt.round() as usize {
            out.push(val);
        }
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

/// Matrix D (scans x fragments) and centroid m/z values of fragments appearing in at least `min_scans`.
pub fn fragment_matrix(
    mzs: &[Vec<f64>],
    ints: &[Vec<f64>],
    ppm: f64,
    min_scans: usize,
    row_norm: bool,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let n = mzs.len();
    let mut peaks: Vec<(f64, f64, usize)> = Vec::new();
    for (i, (m, it)) in mzs.iter().zip(ints).enumerate() {
        let max_it = it.iter().fold(0.0f64, |acc, &x| acc.max(x)).max(1e-12);
        for (&mz_val, &it_val) in m.iter().zip(it) {
            peaks.push((mz_val, it_val / max_it, i));
        }
    }
    let (lab, ncol) = cluster_peaks(&mut peaks, ppm);
    if ncol == 0 {
        return (vec![Vec::new(); n], Vec::new());
    }
    let (mut sum_it, mut sum_itmz) = (vec![0.0; ncol], vec![0.0; ncol]);
    let mut d = vec![vec![0.0; ncol]; n];
    for (p, &l) in peaks.iter().zip(&lab) {
        if p.2 < n && l < ncol {
            d[p.2][l] += p.1;
            sum_it[l] += p.1;
            sum_itmz[l] += p.1 * p.0;
        }
    }
    let cm: Vec<f64> = (0..ncol)
        .map(|l| sum_itmz[l] / sum_it[l].max(1e-12))
        .collect();
    let keep: Vec<bool> = (0..ncol)
        .map(|l| (0..n).filter(|&r| d[r][l] > 0.0).count() >= min_scans)
        .collect();
    let kept_cm: Vec<f64> = cm
        .into_iter()
        .enumerate()
        .filter(|(l, _)| keep.get(*l).copied().unwrap_or(false))
        .map(|(_, v)| v)
        .collect();
    let mut kept_d: Vec<Vec<f64>> = (0..n)
        .map(|r| {
            d[r].iter()
                .enumerate()
                .filter(|(l, _)| keep.get(*l).copied().unwrap_or(false))
                .map(|(_, &v)| v)
                .collect()
        })
        .collect();
    if row_norm && !kept_cm.is_empty() {
        for row in &mut kept_d {
            let row_max = row.iter().fold(0.0f64, |acc, &x| acc.max(x)).max(1e-12);
            for v in row.iter_mut() {
                *v /= row_max;
            }
        }
    }
    (kept_d, kept_cm)
}

/// Consensus MS2 of a feature: peaks present in >= 40% of scans and >= 0.5% base peak.
pub fn ms2_consensus(mzs: &[Vec<f64>], ints: &[Vec<f64>], ppm: f64) -> DeconvolveSpectrum {
    let n = mzs.len();
    if n == 0 {
        return DeconvolveSpectrum {
            mz: Vec::new(),
            rel: Vec::new(),
        };
    }
    let mut peaks: Vec<(f64, f64, usize)> = Vec::new();
    for (i, (m, it)) in mzs.iter().zip(ints).enumerate() {
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
        return DeconvolveSpectrum {
            mz: Vec::new(),
            rel: Vec::new(),
        };
    }
    let (mut sum_w, mut sum_wmz) = (vec![0.0; ncol], vec![0.0; ncol]);
    let mut seen_scans: Vec<std::collections::BTreeSet<usize>> =
        vec![std::collections::BTreeSet::new(); ncol];
    for (p, &l) in peaks.iter().zip(&lab) {
        if l < ncol {
            sum_w[l] += p.1;
            sum_wmz[l] += p.1 * p.0;
            seen_scans[l].insert(p.2);
        }
    }
    let threshold_nsc = (0.4 * n as f64).max(1.0);
    let (mut mz_out, mut rel_out) = (Vec::new(), Vec::new());
    for l in 0..ncol {
        let (nsc, mean) = (seen_scans[l].len() as f64, sum_w[l] / (n as f64));
        if nsc >= threshold_nsc && mean >= 0.005 {
            mz_out.push(sum_wmz[l] / sum_w[l].max(1e-12));
            rel_out.push(mean);
        }
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

fn find_seeds(
    xr: &[f64],
    xy: &[f64],
    rts: &[f64],
    d: &[Vec<f64>],
    max_components: usize,
    subpeak_tol: f64,
) -> Option<(usize, Vec<Vec<f64>>)> {
    if xr.len() <= 3 || xy.len() <= 3 {
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
    let m = d.first()?.len();
    let mut s0 = vec![vec![0.0; k]; m];
    for (col, (_, ix)) in seeds.iter().enumerate() {
        let count = ix.len() as f64;
        for f in 0..m {
            s0[f][col] = ix.iter().map(|&r| d[r][f]).sum::<f64>() / count;
        }
    }
    Some((k, s0))
}

fn build_components(
    c: &[Vec<f64>],
    s: &[Vec<f64>],
    cm: &[f64],
    rts: &[f64],
    xic: Option<(&[f64], &[f64])>,
    min_frac_sig: f64,
) -> Vec<DeconvolveComponent> {
    let (n, m, k) = (c.len(), cm.len(), c.first().map_or(0, |r| r.len()));
    let mut comps = Vec::with_capacity(k);
    for j in 0..k {
        let col: Vec<f64> = (0..n).map(|i| c[i][j]).collect();
        let rt_apex = rts.get(argmax(&col)).copied();
        let s_col: Vec<f64> = (0..m).map(|i| s[i][j]).collect();
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
        let xic_corr = match xic {
            Some((xr, xy)) if xr.len() > 3 && xy.len() > 3 => {
                pearson_corr(&prof, &interp_linear(rts, xr, xy))
            }
            _ => None,
        };
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
    mzs: &[Vec<f64>],
    ints: &[Vec<f64>],
    rts: &[f64],
    rt_apex: Option<f64>,
    ppm: f64,
    frags: usize,
) -> DeconvolveResult {
    let cons = ms2_consensus(mzs, ints, ppm);
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
    let mzs_sorted: Vec<Vec<f64>> = order.iter().map(|&i| mzs[i].clone()).collect();
    let ints_sorted: Vec<Vec<f64>> = order.iter().map(|&i| ints[i].clone()).collect();

    let (d, cm) = fragment_matrix(&mzs_sorted, &ints_sorted, params.ppm, 2, true);
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

    let mut d_flat = Vec::with_capacity(n * m);
    for row in &d {
        d_flat.extend_from_slice(row);
    }
    let est =
        mzlab_core::components::estimate_components(&d_flat, n, m, params.max_components, 0.002);
    let mut k = est.k;
    let mut s0: Option<Vec<Vec<f64>>> = None;

    if let Some((xr, xy)) = xic {
        if let Some((k_seed, s0_seed)) = find_seeds(
            xr,
            xy,
            &rts_sorted,
            &d,
            params.max_components,
            params.subpeak_tol,
        ) {
            k = k_seed;
            s0 = Some(s0_seed);
        }
    }

    if k < 2 {
        let sums: Vec<f64> = d.iter().map(|row| row.iter().sum()).collect();
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

    let (c, s, lof) = mcr_als(&d, k, true, s0);
    let comps = build_components(&c, &s, &cm, &rts_sorted, xic, params.min_frac_sig);
    DeconvolveResult {
        k,
        components: comps,
        scans: n,
        fragments: m,
        lof: Some(lof),
    }
}
