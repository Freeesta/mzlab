//! Deconvolution of chimeric MS2 spectra of co-eluting isomers via MCR-ALS.
#![allow(clippy::needless_range_loop)]

#[derive(Debug, Clone, PartialEq)]
pub struct DeconvolveSpectrum { pub mz: Vec<f64>, pub rel: Vec<f64> }

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
    pub scans: usize, pub fragments: usize, pub lof: Option<f64>,
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
        Self { min_scans: 4, max_components: 3, ppm: 8.0, min_frac_sig: 0.02, subpeak_tol: 0.07 }
    }
}

fn argmax(v: &[f64]) -> usize {
    v.iter().enumerate().fold((0, f64::NEG_INFINITY), |(bi, bv), (i, &x)| if x > bv { (i, x) } else { (bi, bv) }).0
}

fn pav(v: &[f64]) -> Vec<f64> {
    let (mut vals, mut wts): (Vec<f64>, Vec<f64>) = (Vec::with_capacity(v.len()), Vec::with_capacity(v.len()));
    for &x in v {
        vals.push(x);
        wts.push(1.0);
        while vals.len() > 1 && vals[vals.len() - 2] > vals[vals.len() - 1] {
            let l = vals.len();
            let (tot, wt) = (vals[l - 2] * wts[l - 2] + vals[l - 1] * wts[l - 1], wts[l - 2] + wts[l - 1]);
            vals.pop(); vals.pop(); wts.pop(); wts.pop();
            vals.push(tot / wt); wts.push(wt);
        }
    }
    let mut out = Vec::with_capacity(v.len());
    for (val, wt) in vals.into_iter().zip(wts) {
        for _ in 0..wt.round() as usize { out.push(val); }
    }
    out
}

fn isotonic_up_down(c: &[f64]) -> Vec<f64> {
    if c.is_empty() { return Vec::new(); }
    let k = argmax(c);
    let left = pav(&c[..=k]);
    let right_rev: Vec<f64> = c[k..].iter().copied().rev().collect();
    let right: Vec<f64> = pav(&right_rev).into_iter().rev().collect();
    let mut out = Vec::with_capacity(c.len());
    out.extend_from_slice(&left[..k]);
    out.push(if let (Some(&l), Some(&r)) = (left.get(k), right.first()) { l.max(r) } else { c[k] });
    if right.len() > 1 { out.extend_from_slice(&right[1..]); }
    out
}

fn cluster_peaks(peaks: &mut [(f64, f64, usize)], ppm: f64) -> (Vec<usize>, usize) {
    if peaks.is_empty() { return (Vec::new(), 0); }
    crate::sortc::by(peaks, |a, b| a.0.total_cmp(&b.0));
    let mut lab = vec![0usize; peaks.len()];
    for i in 1..peaks.len() {
        let diff = peaks[i].0 - peaks[i - 1].0;
        lab[i] = if diff > peaks[i].0 * ppm * 1e-6 { lab[i - 1] + 1 } else { lab[i - 1] };
    }
    let ncol = match lab.last() { Some(&l) => l + 1, None => 0 };
    (lab, ncol)
}

/// Matrix D (scans x fragments) and centroid m/z values of fragments appearing in at least `min_scans`.
pub fn fragment_matrix(
    mzs: &[Vec<f64>], ints: &[Vec<f64>], ppm: f64, min_scans: usize, row_norm: bool,
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
    if ncol == 0 { return (vec![Vec::new(); n], Vec::new()); }
    let (mut sum_it, mut sum_itmz) = (vec![0.0; ncol], vec![0.0; ncol]);
    let mut d = vec![vec![0.0; ncol]; n];
    for (p, &l) in peaks.iter().zip(&lab) {
        if p.2 < n && l < ncol {
            d[p.2][l] += p.1; sum_it[l] += p.1; sum_itmz[l] += p.1 * p.0;
        }
    }
    let cm: Vec<f64> = (0..ncol).map(|l| sum_itmz[l] / sum_it[l].max(1e-12)).collect();
    let keep: Vec<bool> = (0..ncol).map(|l| (0..n).filter(|&r| d[r][l] > 0.0).count() >= min_scans).collect();
    let kept_cm: Vec<f64> = cm.into_iter().enumerate().filter(|(l, _)| keep.get(*l).copied().unwrap_or(false)).map(|(_, v)| v).collect();
    let mut kept_d: Vec<Vec<f64>> = (0..n)
        .map(|r| d[r].iter().enumerate().filter(|(l, _)| keep.get(*l).copied().unwrap_or(false)).map(|(_, &v)| v).collect())
        .collect();
    if row_norm && !kept_cm.is_empty() {
        for row in &mut kept_d {
            let row_max = row.iter().fold(0.0f64, |acc, &x| acc.max(x)).max(1e-12);
            for v in row.iter_mut() { *v /= row_max; }
        }
    }
    (kept_d, kept_cm)
}

/// Consensus MS2 of a feature: peaks present in >= 40% of scans and >= 0.5% base peak.
pub fn ms2_consensus(mzs: &[Vec<f64>], ints: &[Vec<f64>], ppm: f64) -> DeconvolveSpectrum {
    let n = mzs.len();
    if n == 0 { return DeconvolveSpectrum { mz: Vec::new(), rel: Vec::new() }; }
    let mut peaks: Vec<(f64, f64, usize)> = Vec::new();
    for (i, (m, it)) in mzs.iter().zip(ints).enumerate() {
        let max_it = it.iter().fold(0.0f64, |acc, &x| acc.max(x));
        if max_it > 0.0 {
            for (&mz_val, &it_val) in m.iter().zip(it) {
                if it_val > 0.0 { peaks.push((mz_val, it_val / max_it, i)); }
            }
        }
    }
    let (lab, ncol) = cluster_peaks(&mut peaks, ppm);
    if ncol == 0 { return DeconvolveSpectrum { mz: Vec::new(), rel: Vec::new() }; }
    let (mut sum_w, mut sum_wmz) = (vec![0.0; ncol], vec![0.0; ncol]);
    let mut seen_scans: Vec<std::collections::BTreeSet<usize>> = vec![std::collections::BTreeSet::new(); ncol];
    for (p, &l) in peaks.iter().zip(&lab) {
        if l < ncol {
            sum_w[l] += p.1; sum_wmz[l] += p.1 * p.0; seen_scans[l].insert(p.2);
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
        for v in &mut rel_out { *v = 100.0 * *v / max_rel; }
    }
    DeconvolveSpectrum { mz: mz_out, rel: rel_out }
}

fn det_mat(mut a: Vec<Vec<f64>>) -> f64 {
    let n = a.len();
    let mut det = 1.0;
    for i in 0..n {
        let mut p = i;
        for r in i + 1..n { if a[r][i].abs() > a[p][i].abs() { p = r; } }
        if a[p][i].abs() < 1e-15 { return 0.0; }
        if p != i { a.swap(i, p); det = -det; }
        let piv = a[i][i];
        det *= piv;
        for r in i + 1..n {
            let f = a[r][i] / piv;
            for c in i..n { a[r][c] -= f * a[i][c]; }
        }
    }
    det
}

fn inv_mat(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut m = vec![vec![0.0; 2 * n]; n];
    for i in 0..n { m[i][..n].copy_from_slice(&a[i]); m[i][n + i] = 1.0; }
    for i in 0..n {
        let mut p = i;
        for r in i + 1..n { if m[r][i].abs() > m[p][i].abs() { p = r; } }
        if m[p][i].abs() < 1e-15 { return None; }
        m.swap(i, p);
        let piv = m[i][i];
        for c in 0..2 * n { m[i][c] /= piv; }
        for r in 0..n {
            if r != i {
                let f = m[r][i];
                for c in 0..2 * n { m[r][c] -= f * m[i][c]; }
            }
        }
    }
    Some((0..n).map(|i| m[i][n..].to_vec()).collect())
}

fn simplisma(d: &[Vec<f64>], k: usize, offset: f64) -> Vec<usize> {
    let (n, m) = (d.len(), d.first().map_or(0, |r| r.len()));
    if n == 0 || m == 0 { return Vec::new(); }
    let (mut mu, mut sd) = (vec![0.0; m], vec![0.0; m]);
    for j in 0..m {
        let sum: f64 = (0..n).map(|i| d[i][j]).sum();
        let mean = sum / (n as f64);
        mu[j] = mean;
        sd[j] = ((0..n).map(|i| (d[i][j] - mean).powi(2)).sum::<f64>() / (n as f64)).sqrt();
    }
    let alpha = offset * mu.iter().fold(0.0f64, |acc, &x| acc.max(x));
    let purity: Vec<f64> = (0..m).map(|j| sd[j] / (mu[j] + alpha)).collect();
    let dn: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..m).map(|j| d[i][j] / ((mu[j].powi(2) + sd[j].powi(2) + 1e-300).sqrt() * (n as f64).sqrt())).collect()
        })
        .collect();

    let mut sel: Vec<usize> = Vec::with_capacity(k);
    for _ in 0..k.min(m) {
        let (mut best, mut best_j) = (-1.0f64, 0);
        for j in 0..m {
            if sel.contains(&j) { continue; }
            let mut cols = vec![j];
            cols.extend_from_slice(&sel);
            let p = cols.len();
            let mut g = vec![vec![0.0; p]; p];
            for a in 0..p {
                for b in 0..p { g[a][b] = (0..n).map(|i| dn[i][cols[a]] * dn[i][cols[b]]).sum(); }
            }
            let sc = purity[j] * det_mat(g);
            if sc > best { best = sc; best_j = j; }
        }
        sel.push(best_j);
    }
    sel
}

fn transpose(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let (n, m) = (a.len(), a.first().map_or(0, |r| r.len()));
    (0..m).map(|j| (0..n).map(|i| a[i][j]).collect()).collect()
}

fn ridge_solve(m_mat: &[Vec<f64>], f_mat: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let (n, m, k) = (m_mat.len(), f_mat.len(), f_mat.first().map_or(0, |r| r.len()));
    let mut ftf = vec![vec![0.0; k]; k];
    for a in 0..k {
        for b in 0..k {
            let sum: f64 = (0..m).map(|l| f_mat[l][a] * f_mat[l][b]).sum();
            ftf[a][b] = sum + if a == b { 1e-12 } else { 0.0 };
        }
    }
    let inv = inv_mat(&ftf).unwrap_or_else(|| {
        (0..k).map(|a| (0..k).map(|b| if a == b { 1.0 } else { 0.0 }).collect()).collect()
    });
    let mut mf = vec![vec![0.0; k]; n];
    for i in 0..n {
        for j in 0..k { mf[i][j] = (0..m).map(|l| m_mat[i][l] * f_mat[l][j]).sum(); }
    }
    let mut out = vec![vec![0.0; k]; n];
    for i in 0..n {
        for j in 0..k {
            out[i][j] = (0..k).map(|l| mf[i][l] * inv[l][j]).sum::<f64>().max(0.0);
        }
    }
    out
}

fn update_c(d: &[Vec<f64>], s: &[Vec<f64>], unimodal: bool) -> Vec<Vec<f64>> {
    let mut c = ridge_solve(d, s);
    if unimodal {
        let (n, k) = (c.len(), c.first().map_or(0, |r| r.len()));
        for j in 0..k {
            let col: Vec<f64> = (0..n).map(|i| c[i][j]).collect();
            if col.iter().fold(0.0f64, |acc, &x| acc.max(x)) > 0.0 {
                let iso = isotonic_up_down(&col);
                for (i, &v) in iso.iter().enumerate().take(n) { c[i][j] = v; }
            }
        }
    }
    c
}

fn update_s(dt: &[Vec<f64>], c: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut s = ridge_solve(dt, c);
    let (m, k) = (s.len(), s.first().map_or(0, |r| r.len()));
    for j in 0..k {
        let mx = (0..m).map(|i| s[i][j]).fold(0.0f64, |acc, x| acc.max(x));
        let scale = if mx <= 0.0 { 1.0 } else { mx };
        for i in 0..m { s[i][j] /= scale; }
    }
    s
}

fn calc_lof(d: &[Vec<f64>], c: &[Vec<f64>], s: &[Vec<f64>]) -> f64 {
    let (n, m, k) = (d.len(), d.first().map_or(0, |r| r.len()), c.first().map_or(0, |r| r.len()));
    let (mut sum_r2, mut sum_d2) = (0.0, 0.0);
    for i in 0..n {
        for j in 0..m {
            let pred: f64 = (0..k).map(|l| c[i][l] * s[j][l]).sum();
            let r = d[i][j] - pred;
            sum_r2 += r * r;
            sum_d2 += d[i][j] * d[i][j];
        }
    }
    99.0 * (sum_r2 / sum_d2.max(1e-300)).sqrt()
}

fn mcr_als(
    d: &[Vec<f64>], k: usize, unimodal: bool, s0: Option<Vec<Vec<f64>>>,
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, f64) {
    let (n, m) = (d.len(), d.first().map_or(0, |r| r.len()));
    let dt = transpose(d);
    let mut s = match s0 {
        Some(init) => init,
        None => {
            let idx = simplisma(d, k, 0.05);
            let mut init = vec![vec![0.0; k]; m];
            for (col, &j) in idx.iter().enumerate() {
                let col_vals: Vec<f64> = (0..n).map(|i| d[i][j]).collect();
                let bi = argmax(&col_vals);
                for f in 0..m { init[f][col] = d[bi][f]; }
            }
            init
        }
    };
    for row in &mut s {
        for v in row.iter_mut() { *v = v.max(1e-12); }
    }
    let mut prev: Option<f64> = None;
    let mut c = vec![vec![0.0; k]; n];
    for _ in 1..=300 {
        c = update_c(d, &s, unimodal);
        s = update_s(&dt, &c);
        let cur = calc_lof(d, &update_c(d, &s, unimodal), &s);
        if let Some(p) = prev {
            if (p - cur).abs() < 1e-6 * p.max(1e-12) { break; }
        }
        prev = Some(cur);
    }
    c = update_c(d, &s, unimodal);
    let lof = calc_lof(d, &c, &s);
    (c, s, lof)
}

fn interp_linear(x: &[f64], xp: &[f64], fp: &[f64]) -> Vec<f64> {
    if xp.is_empty() || fp.is_empty() { return vec![0.0; x.len()]; }
    let n = xp.len().min(fp.len());
    let mut out = Vec::with_capacity(x.len());
    for &t in x {
        if t <= xp[0] {
            out.push(fp[0]);
        } else if t >= xp[n - 1] {
            out.push(fp[n - 1]);
        } else {
            let mut idx = 0;
            while idx + 1 < n && xp[idx + 1] < t { idx += 1; }
            let (x0, x1, y0, y1) = (xp[idx], xp[idx + 1], fp[idx], fp[idx + 1]);
            let frac = if (x1 - x0).abs() > 1e-12 { (t - x0) / (x1 - x0) } else { 0.0 };
            out.push(y0 + frac * (y1 - y0));
        }
    }
    out
}

fn pearson_corr(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.is_empty() { return None; }
    let n = x.len() as f64;
    let (mean_x, mean_y) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let (mut num, mut den_x, mut den_y) = (0.0, 0.0, 0.0);
    for (&xi, &yi) in x.iter().zip(y) {
        let (dx, dy) = (xi - mean_x, yi - mean_y);
        num += dx * dy; den_x += dx * dx; den_y += dy * dy;
    }
    let (sd_x, sd_y) = ((den_x / n).sqrt(), (den_y / n).sqrt());
    if sd_x > 0.0 && sd_y > 0.0 {
        Some((num / (den_x.sqrt() * den_y.sqrt())).clamp(-1.0, 1.0))
    } else {
        None
    }
}

fn find_seeds(
    xr: &[f64], xy: &[f64], rts: &[f64], d: &[Vec<f64>], max_components: usize, subpeak_tol: f64,
) -> Option<(usize, Vec<Vec<f64>>)> {
    if xr.len() <= 3 || xy.len() <= 3 { return None; }
    let peaks = mzlab_core::subpeaks::subpeaks(xr, xy, 0.15, 0.03, 0.08);
    let mut seeds: Vec<(f64, Vec<usize>)> = Vec::new();
    for p in peaks {
        let ix: Vec<usize> = rts.iter().enumerate().filter(|(_, &rt)| (rt - p).abs() <= subpeak_tol).map(|(i, _)| i).collect();
        if !ix.is_empty() { seeds.push((p, ix)); }
    }
    if seeds.len() < 2 { return None; }
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
    c: &[Vec<f64>], s: &[Vec<f64>], cm: &[f64], rts: &[f64], xic: Option<(&[f64], &[f64])>, min_frac_sig: f64,
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
            Some((xr, xy)) if xr.len() > 3 && xy.len() > 3 => pearson_corr(&prof, &interp_linear(rts, xr, xy)),
            _ => None,
        };
        comps.push(DeconvolveComponent {
            rt_apex,
            spectrum: DeconvolveSpectrum { mz: spec_mz, rel: spec_rel },
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
    mzs: &[Vec<f64>], ints: &[Vec<f64>], rts: &[f64], rt_apex: Option<f64>, ppm: f64, frags: usize,
) -> DeconvolveResult {
    let cons = ms2_consensus(mzs, ints, ppm);
    let comp = DeconvolveComponent { rt_apex, spectrum: cons, profile: None, xic_corr: None, n_scans_main: None };
    DeconvolveResult { k: 1, components: vec![comp], scans: rts.len(), fragments: frags, lof: None }
}

/// Deconvolve chimeric MS2 spectra of co-eluting isomers across chromatographic scans.
pub fn deconvolve_isomers(
    mzs: &[Vec<f64>], ints: &[Vec<f64>], rts: &[f64], xic: Option<(&[f64], &[f64])>, params: &DeconvolveParams,
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
        let rt_apex = if n > 0 { rts_sorted.get(argmax(&sums)).copied() } else { None };
        return single_comp_res(&mzs_sorted, &ints_sorted, &rts_sorted, rt_apex, params.ppm, m);
    }

    let mut d_flat = Vec::with_capacity(n * m);
    for row in &d { d_flat.extend_from_slice(row); }
    let est = mzlab_core::components::estimate_components(&d_flat, n, m, params.max_components, 0.002);
    let mut k = est.k;
    let mut s0: Option<Vec<Vec<f64>>> = None;

    if let Some((xr, xy)) = xic {
        if let Some((k_seed, s0_seed)) = find_seeds(xr, xy, &rts_sorted, &d, params.max_components, params.subpeak_tol) {
            k = k_seed;
            s0 = Some(s0_seed);
        }
    }

    if k < 2 {
        let sums: Vec<f64> = d.iter().map(|row| row.iter().sum()).collect();
        let rt_apex = rts_sorted.get(argmax(&sums)).copied();
        return single_comp_res(&mzs_sorted, &ints_sorted, &rts_sorted, rt_apex, params.ppm, m);
    }

    let (c, s, lof) = mcr_als(&d, k, true, s0);
    let comps = build_components(&c, &s, &cm, &rts_sorted, xic, params.min_frac_sig);
    DeconvolveResult { k, components: comps, scans: n, fragments: m, lof: Some(lof) }
}
