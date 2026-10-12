//! Profile similarity between the trace of an ion X and that of a candidate precursor P: a port of `ionfamily.profile_similarity`.
//! Measurements only, never a verdict; the Python engine is the reference (same windows, same random draws).

use crate::json::Value;
use crate::jsonb::{floats, int, msg, num, Obj};
use crate::peaks::MIN_SCANS_RELIABLE;
use crate::rng::Pcg64;
use crate::signal::savgol_d;
use crate::stats::{wpearson, wspearman};
use crate::tracestat::{cosine, detect, percentile, pick_peak, prep, roll, PeakX};

fn peak_obj(p: &PeakX, snr: bool) -> Value {
    let mut o = Obj::new()
        .n("apex_rt", p.e.apex_rt)
        .n("apex_sigma", p.e.apex_sigma)
        .n("height", p.p.height)
        .n("area", p.p.area)
        .i("lo", p.p.lo as i64)
        .i("hi", p.p.hi as i64)
        .n("fwhm_min", p.e.fwhm_min)
        .n("fwhm_scans", p.e.fwhm_scans)
        .i("n_scans", p.p.n_scans as i64);
    if snr {
        o = o.n("snr", p.p.snr);
    }
    o.done()
}

/// The indices `idx` of a block-bootstrap resample: `ceil(nw / bl)` blocks of `bl` consecutive points, cut to `nw`.
pub fn block_indices(rng: &mut Pcg64, nw: usize, bl: usize) -> Vec<usize> {
    let nb = nw.div_ceil(bl);
    let mut idx = Vec::with_capacity(nb * bl);
    for a in rng.integers((nw.saturating_sub(bl)).max(1), nb) {
        idx.extend(a..(a + bl).min(nw));
    }
    idx.truncate(nw);
    idx
}

fn pick(v: &[f64], idx: &[usize]) -> Vec<f64> {
    idx.iter().map(|&i| v[i]).collect()
}

/// Fraction of the time-shifted copies of X that correlate with P at least as well as the observed alignment.
fn shift_pvalue(
    rng: &mut Pcg64,
    xc: &[f64],
    pw: &[f64],
    sl: (usize, usize),
    obs: f64,
    n_shift: usize,
) -> f64 {
    let n = xc.len() as i64;
    if !obs.is_finite() {
        return f64::NAN;
    }
    let l = n / 2;
    let mut sh: Vec<i64> = (-l..=l).filter(|s| s.abs() >= 2).collect();
    if sh.len() > n_shift {
        sh = rng
            .choice_no_replace(sh.len(), n_shift)
            .into_iter()
            .map(|i| sh[i])
            .collect();
    }
    let (mut cnt, mut used) = (0usize, 0usize);
    for s in sh {
        let c = wpearson(&roll(xc, s)[sl.0..=sl.1], pw, None);
        if c.is_finite() {
            used += 1;
            if c >= obs - 1e-12 {
                cnt += 1;
            }
        }
    }
    if used > 0 {
        (1 + cnt) as f64 / (1 + used) as f64
    } else {
        f64::NAN
    }
}

/// 95% interval of Pearson by block bootstrap.
fn corr_ci(rng: &mut Pcg64, xw: &[f64], pw: &[f64], pk: &PeakX, n_boot: usize) -> [f64; 2] {
    let nw = xw.len();
    let bl = if pk.e.fwhm_scans > 0.0 {
        (pk.e.fwhm_scans / 2.0).round_ties_even().max(3.0) as usize
    } else {
        3
    };
    let mut cs = Vec::new();
    for _ in 0..n_boot {
        let idx = block_indices(rng, nw, bl);
        let c = wpearson(&pick(xw, &idx), &pick(pw, &idx), None);
        if c.is_finite() {
            cs.push(c);
        }
    }
    if cs.len() > 10 {
        [percentile(&cs, 2.5), percentile(&cs, 97.5)]
    } else {
        [f64::NAN; 2]
    }
}

/// All the profile-similarity evidence of the ion trace `x` against the precursor trace `p` (same RT axis `rt`).
pub fn profile_similarity(
    rt: &[f64],
    x: &[f64],
    p: &[f64],
    p_apex_rt: Option<f64>,
    n_boot: usize,
    n_shift: usize,
    seed: u32,
) -> Obj {
    let n = rt.len();
    let mut res = Obj::new()
        .i("n_scans", 0)
        .b("reliable", false)
        .b("x_has_peak", false);
    let mut warnings = Vec::new();
    if n < 7 {
        warnings.push(msg("of.warn.shortWindow", &[]));
        return res.put("warnings", Value::Arr(warnings));
    }
    let (pc, ps, _) = prep(p);
    let (xc, xs, _) = prep(x);
    let (detp, detx) = (detect(rt, p), detect(rt, x));
    let near = p_apex_rt
        .map(|t| crate::tracestat::argmin(&rt.iter().map(|v| (v - t).abs()).collect::<Vec<_>>()));
    let Some(pk) = pick_peak(&detp, near, None) else {
        warnings.push(msg("of.warn.noParentPeak", &[]));
        return res.put("warnings", Value::Arr(warnings));
    };
    let xk = pick_peak(
        &detx,
        Some(pk.p.apex_i),
        Some(3f64.max((pk.e.fwhm_scans * 2.0).trunc())),
    );
    res.set("x_has_peak", Value::Bool(xk.is_some()));
    let (mut lo, mut hi) = (pk.p.lo, pk.p.hi);
    if let Some(k) = xk {
        lo = lo.min(k.p.lo);
        hi = hi.max(k.p.hi);
    }
    let nw = hi - lo + 1;
    res.set("n_scans", int(nw as i64));
    res.set("window", floats(&[rt[lo], rt[hi]]));
    if nw < MIN_SCANS_RELIABLE {
        warnings.push(msg("of.warn.fewScans", &[("n", int(nw as i64))]));
    }
    if xk.is_none() {
        warnings.push(msg("of.warn.ionNoPeak", &[]));
    }
    let (xw, pw) = (&xc[lo..=hi], &pc[lo..=hi]);
    let (xsw, psw) = (&xs[lo..=hi], &ps[lo..=hi]);
    let mx = xsw.iter().cloned().fold(f64::MIN, f64::max).max(1e-12);
    let mp = psw.iter().cloned().fold(f64::MIN, f64::max).max(1e-12);
    let wts: Vec<f64> = xsw
        .iter()
        .zip(psw)
        .map(|(a, b)| 0.5 * (a / mx + b / mp) + 1e-3)
        .collect();
    let obs = wpearson(xw, pw, None);
    res.set("pearson", num(obs));
    res.set("pearson_w", num(wpearson(xw, pw, Some(&wts))));
    res.set("spearman", num(wspearman(xw, pw, None)));
    res.set("spearman_w", num(wspearman(xw, pw, Some(&wts))));
    let (dxs, dps) = (savgol_d(&xc, 7, 2, 1), savgol_d(&pc, 7, 2, 1));
    res.set(
        "deriv_corr",
        num(wpearson(&dxs[lo..=hi], &dps[lo..=hi], None)),
    );
    res.set("cosine", num(cosine(xw, pw)));
    match xk {
        Some(k) => {
            res.set("apex_diff_s", num((k.e.apex_rt - pk.e.apex_rt) * 60.0));
            res.set(
                "apex_diff_sigma_s",
                num(k.e.apex_sigma.hypot(pk.e.apex_sigma) * 60.0),
            );
            res.set(
                "fwhm_ratio",
                num(if pk.e.fwhm_min > 0.0 {
                    k.e.fwhm_min / pk.e.fwhm_min
                } else {
                    f64::NAN
                }),
            );
            res.set("x_snr", num(k.p.snr));
        }
        None => {
            for k in ["apex_diff_s", "apex_diff_sigma_s", "fwhm_ratio"] {
                res.set(k, Value::Null);
            }
            res.set("x_snr", num(0.0));
        }
    }
    res.set("p_apex_rt", num(pk.e.apex_rt));
    let lags: Vec<i64> = (-2..=2).collect();
    let xcs: Vec<f64> = lags
        .iter()
        .map(|&l| wpearson(&roll(&xc, -l)[lo..=hi], pw, None))
        .collect();
    let xa: Vec<f64> = xcs
        .iter()
        .map(|&c| if c.is_finite() { c } else { -2.0 })
        .collect();
    let k = crate::tracestat::argmin(&xa.iter().map(|v| -v).collect::<Vec<_>>());
    let best = xa[k];
    res.set(
        "xcorr_lags",
        floats(&lags.iter().map(|&l| l as f64).collect::<Vec<_>>()),
    );
    res.set("xcorr", floats(&xcs));
    res.set("lag_best", int(lags[k]));
    res.set("xcorr_best", num(if best > -2.0 { best } else { f64::NAN }));
    let mut rng = Pcg64::new(seed);
    res.set(
        "p_shift",
        num(shift_pvalue(&mut rng, &xc, pw, (lo, hi), obs, n_shift)),
    );
    let ci = if nw >= 8 && obs.is_finite() {
        corr_ci(&mut rng, xw, pw, pk, n_boot)
    } else {
        [f64::NAN; 2]
    };
    res.set("corr_ci", floats(&ci));
    res.set(
        "reliable",
        Value::Bool(nw >= MIN_SCANS_RELIABLE && xk.is_some() && pk.e.reliable),
    );
    res.set("p_peak", peak_obj(pk, false));
    if let Some(k) = xk {
        res.set("x_peak", peak_obj(k, true));
    }
    res.put("warnings", Value::Arr(warnings))
}
