//! Analog traces of an mzML (pump pressure, flow...) and the delay between two detectors in series: `mzlab/analog.py`
//! (`pressure_to_bar`, `reduce_minmax`, `detector_delay`) and `Item.analog` of `mzlab/explore.py`.

use crate::json::{round, Out};

const TO_BAR: [(&str, f64); 9] = [
    ("megapascal", 10.0),
    ("mpa", 10.0),
    ("kilopascal", 0.01),
    ("kpa", 0.01),
    ("pascal", 1e-5),
    ("pa", 1e-5),
    ("psi", 0.0689476),
    ("pound", 0.0689476),
    ("bar", 1.0),
];
/// No HPLC pump reaches this: values above it come from a unit written ten times too small.
const BAR_MAX: f64 = 2000.0;

/// (values in bar, true when the unit had to be corrected by a factor of ten).
pub fn pressure_to_bar(y: &[f64], unit: &str) -> (Vec<f64>, bool) {
    let u = unit.trim().to_lowercase();
    let f = TO_BAR.iter().find(|(n, _)| *n == u).or_else(|| TO_BAR.iter().find(|(n, _)| u.contains(n))).map_or(1.0, |x| x.1);
    let out: Vec<f64> = y.iter().map(|v| v * f).collect();
    let top = out.iter().filter(|v| !v.is_nan()).map(|v| v.abs()).fold(0.0, f64::max);
    if top > BAR_MAX {
        return (out.iter().map(|v| v / 10.0).collect(), true);
    }
    (out, false)
}

/// (t, y) with at most about 2 n points: per interval of equal width the lowest and the highest value, in time order.
pub fn reduce_minmax(t: &[f64], y: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    if t.len() <= 2 * n || n < 1 {
        return (t.to_vec(), y.to_vec());
    }
    let step = t.len() as f64 / n as f64;
    let edge = |i: usize| if i == n { t.len() } else { (i as f64 * step) as usize };
    let (mut tt, mut yy) = (Vec::new(), Vec::new());
    for i in 0..n {
        let (a, b) = (edge(i), edge(i + 1));
        if b <= a {
            continue;
        }
        let seg = &y[a..b];
        let pick = |better: &dyn Fn(f64, f64) -> bool| {
            let mut k = 0;
            for (j, v) in seg.iter().enumerate() {
                if seg[k].is_nan() {
                    break;
                }
                if v.is_nan() || better(*v, seg[k]) {
                    k = j;
                }
            }
            a + k
        };
        let (lo, hi) = (pick(&|v, m| v < m), pick(&|v, m| v > m));
        let mut ks = vec![lo, hi];
        ks.sort();
        ks.dedup();
        for k in ks {
            tt.push(t[k]);
            yy.push(y[k]);
        }
    }
    (tt, yy)
}

/// `{"i", "kind", "id", "unit", "t", "y", "unit_note"?}` of one analog trace, reduced for drawing.
pub fn analog_json(i: i64, kind: &str, id: &str, unit: &str, t: &[f64], y: &[f64], n: usize) -> String {
    let (mut y, mut unit, mut note) = (y.to_vec(), unit.to_string(), None);
    if kind == "pressure" {
        let (v, fixed) = pressure_to_bar(&y, &unit);
        note = fixed.then(|| std::mem::take(&mut unit));
        (y, unit) = (v, "bar".into());
    }
    let (t, y) = reduce_minmax(t, &y, n);
    let mut o = Out::default();
    o.raw("{\"i\":").int(i).raw(",\"kind\":").str(kind).raw(",\"id\":").str(id).raw(",\"unit\":").str(&unit).raw(",\"t\":").rounded(&t, 4).raw(",\"y\":").rounded(&y, 3);
    if let Some(u) = note {
        o.raw(",\"unit_note\":{\"key\":\"analog.unit.tenfold\",\"params\":{\"unit\":").str(&u).raw("}}");
    }
    o.raw("}");
    o.0
}

// ---------------------------------------------------------------- delay between two detectors in series

/// `np.interp(x, xp, fp)` for increasing `xp`.
fn interp(x: f64, xp: &[f64], fp: &[f64]) -> f64 {
    let n = xp.len();
    if x < xp[0] {
        return fp[0];
    }
    if x > xp[n - 1] {
        return fp[n - 1];
    }
    let j = xp.partition_point(|v| *v <= x) - 1;
    if j == n - 1 || xp[j] == x {
        return fp[j];
    }
    let slope = (fp[j + 1] - fp[j]) / (xp[j + 1] - xp[j]);
    slope * (x - xp[j]) + fp[j]
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    crate::sortc::by(&mut s, f64::total_cmp);
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

/// `np.interp(np.arange(t0, t1, dt), t, y)`.
fn grid(t: &[f64], y: &[f64], t0: f64, t1: f64, dt: f64) -> Vec<f64> {
    let n = ((t1 - t0) / dt).ceil().max(0.0) as usize;
    let step = (t0 + dt) - t0;
    (0..n).map(|i| interp(t0 + i as f64 * step, t, y)).collect()
}

/// The trace without its slow baseline (running median) and scaled to unit spread: only the peaks are left to compare.
fn flatten(y: &[f64], width: usize) -> Vec<f64> {
    let k = (width | 1).max(3);
    let h = k / 2;
    let pad: Vec<f64> = (0..y.len() + 2 * h).map(|i| y[i.saturating_sub(h).min(y.len() - 1)]).collect();
    let idx: Vec<f64> = (0..y.len()).step_by(8).map(|i| i as f64).collect();
    let med: Vec<f64> = idx.iter().map(|&i| median(&pad[i as usize..i as usize + k])).collect();
    let z: Vec<f64> = y.iter().enumerate().map(|(i, v)| v - interp(i as f64, &idx, &med)).collect();
    let mean = z.iter().sum::<f64>() / z.len() as f64;
    let s = (z.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / z.len() as f64).sqrt();
    if s > 0.0 {
        z.iter().map(|v| v / s).collect()
    } else {
        z
    }
}

fn tops(v: &[f64], k: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (1..v.len().saturating_sub(1)).filter(|&j| v[j] > v[j - 1] && v[j] >= v[j + 1] && v[j] > 2.0).collect();
    crate::sortc::by(&mut idx, |a, b| v[*b].total_cmp(&v[*a]));
    idx.truncate(k);
    idx
}

pub struct Delay {
    pub delay: f64,
    pub peaks: usize,
    pub corr: f64,
}

/// How much later the trace B (mass spectrometer, in series after the PDA A) shows the same peaks. `None` when the traces share too little time.
pub fn detector_delay(ta: &[f64], ya: &[f64], tb: &[f64], yb: &[f64]) -> Option<Delay> {
    let max_lag = 0.6;
    if ta.len() < 20 || tb.len() < 20 {
        return None;
    }
    let t0 = ta[0].max(tb[0]) + max_lag;
    let t1 = ta[ta.len() - 1].min(tb[tb.len() - 1]) - max_lag;
    if t1 - t0 < 1.0 {
        return None;
    }
    let step = |t: &[f64]| median(&t.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>());
    let dt = step(tb).max(step(ta)).max(0.002) / 2.0;
    let a = grid(ta, ya, t0 - max_lag, t1 + max_lag, dt);
    let b = grid(tb, yb, t0 - max_lag, t1 + max_lag, dt);
    let w = (0.5 / dt) as usize;
    let (a, b) = (flatten(&a, w), flatten(&b, w));
    let n = (max_lag / dt) as usize;
    let (lo, hi) = (n, a.len() - n);
    let aa = &a[lo..hi];
    let norm = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
    let den = norm(aa) * norm(&b[lo..hi]) + 1e-12;
    let cc: Vec<f64> = (0..=2 * n).map(|l| aa.iter().zip(&b[lo + l - n..hi + l - n]).map(|(x, y)| x * y).sum::<f64>() / den).collect();
    let mut i = 0;
    for (j, v) in cc.iter().enumerate() {
        if *v > cc[i] {
            i = j;
        }
    }
    let off = if i > 0 && i < cc.len() - 1 {
        let d = cc[i - 1] - 2.0 * cc[i] + cc[i + 1];
        if d != 0.0 {
            0.5 * (cc[i - 1] - cc[i + 1]) / d
        } else {
            0.0
        }
    } else {
        0.0
    };
    let delay = (i as f64 - n as f64 + off) * dt;
    let (pa, pb) = (tops(aa, 8), tops(&b[lo..hi], 40));
    let shift = (delay / dt).round_ties_even() as i64;
    let peaks = pa.iter().filter(|&&j| pb.iter().any(|&q| (j as i64 + shift - q as i64).abs() <= 2)).count();
    Some(Delay { delay: round(delay, 4), peaks, corr: round(cc[i], 3) })
}
