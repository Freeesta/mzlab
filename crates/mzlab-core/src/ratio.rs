//! The ratio F/P inside the peak of the precursor and its constancy between samples: a port of `ionfamily.ratio_fit` and
//! `ionfamily.ratio_constancy` (weighted total least squares through the origin, Theil-Sen, residual diagnostics, Cochran Q).

use crate::json::Value;
use crate::jsonb::{floats, Obj};
use crate::rng::Pcg64;
use crate::signal::median;
use crate::similarity::block_indices;
use crate::stats::{chi2_sf, psum, theil_sen, wpearson};
use crate::tracestat::{argmin, geomspace, linspace, mean, prep, std};

/// Slope `r` of `F = r P` through the origin minimising `sum (F - rP)^2 / (sF^2 + r^2 sP^2)` (grid search, refined once).
pub fn deming_origin(p: &[f64], f: &[f64], sp: &[f64], sf: &[f64]) -> f64 {
    let pp = psum(&p.iter().map(|v| v * v).collect::<Vec<_>>());
    let r0 = psum(&f.iter().zip(p).map(|(a, b)| a * b).collect::<Vec<_>>()) / pp.max(1e-300);
    if r0 <= 0.0 {
        return 0.0;
    }
    let mut grid: Vec<f64> = geomspace(1.0 / 60.0, 60.0, 240)
        .into_iter()
        .map(|g| r0 * g)
        .collect();
    let mut best = 0.0;
    for _ in 0..2 {
        let s: Vec<f64> = grid
            .iter()
            .map(|&r| {
                let t: Vec<f64> = (0..p.len())
                    .map(|i| (f[i] - r * p[i]).powi(2) / (sf[i].powi(2) + r * r * sp[i].powi(2)))
                    .collect();
                psum(&t)
            })
            .collect();
        let k = argmin(&s);
        best = grid[k];
        let (lo, hi) = (grid[k.saturating_sub(1)], grid[(k + 1).min(grid.len() - 1)]);
        grid = linspace(lo, hi, 120);
    }
    best
}

/// Fit `F = r P` in the peak window `window = (first, end)` (indices of `rt`, `end` exclusive: Python's `[i_lo:i_hi + 1]`) with the fragment trace `x` and the precursor trace `p`.
/// `{"ok": false}` when there are too few points.
pub fn ratio_fit(x: &[f64], p: &[f64], window: (usize, usize), n_boot: usize, seed: u32) -> Value {
    let (cv, min_rel) = (0.1, 0.2);
    let (xc, _, nx) = prep(x);
    let (pc, _, npn) = prep(p);
    let cut = |v: &'_ [f64]| -> std::ops::Range<usize> {
        window.0.min(v.len())..window.1.min(v.len()).max(window.0.min(v.len()))
    };
    let (pw, fw) = (&pc[cut(&pc)], &xc[cut(&xc)]);
    let fail = Obj::new().b("ok", false).done();
    let pmax = pw.iter().cloned().fold(f64::MIN, f64::max);
    if pw.len() < 4 || pmax <= 0.0 {
        return fail;
    }
    let keep: Vec<usize> = (0..pw.len()).filter(|&i| pw[i] >= min_rel * pmax).collect();
    if keep.len() < 4 {
        return fail;
    }
    let pv: Vec<f64> = keep.iter().map(|&i| pw[i]).collect();
    let fv: Vec<f64> = keep.iter().map(|&i| fw[i]).collect();
    let n = pv.len();
    let sp: Vec<f64> = pv
        .iter()
        .map(|v| (npn * npn + (cv * v).powi(2)).sqrt())
        .collect();
    let sf: Vec<f64> = fv
        .iter()
        .map(|v| (nx * nx + (cv * v).powi(2)).sqrt())
        .collect();
    let r = deming_origin(&pv, &fv, &sp, &sf);
    let (ts, ic) = theil_sen(&pv, &fv, 80);
    let ratios: Vec<f64> = fv.iter().zip(&pv).map(|(a, b)| a / b).collect();
    let med = median(&ratios);
    let mad = 1.4826 * median(&ratios.iter().map(|v| (v - med).abs()).collect::<Vec<_>>());
    let fit: Vec<f64> = pv.iter().map(|v| r * v).collect();
    let resid: Vec<f64> = fv.iter().zip(&fit).map(|(a, b)| a - b).collect();
    let ss_res = psum(&resid.iter().map(|v| v * v).collect::<Vec<_>>());
    let fm = mean(&fv);
    let ss_tot = psum(&fv.iter().map(|v| (v - fm) * (v - fm)).collect::<Vec<_>>());
    let acf1 = if resid.len() > 4 {
        wpearson(&resid[..n - 1], &resid[1..], None)
    } else {
        f64::NAN
    };
    let mut rng = Pcg64::new(seed);
    let bl = 2.max(n / 5);
    let rs: Vec<f64> = (0..n_boot)
        .map(|_| {
            let idx = block_indices(&mut rng, n, bl);
            let g = |v: &[f64]| idx.iter().map(|&i| v[i]).collect::<Vec<f64>>();
            deming_origin(&g(&pv), &g(&fv), &g(&sp), &g(&sf))
        })
        .collect();
    let fmax = fv.iter().cloned().fold(f64::MIN, f64::max);
    let rpmax = r * pv.iter().cloned().fold(f64::MIN, f64::max);
    let rm = mean(&ratios);
    let pts = Obj::new()
        .put("p", floats(&pv))
        .put("f", floats(&fv))
        .put("fit", floats(&fit))
        .done();
    Obj::new()
        .b("ok", true)
        .i("n", n as i64)
        .n("r_tls", r)
        .n("r_se", if rs.is_empty() { f64::NAN } else { std(&rs) })
        .n("r_ts", ts)
        .n("intercept_ts", ic)
        .n(
            "intercept_rel",
            if rpmax > 0.0 { ic / rpmax } else { f64::NAN },
        )
        .n("r_median", med)
        .n(
            "cv_ratio",
            if rm != 0.0 {
                std(&ratios) / rm.abs()
            } else {
                f64::NAN
            },
        )
        .n(
            "cv_ratio_mad",
            if med != 0.0 {
                mad / med.abs()
            } else {
                f64::NAN
            },
        )
        .n(
            "r2",
            if ss_tot > 0.0 {
                1.0 - ss_res / ss_tot
            } else {
                f64::NAN
            },
        )
        .n("resid_rel", (ss_res / n as f64).sqrt() / fmax.max(1e-12))
        .n("resid_acf1", acf1)
        .put("points", pts)
        .done()
}

/// Is the ratio the same in every sample? `ratios` = `r_tls` per sample, `ses` = their standard errors (Cochran Q ~ chi2(K-1)).
pub fn ratio_constancy(ratios: &[f64], ses: &[f64], labels: &[String]) -> Value {
    let ok: Vec<usize> = (0..ratios.len())
        .filter(|&i| ratios[i].is_finite())
        .collect();
    let k = ok.len() as i64;
    let mut out = Obj::new().i("k", k);
    if ok.len() < 2 {
        return out
            .n("q", f64::NAN)
            .n("p", f64::NAN)
            .n("cv", f64::NAN)
            .n("range_rel", f64::NAN)
            .done();
    }
    let r: Vec<f64> = ok.iter().map(|&i| ratios[i]).collect();
    let se: Vec<f64> = ok
        .iter()
        .zip(&r)
        .map(|(&i, v)| ses[i].max(1e-3 * v.abs().max(1e-12)))
        .collect();
    let w: Vec<f64> = se.iter().map(|s| 1.0 / (s * s)).collect();
    let rw = psum(&w.iter().zip(&r).map(|(a, b)| a * b).collect::<Vec<_>>()) / psum(&w);
    let q = psum(
        &w.iter()
            .zip(&r)
            .map(|(a, b)| a * (b - rw) * (b - rw))
            .collect::<Vec<_>>(),
    );
    let rm = mean(&r);
    let n = r.len() as f64;
    let sd1 = (psum(&r.iter().map(|v| (v - rm) * (v - rm)).collect::<Vec<_>>()) / (n - 1.0)).sqrt();
    let (mx, mn) = (
        r.iter().cloned().fold(f64::MIN, f64::max),
        r.iter().cloned().fold(f64::MAX, f64::min),
    );
    out = out
        .n("r_weighted", rw)
        .n("q", q)
        .i("df", r.len() as i64 - 1)
        .n("p", chi2_sf(q, r.len() as u32 - 1))
        .n("cv", if rm != 0.0 { sd1 / rm.abs() } else { f64::NAN })
        .n(
            "range_rel",
            if rm != 0.0 {
                (mx - mn) / rm.abs()
            } else {
                f64::NAN
            },
        )
        .put("ratios", floats(&r));
    if !labels.is_empty() {
        out = out.put(
            "labels",
            Value::Arr(ok.iter().map(|&i| crate::jsonb::text(&labels[i])).collect()),
        );
    }
    out.done()
}
