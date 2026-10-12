//! `mzlab/statistica.py` (part 3): the exploratory PCA of a feature table (`pca_analysis`). NOT a test: filters, declared imputation,
//! transform, centring and scaling, SVD, stability of the loadings leaving one sample out, 95 % ellipses of the groups of replicates.
//! Matrices are row-major `Vec<f64>`; samples × features unless said.

use std::cmp::Ordering::Equal;

use crate::json::Value;
use crate::jsonb::{floats, int, text, Obj};
use crate::statistica::{bad, blank_filter, ncols, user, StatErr, R};
use crate::statmat::{eigh, svd};
use crate::stats::psum;

pub const MIN_SAMPLES: usize = 5;

pub struct PcaOpts {
    pub factor: f64,
    pub stat: String,
    pub min_detect: i64,
    pub min_intensity: f64,
    pub min_samples: i64,
    pub transform: String,
    pub scaling: String,
    pub ncomp: usize,
}

impl Default for PcaOpts {
    fn default() -> Self {
        PcaOpts { factor: 3.0, stat: "max".into(), min_detect: 1, min_intensity: 0.0, min_samples: 2, transform: "log".into(), scaling: "pareto".into(), ncomp: 5 }
    }
}

/// `impute`: a missing value (0) of a feature becomes 1/5 of its smallest positive value (counted, never silent).
fn impute(x: &mut [Vec<f64>]) -> usize {
    let mut n = 0;
    for r in x.iter_mut() {
        let min = r.iter().cloned().filter(|&v| v > 0.0).fold(f64::INFINITY, f64::min);
        if r.iter().all(|&v| v > 0.0) {
            continue;
        }
        for v in r.iter_mut().filter(|v| !(**v > 0.0)) {
            *v = if min.is_finite() { min / 5.0 } else { 0.0 };
            n += 1;
        }
    }
    n
}

/// Column `j` of a row-major matrix with `p` columns.
fn col(z: &[f64], p: usize, j: usize) -> impl Iterator<Item = f64> + '_ {
    z.iter().skip(j).step_by(p).cloned()
}

/// `std(axis=0, ddof=1)` of the columns of an `n × p` matrix: rows added in order, as numpy does along the first axis.
fn col_sd(z: &[f64], n: usize, p: usize) -> Vec<f64> {
    (0..p).map(|j| {
        let m = col(z, p, j).fold(0.0, |a, v| a + v) / n as f64;
        (col(z, p, j).fold(0.0, |a, v| a + (v - m) * (v - m)) / (n as f64 - 1.0)).sqrt()
    }).collect()
}

/// `_scale`: centre every feature, then divide by sd (auto), sqrt(sd) (Pareto) or nothing.
fn scale(z: &[f64], n: usize, p: usize, scaling: &str) -> Vec<f64> {
    let means: Vec<f64> = (0..p).map(|j| col(z, p, j).fold(0.0, |a, v| a + v) / n as f64).collect();
    let mut c: Vec<f64> = z.iter().enumerate().map(|(i, v)| v - means[i % p]).collect();
    if scaling == "none" {
        return c;
    }
    let sd: Vec<f64> = col_sd(&c, n, p).into_iter().map(|s| if s == 0.0 { 1.0 } else { s }).collect();
    c.iter_mut().enumerate().for_each(|(i, v)| *v /= if scaling == "auto" { sd[i % p] } else { sd[i % p].sqrt() });
    c
}

/// Scores (n × k), loadings (p × k), variance explained by every component, in `statistica._svd`'s sign convention.
struct Fit {
    scores: Vec<f64>,
    loadings: Vec<f64>,
    expl: Vec<f64>,
    k: usize,
}

fn fit(xs: &[f64], n: usize, p: usize, want: usize) -> Fit {
    let r = svd(xs, n, p);
    let k = want.min(r.s.len());
    let (mut u, mut vt) = (r.u.clone(), r.vt.clone());
    for j in 0..k {
        let row = &vt[j * p..(j + 1) * p];
        let at = (0..p).fold(0, |b, i| if row[i].abs() > row[b].abs() { i } else { b });
        if row[at] < 0.0 {
            vt[j * p..(j + 1) * p].iter_mut().for_each(|v| *v = -*v);
            (0..n).for_each(|i| u[i * r.k + j] = -u[i * r.k + j]);
        }
    }
    let total = r.s.iter().map(|s| s * s).sum::<f64>().max(1e-300);
    Fit {
        scores: (0..n).flat_map(|i| (0..k).map(|j| (i, j)).collect::<Vec<_>>()).map(|(i, j)| u[i * r.k + j] * r.s[j]).collect(),
        loadings: (0..p).flat_map(|i| (0..k).map(|j| (i, j)).collect::<Vec<_>>()).map(|(i, j)| vt[j * p + i]).collect(),
        expl: r.s.iter().map(|s| s * s / total).collect(),
        k,
    }
}

/// `_align`: the components of a PCA without one sample, matched to the full ones by the largest |cosine| (order and sign can change).
fn align(full: &[f64], kf: usize, sub: &[f64], ks: usize, p: usize) -> Vec<f64> {
    let dot = |a: usize, b: usize| (0..p).map(|i| full[i * kf + a] * sub[i * ks + b]).sum::<f64>();
    let mut out = vec![0.0; p * kf];
    let mut used = vec![false; ks];
    for j in 0..kf {
        let c: Vec<f64> = (0..ks).map(|i| dot(j, i)).collect();
        let mut order: Vec<usize> = (0..ks).filter(|&i| !used[i]).collect();
        order.sort_by(|&a, &b| c[b].abs().partial_cmp(&c[a].abs()).unwrap_or(Equal));
        let Some(&i) = order.first() else { continue };
        used[i] = true;
        let sign = if c[i] >= 0.0 { 1.0 } else { -1.0 };
        (0..p).for_each(|f| out[f * kf + j] = sub[f * ks + i] * sign);
    }
    out
}

/// `loo_stability`: 1 / (1 + sd of the loading over the repetitions / (1/sqrt(p))), features × `ncomp` (here 2 or fewer).
fn loo_stability(zt: &[f64], n: usize, p: usize, scaling: &str, full: &Fit, ncomp: usize) -> (Vec<f64>, usize) {
    let k = ncomp.min(full.k);
    let mut head = vec![0.0; p * k];
    (0..p).for_each(|f| (0..k).for_each(|j| head[f * k + j] = full.loadings[f * full.k + j]));
    let mut runs: Vec<Vec<f64>> = Vec::with_capacity(n);
    for i in 0..n {
        let sub: Vec<f64> = (0..n).filter(|&r| r != i).flat_map(|r| zt[r * p..(r + 1) * p].to_vec()).collect();
        let f = fit(&scale(&sub, n - 1, p, scaling), n - 1, p, k.max(1) + 1);
        runs.push(align(&head, k, &f.loadings, f.k, p));
    }
    let root = (p as f64).sqrt();
    let stab = (0..p * k).map(|x| {
        let m = runs.iter().fold(0.0, |a, r| a + r[x]) / n as f64;
        let sd = (runs.iter().fold(0.0, |a, r| a + (r[x] - m) * (r[x] - m)) / n as f64).sqrt();
        1.0 / (1.0 + sd * root)
    }).collect();
    (stab, k)
}

/// `ellipse`: the 95 % covariance ellipse of one group of replicates (n × 2 scores): centre, semi-axes, angle of the first axis.
fn ellipse(pts: &[[f64; 2]]) -> Option<Value> {
    if pts.len() < 3 {
        return None;
    }
    let n = pts.len() as f64;
    let c: Vec<f64> = (0..2).map(|d| psum(&pts.iter().map(|q| q[d]).collect::<Vec<_>>()) / n).collect();
    let cov = |a: usize, b: usize| psum(&pts.iter().map(|q| (q[a] - c[a]) * (q[b] - c[b])).collect::<Vec<_>>()) / (n - 1.0);
    let (w, v) = eigh(&[cov(0, 0), cov(0, 1), cov(1, 0), cov(1, 1)], 2);
    let chi2 = 5.991;
    Some(Obj::new().n("cx", c[0]).n("cy", c[1]).n("a", (chi2 * w[1].max(0.0)).sqrt()).n("b", (chi2 * w[0].max(0.0)).sqrt()).n("angle", v[3].atan2(v[1])).done())
}

fn ellipses(groups: &[Option<String>], sm: &[usize], f: &Fit, n: usize) -> Value {
    let mut out = Obj::new();
    let gs: Vec<String> = sm.iter().map(|&i| groups[i].clone().unwrap_or_else(|| "null".into())).collect();
    let mut seen: Vec<&String> = Vec::new();
    for g in &gs {
        if seen.contains(&g) {
            continue;
        }
        seen.push(g);
        let pts: Vec<[f64; 2]> = (0..n).filter(|&j| gs[j] == *g).map(|j| [f.scores[j * f.k], f.scores[j * f.k + 1]]).collect();
        if let (true, Some(e)) = (f.k >= 2, ellipse(&pts)) {
            out.set(g, e);
        }
    }
    out.done()
}

fn rows(a: &[f64], width: usize) -> Value {
    Value::Arr(a.chunks(width.max(1)).map(floats).collect())
}

fn step(name: &str, n: usize) -> Obj {
    Obj::new().put("step", text(name)).i("n", n as i64)
}

/// `pca_analysis`: the table is features × files (areas); indices in the answer are relative to the input.
pub fn pca_analysis(matrix: &[Vec<f64>], roles: &[String], groups: Option<&[Option<String>]>, o: &PcaOpts) -> R<Value> {
    if !["log", "none"].contains(&o.transform.as_str()) || !["pareto", "auto", "none"].contains(&o.scaling.as_str()) {
        return user("err.stat.pca.param", "unknown transform or scaling");
    }
    let sm: Vec<usize> = (0..roles.len()).filter(|&i| roles[i] != "blank" && roles[i] != "dark").collect();
    if sm.len() < MIN_SAMPLES {
        return Err(StatErr { key: Some("err.stat.pca.few"), params: vec![("n", sm.len() as f64), ("min", MIN_SAMPLES as f64)], text: format!("PCA needs at least {MIN_SAMPLES} samples, there are {}", sm.len()) });
    }
    if matrix.is_empty() {
        return user("err.stat.pca.nofeatures", "no feature in the table");
    }
    if sm.iter().any(|&c| c >= ncols(matrix)) || groups.map_or(false, |g| g.len() != roles.len()) {
        return bad("index out of bounds: the table has fewer columns than files");
    }
    let mut steps = vec![step("table", matrix.len())];
    let mut keep = vec![true; matrix.len()];
    let has_blank = roles.iter().any(|r| r == "blank");
    if has_blank {
        let bf = blank_filter(matrix, roles, o.factor, &o.stat, o.min_detect)?;
        keep.iter_mut().enumerate().for_each(|(f, k)| *k = bf.kept.contains(&f));
    }
    steps.push(step("blank", keep.iter().filter(|k| **k).count()).b("applied", has_blank));
    let x: Vec<Vec<f64>> = matrix.iter().map(|r| sm.iter().map(|&c| r[c]).collect()).collect();
    let floor = o.min_intensity.max(f64::MIN_POSITIVE);
    keep.iter_mut().zip(&x).for_each(|(k, r)| *k = *k && r.iter().cloned().fold(f64::NEG_INFINITY, f64::max) >= o.min_intensity);
    steps.push(step("intensity", keep.iter().filter(|k| **k).count()).n("min", o.min_intensity));
    keep.iter_mut().zip(&x).for_each(|(k, r)| *k = *k && r.iter().filter(|&&v| v >= floor).count() as i64 >= o.min_samples);
    steps.push(step("presence", keep.iter().filter(|k| **k).count()).i("min_samples", o.min_samples));
    let idx: Vec<usize> = (0..matrix.len()).filter(|&f| keep[f]).collect();
    if idx.len() < 2 {
        return user("err.stat.pca.nofeatures", "fewer than two features survive the filters");
    }
    let mut filled: Vec<Vec<f64>> = idx.iter().map(|&f| x[f].clone()).collect();
    let n_imp = impute(&mut filled);
    analyse(&sm, idx, filled, n_imp, steps, groups, o)
}

/// Steps 3-6 of `pca_analysis`: transform, scaling, SVD, stability, ellipses.
fn analyse(sm: &[usize], idx: Vec<usize>, filled: Vec<Vec<f64>>, n_imp: usize, steps: Vec<Obj>, groups: Option<&[Option<String>]>, o: &PcaOpts) -> R<Value> {
    let n = sm.len();
    let live: Vec<usize> = (0..idx.len()).filter(|&f| {
        let z: Vec<f64> = filled[f].iter().map(|&v| if o.transform == "log" { v.log2() } else { v }).collect();
        let m = psum(&z) / n as f64;
        psum(&z.iter().map(|v| (v - m) * (v - m)).collect::<Vec<_>>()) / (n as f64 - 1.0) > 0.0
    }).collect();
    if live.len() < 2 {
        return user("err.stat.pca.nofeatures", "fewer than two features vary between the samples");
    }
    let p = live.len();
    let zt: Vec<f64> = (0..n).flat_map(|j| live.iter().map(|&f| (j, f)).collect::<Vec<_>>()).map(|(j, f)| if o.transform == "log" { filled[f][j].log2() } else { filled[f][j] }).collect();
    let full = fit(&scale(&zt, n, p, &o.scaling), n, p, o.ncomp);
    let (stab_pc, ks) = loo_stability(&zt, n, p, &o.scaling, &full, 2);
    let stab: Vec<f64> = (0..p).map(|f| {
        let w: Vec<f64> = (0..ks).map(|j| full.loadings[f * full.k + j].powi(2)).collect();
        let sw: f64 = w.iter().sum();
        if sw > 0.0 { w.iter().zip(&stab_pc[f * ks..]).map(|(a, b)| a * b).sum::<f64>() / sw.max(1e-300) } else { stab_pc[f * ks..(f + 1) * ks].iter().cloned().fold(f64::INFINITY, f64::min) }
    }).collect();
    let ell = groups.map_or(Obj::new().done(), |g| ellipses(g, sm, &full, n));
    let features = Value::Arr(live.iter().map(|&f| int(idx[f] as i64)).collect());
    Ok(Obj::new().put("samples", Value::Arr(sm.iter().map(|&i| int(i as i64)).collect())).put("features", features).put("scores", rows(&full.scores, full.k))
        .put("loadings", rows(&full.loadings, full.k)).put("explained", floats(&full.expl[..full.k])).put("stability", floats(&stab)).put("stability_pc", rows(&stab_pc, ks))
        .put("ellipses", ell).put("steps", Value::Arr(steps.into_iter().map(Obj::done).collect())).i("imputed", n_imp as i64)
        .put("transform", text(&o.transform)).put("scaling", text(&o.scaling)).i("n_samples", n as i64).i("n_features", p as i64).done())
}
