//! Behaviour of an ion across samples at different treatment times, compared with the parent: a port of `ionfamily.kinetics`
//! (rank correlations, first-order decay, formation-decay A -> B -> C, residual bootstrap, AICc). Few points: every number comes
//! with its uncertainty and a warning.

use crate::json::Value;
use crate::jsonb::{floats, int, msg, Obj};
use crate::rng::Pcg64;
use crate::stats::{kendall_tau, perm_pvalue, psum, theil_sen, wspearman, Stat};
use crate::tracestat::{argmin, geomspace, linspace, percentile};

fn sum_sq(a: impl Iterator<Item = f64>) -> f64 {
    psum(&a.map(|v| v * v).collect::<Vec<_>>())
}

/// Best `(scale, parameter, sse)` over a grid of decay constants for the profiles `shape(k)` (closed-form scale per k).
fn best_on_grid(
    ks: &[f64],
    yy: &[f64],
    shape: &dyn Fn(f64) -> Vec<f64>,
    clamp0: bool,
) -> (f64, f64, f64) {
    let rows: Vec<(f64, f64)> = ks
        .iter()
        .map(|&k| {
            let e = shape(k);
            let ee = psum(&e.iter().map(|v| v * v).collect::<Vec<_>>());
            let mut a =
                psum(&e.iter().zip(yy).map(|(a, b)| a * b).collect::<Vec<_>>()) / ee.max(1e-300);
            if clamp0 {
                a = a.max(0.0);
            }
            (a, sum_sq(yy.iter().zip(&e).map(|(y, v)| y - a * v)))
        })
        .collect();
    let i = argmin(&rows.iter().map(|r| r.1).collect::<Vec<_>>());
    (rows[i].0, ks[i], rows[i].1)
}

fn solve_decay(t: &[f64], yy: &[f64]) -> (f64, f64, f64) {
    let span = (t.iter().cloned().fold(f64::MIN, f64::max)
        - t.iter().cloned().fold(f64::MAX, f64::min))
    .max(1e-9);
    let mut ks = vec![0.0];
    ks.extend(geomspace(1e-3 / span, 50.0 / span, 160));
    let shape = |k: f64| t.iter().map(|x| (-k * x).exp()).collect::<Vec<f64>>();
    for _ in 0..2 {
        let rows: Vec<f64> = ks
            .iter()
            .map(|&k| best_on_grid(&[k], yy, &shape, false).2)
            .collect();
        let i = argmin(&rows);
        ks = linspace(ks[i.saturating_sub(1)], ks[(i + 1).min(ks.len() - 1)], 60);
        ks.retain(|&k| k >= 0.0);
        if ks.is_empty() {
            ks = vec![0.0];
        }
    }
    best_on_grid(&ks, yy, &shape, false)
}

/// First-order decay `y = A exp(-k t)`, `k >= 0`, with residual-bootstrap 95% interval of `k`.
pub fn fit_decay(t: &[f64], y: &[f64], n_boot: usize, seed: u32) -> Value {
    let (a, k, sse) = solve_decay(t, y);
    let fit: Vec<f64> = t.iter().map(|x| a * (-k * x).exp()).collect();
    let res: Vec<f64> = y.iter().zip(&fit).map(|(a, b)| a - b).collect();
    let mut rng = Pcg64::new(seed);
    let ks: Vec<f64> = (0..n_boot)
        .map(|_| {
            let yb: Vec<f64> = rng
                .choice_replace(res.len(), res.len())
                .iter()
                .zip(&fit)
                .map(|(&i, f)| f + res[i])
                .collect();
            solve_decay(t, &yb).1
        })
        .collect();
    Obj::new()
        .n("A", a)
        .n("k", k)
        .n("sse", sse)
        .n(
            "half_life",
            if k > 0.0 {
                std::f64::consts::LN_2 / k
            } else {
                f64::INFINITY
            },
        )
        .i("n", t.len() as i64)
        .b("reliable", t.len() >= 5)
        .put(
            "k_ci",
            floats(&[percentile(&ks, 2.5), percentile(&ks, 97.5)]),
        )
        .done()
}

/// `k1 / (k2 - k1) (e^{-k1 t} - e^{-k2 t})` (the limit `k1 t e^{-k1 t}` when `k2 ~ k1`).
fn consecutive_shape(t: &[f64], k1: f64, k2: f64) -> Vec<f64> {
    let d = k2 - k1;
    if d.abs() < 1e-6 * k1.max(1e-9) {
        return t.iter().map(|x| k1 * x * (-k1 * x).exp()).collect();
    }
    t.iter()
        .map(|x| k1 / d * ((-k1 * x).exp() - (-k2 * x).exp()))
        .collect()
}

fn solve_consecutive(t: &[f64], k1: f64, k2s: &[f64], yy: &[f64]) -> (f64, f64, f64) {
    best_on_grid(k2s, yy, &|k2| consecutive_shape(t, k1, k2), true)
}

/// Formation-decay of an intermediate B in A -> B -> C with `k1` known (the decay of the parent), scale and `k2` free.
pub fn fit_consecutive(t: &[f64], y: &[f64], k1: f64, n_boot: usize, seed: u32) -> (Obj, f64) {
    let span = (t.iter().cloned().fold(f64::MIN, f64::max)
        - t.iter().cloned().fold(f64::MAX, f64::min))
    .max(1e-9);
    let k2s = geomspace(1e-3 / span, 50.0 / span, 500);
    let (s, k2, sse) = solve_consecutive(t, k1, &k2s, y);
    let fit: Vec<f64> = consecutive_shape(t, k1, k2).iter().map(|v| s * v).collect();
    let res: Vec<f64> = y.iter().zip(&fit).map(|(a, b)| a - b).collect();
    let mut rng = Pcg64::new(seed);
    let kk: Vec<f64> = (0..n_boot)
        .map(|_| {
            let yb: Vec<f64> = rng
                .choice_replace(res.len(), res.len())
                .iter()
                .zip(&fit)
                .map(|(&i, f)| f + res[i])
                .collect();
            solve_consecutive(t, k1, &k2s, &yb).1
        })
        .collect();
    let t_max = if k2 > 0.0 && (k2 - k1).abs() > 1e-9 {
        (k2 / k1).ln() / (k2 - k1)
    } else {
        1.0 / k1
    };
    let o = Obj::new()
        .n("s", s)
        .n("k2", k2)
        .n("sse", sse)
        .put(
            "k2_ci",
            floats(&[percentile(&kk, 2.5), percentile(&kk, 97.5)]),
        )
        .i("n", t.len() as i64)
        .n("t_max", t_max);
    (o, sse)
}

fn aicc(sse: f64, n: usize, k: usize) -> f64 {
    if (n as i64) - (k as i64) - 1 <= 0 || sse <= 0.0 {
        return f64::NAN;
    }
    let (n, k) = (n as f64, k as f64);
    n * (sse / n).ln() + 2.0 * k + 2.0 * k * (k + 1.0) / (n - k - 1.0)
}

fn get(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(|x| x.f64()).unwrap_or(f64::NAN)
}

/// Kinetics of an ion against the parent: areas `parent_area` and `ion_area` at treatment `times` (any order, sorted here).
pub fn kinetics(times: &[f64], parent_area: &[f64], ion_area: &[f64]) -> Value {
    let mut o: Vec<usize> = (0..times.len()).collect();
    o.sort_by(|&a, &b| times[a].total_cmp(&times[b]));
    let t: Vec<f64> = o.iter().map(|&i| times[i]).collect();
    let p: Vec<f64> = o.iter().map(|&i| parent_area[i]).collect();
    let x: Vec<f64> = o.iter().map(|&i| ion_area[i]).collect();
    let n = t.len();
    let mut warnings = Vec::new();
    let mut out = Obj::new().i("n", n as i64);
    if n < 3 {
        warnings.push(msg("of.warn.fewSamples3", &[]));
        return out.put("warnings", Value::Arr(warnings)).done();
    }
    if n < 5 {
        warnings.push(msg("of.warn.fewSamplesCi", &[("n", int(n as i64))]));
    }
    let mx = |v: &[f64]| v.iter().cloned().fold(f64::MIN, f64::max).max(1e-12);
    let (pn, xn): (Vec<f64>, Vec<f64>) = (
        p.iter().map(|v| v / mx(&p)).collect(),
        x.iter().map(|v| v / mx(&x)).collect(),
    );
    let ratio: Vec<f64> = p
        .iter()
        .zip(&x)
        .map(|(a, b)| if *a > 0.0 { b / a } else { f64::NAN })
        .collect();
    out = out
        .put("times", floats(&t))
        .put("parent_norm", floats(&pn))
        .put("ion_norm", floats(&xn))
        .put("ratio", floats(&ratio))
        .n("spearman_parent", wspearman(&x, &p, None))
        .n("kendall_parent", kendall_tau(&x, &p))
        .n(
            "p_spearman_parent",
            perm_pvalue(Stat::Spearman, &x, &p, 4000, 0),
        )
        .n("spearman_time", wspearman(&t, &x, None))
        .n("kendall_time", kendall_tau(&t, &x));
    let (sl, _) = theil_sen(&t, &xn, 80);
    out = out.n("slope_norm_per_min", sl).i(
        "slope_sign",
        if sl.is_finite() {
            sl.signum() as i64 * (sl != 0.0) as i64
        } else {
            0
        },
    );
    let dp = fit_decay(&t, &pn, 120, 0);
    let k1 = get(&dp, "k");
    out = out.put("parent_decay", dp);
    let c = psum(&x.iter().zip(&p).map(|(a, b)| a * b).collect::<Vec<_>>())
        / psum(&p.iter().map(|v| v * v).collect::<Vec<_>>()).max(1e-300);
    let sse = sum_sq(x.iter().zip(&p).map(|(a, b)| a - c * b));
    let mut models = Obj::new().put(
        "tracks_parent",
        Obj::new()
            .n("c", c)
            .n("sse", sse)
            .i("n_par", 1)
            .n("aicc", aicc(sse, n, 1))
            .done(),
    );
    let de = fit_decay(&t, &x, 120, 0);
    models = models.put(
        "decay",
        Obj::new()
            .n("A", get(&de, "A"))
            .n("k", get(&de, "k"))
            .put("k_ci", de.get("k_ci").cloned().unwrap_or(Value::Null))
            .n("sse", get(&de, "sse"))
            .i("n_par", 2)
            .n("aicc", aicc(get(&de, "sse"), n, 2))
            .done(),
    );
    if k1 > 0.0 {
        let (fc, sse) = fit_consecutive(&t, &x, k1, 300, 0);
        models = models.put(
            "formation_decay",
            fc.i("n_par", 2).n("aicc", aicc(sse, n, 2)).done(),
        );
    }
    out.put("models", models.done())
        .put("warnings", Value::Arr(warnings))
        .done()
}
