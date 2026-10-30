//! Analog traces of an mzML (pump pressure, flow...) and the delay between two detectors in series: `mzlab/analog.py`
//! (`pressure_to_bar`, `reduce_minmax`, `detector_delay`) and `Item.analog` of `mzlab/explore.py`.

use crate::json::{round, Out};

// `as` saturates and maps NaN to 0, which is what every caller below wants.
#[allow(clippy::cast_possible_truncation)]
fn to_idx(x: f64) -> usize {
    x as usize
}

#[allow(clippy::cast_possible_truncation)]
fn to_i64(x: f64) -> i64 {
    x as i64
}

/// Bar in one unit of a pressure, by the exact name the file gives (compared in lower case).
const TO_BAR: [(&str, f64); 12] = [
    ("megapascal", 10.0),
    ("mpa", 10.0),
    ("kilopascal", 0.01),
    ("kpa", 0.01),
    ("pascal", 1e-5),
    ("pa", 1e-5),
    ("psi", 0.0689476),
    ("pound per square inch", 0.0689476),
    ("pounds per square inch", 0.0689476),
    ("millibar", 0.001),
    ("mbar", 0.001),
    ("bar", 1.0),
];

/// The values in bar when the file declares a pressure unit; `None` when it declares none or another one (e.g. `number of detector counts`, which only says
/// "a number"): the scale is then unknown and the values are shown as they are.
pub fn pressure_to_bar(y: &[f64], unit: &str) -> Option<Vec<f64>> {
    let u = unit.trim().to_lowercase();
    let f = TO_BAR.iter().find(|(n, _)| *n == u)?.1;
    Some(y.iter().map(|v| v * f).collect())
}

/// (t, y) with at most about 2 n points: per interval of equal width the lowest and the highest value, in time order.
pub fn reduce_minmax(t: &[f64], y: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    if t.len() <= 2 * n || n < 1 {
        return (t.to_vec(), y.to_vec());
    }
    let step = t.len() as f64 / n as f64;
    let edge = |i: usize| {
        if i == n {
            t.len()
        } else {
            to_idx(i as f64 * step)
        }
    };
    let (mut tt, mut yy) = (Vec::new(), Vec::new());
    for i in 0..n {
        let (a, b) = (edge(i), edge(i + 1));
        if b <= a {
            continue;
        }
        let Some(seg) = y.get(a..b) else { continue };
        let pick = |better: &dyn Fn(f64, f64) -> bool| {
            let (mut k, mut m) = (0, seg.first().copied().unwrap_or(f64::NAN));
            for (j, v) in seg.iter().enumerate() {
                if m.is_nan() {
                    break;
                }
                if v.is_nan() || better(*v, m) {
                    (k, m) = (j, *v);
                }
            }
            a + k
        };
        let (lo, hi) = (pick(&|v, m| v < m), pick(&|v, m| v > m));
        let mut ks = vec![lo, hi];
        crate::sortc::slice_ord(&mut ks);
        ks.dedup();
        for k in ks {
            if let (Some(&tk), Some(&yk)) = (t.get(k), y.get(k)) {
                tt.push(tk);
                yy.push(yk);
            }
        }
    }
    (tt, yy)
}

/// `{"i", "kind", "id", "unit", "t", "y"}` of one analog trace, reduced for drawing; a pressure with a declared pressure unit is in bar, otherwise as the file has it.
pub fn analog_json(
    i: i64,
    kind: &str,
    id: &str,
    unit: &str,
    t: &[f64],
    y: &[f64],
    n: usize,
) -> String {
    let (mut y, mut unit) = (y.to_vec(), unit.to_string());
    if kind == "pressure" {
        if let Some(v) = pressure_to_bar(&y, &unit) {
            (y, unit) = (v, "bar".into());
        }
    }
    let (t, y) = reduce_minmax(t, &y, n);
    let mut o = Out::default();
    o.raw("{\"i\":")
        .int(i)
        .raw(",\"kind\":")
        .str(kind)
        .raw(",\"id\":")
        .str(id)
        .raw(",\"unit\":")
        .str(&unit)
        .raw(",\"t\":")
        .rounded(&t, 4)
        .raw(",\"y\":")
        .rounded(&y, 3);
    o.raw("}");
    o.0
}

// ---------------------------------------------------------------- delay between two detectors in series

/// `np.interp(x, xp, fp)` for increasing `xp`.
fn interp(x: f64, xp: &[f64], fp: &[f64]) -> f64 {
    let n = xp.len();
    let (Some(&x0), Some(&f0), Some(&xl), Some(&fl)) =
        (xp.first(), fp.first(), xp.last(), fp.last())
    else {
        return f64::NAN;
    };
    if x < x0 {
        return f0;
    }
    if x > xl {
        return fl;
    }
    let j = xp.partition_point(|v| *v <= x).saturating_sub(1);
    let (Some(&xj), Some(&fj)) = (xp.get(j), fp.get(j)) else {
        return fl;
    };
    if j + 1 >= n || xj == x {
        return fj;
    }
    let (Some(&xn), Some(&fnx)) = (xp.get(j + 1), fp.get(j + 1)) else {
        return fj;
    };
    (fnx - fj) / (xn - xj) * (x - xj) + fj
}

fn median(v: &[f64]) -> f64 {
    crate::signal::median_or(v, f64::NAN)
}

/// `np.interp(np.arange(t0, t1, dt), t, y)`.
fn grid(t: &[f64], y: &[f64], t0: f64, t1: f64, dt: f64) -> Vec<f64> {
    let n = to_idx(((t1 - t0) / dt).ceil().max(0.0));
    let step = (t0 + dt) - t0;
    (0..n).map(|i| interp(t0 + i as f64 * step, t, y)).collect()
}

/// The trace without its slow baseline (running median) and scaled to unit spread: only the peaks are left to compare.
fn flatten(y: &[f64], width: usize) -> Vec<f64> {
    let k = (width | 1).max(3);
    let h = k / 2;
    let pad: Vec<f64> = (0..y.len() + 2 * h)
        .map(|i| {
            y.get(i.saturating_sub(h).min(y.len().saturating_sub(1)))
                .copied()
                .unwrap_or(f64::NAN)
        })
        .collect();
    let starts: Vec<usize> = (0..y.len()).step_by(8).collect();
    let idx: Vec<f64> = starts.iter().map(|&i| i as f64).collect();
    let med: Vec<f64> = starts
        .iter()
        .map(|&i| pad.get(i..i + k).map_or(f64::NAN, median))
        .collect();
    let z: Vec<f64> = y
        .iter()
        .enumerate()
        .map(|(i, v)| v - interp(i as f64, &idx, &med))
        .collect();
    let mean = z.iter().sum::<f64>() / z.len() as f64;
    let s = (z.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / z.len() as f64).sqrt();
    if s > 0.0 {
        z.iter().map(|v| v / s).collect()
    } else {
        z
    }
}

fn tops(v: &[f64], k: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = v
        .windows(3)
        .enumerate()
        .filter_map(|(j, w)| match w {
            [a, b, c] if b > a && b >= c && *b > 2.0 => Some(j + 1),
            _ => None,
        })
        .collect();
    let at = |i: &usize| v.get(*i).copied().unwrap_or(f64::NAN);
    crate::sortc::by(&mut idx, |a, b| at(b).total_cmp(&at(a)));
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
    let (Some(&a0), Some(&b0), Some(&a1), Some(&b1)) =
        (ta.first(), tb.first(), ta.last(), tb.last())
    else {
        return None;
    };
    let t0 = a0.max(b0) + max_lag;
    let t1 = a1.min(b1) - max_lag;
    if t1 - t0 < 1.0 {
        return None;
    }
    let step = |t: &[f64]| {
        median(
            &t.windows(2)
                .filter_map(|w| match w {
                    [p, q] => Some(q - p),
                    _ => None,
                })
                .collect::<Vec<_>>(),
        )
    };
    let dt = step(tb).max(step(ta)).max(0.002) / 2.0;
    let a = grid(ta, ya, t0 - max_lag, t1 + max_lag, dt);
    let b = grid(tb, yb, t0 - max_lag, t1 + max_lag, dt);
    let w = to_idx(0.5 / dt);
    let (a, b) = (flatten(&a, w), flatten(&b, w));
    let n = to_idx(max_lag / dt);
    let (lo, hi) = (n, a.len().saturating_sub(n));
    let (aa, bb) = (a.get(lo..hi)?, b.get(lo..hi)?);
    let norm = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
    let den = norm(aa) * norm(bb) + 1e-12;
    let cc: Vec<f64> = (0..=2 * n)
        .map(|l| {
            b.get(lo + l - n..hi + l - n)
                .map(|bs| aa.iter().zip(bs).map(|(x, y)| x * y).sum::<f64>() / den)
        })
        .collect::<Option<_>>()?;
    let (mut i, mut best) = (0, *cc.first()?);
    for (j, v) in cc.iter().enumerate() {
        if *v > best {
            (i, best) = (j, *v);
        }
    }
    let off = i
        .checked_sub(1)
        .and_then(|j| cc.get(j..j + 3))
        .map_or(0.0, |w| match *w {
            [p, c, nx] => {
                let d = p - 2.0 * c + nx;
                if d != 0.0 {
                    0.5 * (p - nx) / d
                } else {
                    0.0
                }
            }
            _ => 0.0,
        });
    let delay = (i as f64 - n as f64 + off) * dt;
    let (pa, pb) = (tops(aa, 8), tops(bb, 40));
    let shift = to_i64((delay / dt).round_ties_even());
    let peaks = pa
        .iter()
        .filter(|&&j| pb.iter().any(|&q| (j as i64 + shift - q as i64).abs() <= 2))
        .count();
    Some(Delay {
        delay: round(delay, 4),
        peaks,
        corr: round(best, 3),
    })
}
