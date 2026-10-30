//! Edges of the peak under a click on a low-resolution trace (`autoEdges` of the page, the one-click integration of an XIC or MRM trace).
//! Quadratic Savitzky-Golay smoothing that follows the peak width, noise from the residual, edges where the smoothed trace falls below the local
//! baseline plus a fraction of the height or turns up again. Used by `lr_edges` of mzlab-wasm (page and desktop) and `mzlab peaks-edges`.

use crate::sortc;

/// Parameters of the detector (the dialog «Peak detection» of the page, `PK`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LrParams {
    /// Baseline window around the apex (min).
    pub win: f64,
    /// Edge limit: this many noise sigmas above the local baseline.
    pub area_noise: f64,
    /// Edge limit: this percentage of the peak height above the local baseline.
    pub edge_frac: f64,
    /// Minimum S/N of the peak (0 = no check).
    pub sn_min: f64,
    pub noise: NoiseMode,
    /// Smallest half window of the smoothing (points).
    pub min_half: f64,
    /// A valley needs this percentage of the half window of rising points.
    pub multi: f64,
    /// Longest tail walked from the apex (min).
    pub tail: f64,
    /// Base width kept within +- this percentage of the reference peak (0 = free).
    pub constrain: f64,
    /// Region (min) the noise is measured in when `noise` is `Manual`.
    pub nr0: f64,
    pub nr1: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoiseMode {
    Mad,
    Rms,
    Manual,
}

impl Default for LrParams {
    fn default() -> Self {
        LrParams {
            win: 1.2,
            area_noise: 3.0,
            edge_frac: 3.0,
            sn_min: 0.0,
            noise: NoiseMode::Mad,
            min_half: 2.0,
            multi: 50.0,
            tail: 1.5,
            constrain: 0.0,
            nr0: 0.0,
            nr1: 0.0,
        }
    }
}

impl LrParams {
    /// The eleven numbers of the wasm and CLI interface, in the order of the fields (`noise`: 0 = MAD, 1 = RMS, 2 = manual region).
    pub fn from_slice(p: &[f64]) -> Option<LrParams> {
        if p.len() < 11 {
            return None;
        }
        let noise = match p[4] as i64 {
            1 => NoiseMode::Rms,
            2 => NoiseMode::Manual,
            _ => NoiseMode::Mad,
        };
        Some(LrParams {
            win: p[0],
            area_noise: p[1],
            edge_frac: p[2],
            sn_min: p[3],
            noise,
            min_half: p[5],
            multi: p[6],
            tail: p[7],
            constrain: p[8],
            nr0: p[9],
            nr1: p[10],
        })
    }
}

/// `Math.round` of the page: halves go up.
fn round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// Index of the point nearest to `v` in the sorted `xs` (a tie goes to the later one).
pub fn near_idx(xs: &[f64], v: f64) -> usize {
    if xs.len() < 2 {
        return 0;
    }
    let (mut lo, mut hi) = (0, xs.len() - 1);
    while hi - lo > 1 {
        let m = (lo + hi) >> 1;
        if xs[m] < v {
            lo = m;
        } else {
            hi = m;
        }
    }
    if (xs[lo] - v).abs() < (xs[hi] - v).abs() {
        lo
    } else {
        hi
    }
}

/// Quadratic Savitzky-Golay smoothing (`deriv` false) or first derivative (true) with half window `m`; points outside the trace repeat the end value.
pub fn sg_filter(y: &[f64], m: usize, deriv: bool) -> Vec<f64> {
    let n = y.len();
    let mf = m as f64;
    let mm = mf * (mf + 1.0) * (2.0 * mf + 1.0) / 3.0;
    let w: Vec<f64> = (0..=2 * m)
        .map(|i| {
            let j = i as f64 - mf;
            if deriv {
                j / mm
            } else {
                3.0 * (3.0 * mf * mf + 3.0 * mf - 1.0 - 5.0 * j * j)
                    / ((2.0 * mf - 1.0) * (2.0 * mf + 1.0) * (2.0 * mf + 3.0))
            }
        })
        .collect();
    (0..n)
        .map(|i| {
            let mut v = 0.0;
            if i >= m && i + m < n {
                for (k, wk) in w.iter().enumerate() {
                    v += wk * y[i + k - m];
                }
            } else {
                for (k, wk) in w.iter().enumerate() {
                    v += wk * y[(i + k).saturating_sub(m).min(n - 1)];
                }
            }
            v
        })
        .collect()
}

/// Standard deviation (n - 1) of the signal around the straight line fitted in the stretch [a, b] (a drift is not noise); `None` under 5 points.
pub fn noise_sd(xs: &[f64], ys: &[f64], a: f64, b: f64) -> Option<f64> {
    let (lo, hi) = (a.min(b), a.max(b));
    let (px, py): (Vec<f64>, Vec<f64>) = xs
        .iter()
        .zip(ys)
        .filter(|(x, _)| **x >= lo && **x <= hi)
        .map(|(x, y)| (*x, *y))
        .unzip();
    let n = px.len();
    if n < 5 {
        return None;
    }
    let nf = n as f64;
    let mx = px.iter().fold(0.0, |s, v| s + v) / nf;
    let my = py.iter().fold(0.0, |s, v| s + v) / nf;
    let (mut sxx, mut sxy) = (0.0, 0.0);
    for (x, y) in px.iter().zip(&py) {
        sxx += (x - mx).powi(2);
        sxy += (x - mx) * (y - my);
    }
    let m = if sxx != 0.0 { sxy / sxx } else { 0.0 };
    let ss = px
        .iter()
        .zip(&py)
        .fold(0.0, |s, (x, y)| s + (y - (my + m * (x - mx))).powi(2));
    Some((ss / (nf - 1.0)).sqrt())
}

fn sorted(v: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut o: Vec<f64> = v.collect();
    sortc::by(&mut o, |a, b| a.total_cmp(b));
    o
}

/// Coarse apex `m0` near the click and the half window `m` of the final smoothing: 30 % of the width at half height (so the window is about 60 % of it).
fn apex_and_window(
    y: &[f64],
    xs: &[f64],
    a0: usize,
    x: f64,
    dt: f64,
    pk: &LrParams,
) -> (usize, usize) {
    let nn = y.len();
    let co = sg_filter(y, round(0.03 / dt / 2.0).max(2.0) as usize, false);
    let lo = near_idx(xs, x - 0.25).saturating_sub(a0).min(nn - 1);
    let hi = near_idx(xs, x + 0.25).saturating_sub(a0).min(nn - 1);
    let mut m0 = lo;
    for j in lo..=hi {
        if co[j] > co[m0] {
            m0 = j;
        }
    }
    let cmin = co.iter().fold(f64::INFINITY, |m, v| {
        if v.is_nan() || m.is_nan() {
            f64::NAN
        } else {
            m.min(*v)
        }
    });
    let half = cmin + (co[m0] - cmin) / 2.0;
    let (mut hl, mut hr) = (m0, m0);
    while hl > 0 && co[hl] > half {
        hl -= 1;
    }
    while hr < nn - 1 && co[hr] > half {
        hr += 1;
    }
    (
        m0,
        pk.min_half.max(round((hr - hl) as f64 * 0.3).min(60.0)) as usize,
    )
}

/// Noise sigma: the region chosen by the student when set, else MAD or RMS of the sorted residual `res`.
fn noise_of(res: &[f64], xs: &[f64], ys: &[f64], pk: &LrParams) -> f64 {
    let manual = if pk.noise == NoiseMode::Manual && pk.nr1 > pk.nr0 {
        noise_sd(xs, ys, pk.nr0, pk.nr1)
    } else {
        None
    };
    match manual {
        Some(v) if v > 0.0 => v,
        _ => {
            let v = if pk.noise == NoiseMode::Rms {
                (res.iter().fold(0.0, |a, v| a + v * v) / (res.len() as f64).max(1.0)).sqrt()
            } else {
                1.4826 * res[res.len() >> 1]
            };
            v.max(1e-12)
        }
    }
}

/// Where the edge on one side of the apex `k` stops: under `lim`, or where the smoothed trace turns up again for `need` points, or after `maxw` points.
#[allow(clippy::too_many_arguments)]
fn walk(
    sm: &[f64],
    d: &[f64],
    k: usize,
    dir: i64,
    lim: f64,
    need: i64,
    dth: f64,
    maxw: f64,
) -> i64 {
    let (mut j, mut bad, n) = (k as i64, 0i64, sm.len() as i64);
    while j + dir >= 0 && j + dir < n && ((j - k as i64).abs() as f64) < maxw {
        let nx = j + dir;
        j = nx;
        if sm[nx as usize] <= lim {
            break;
        }
        let rising = if dir < 0 {
            d[nx as usize] < -dth
        } else {
            d[nx as usize] > dth
        };
        bad = if rising { bad + 1 } else { 0 };
        if bad >= need {
            j = nx - dir * need;
            break;
        }
    }
    j
}

/// Limits (min) of the peak at time `x` on the trace `xs`/`ys`, or `None` when its S/N is under `sn_min`. `reference` is the base width of the
/// peaks already integrated (0 = none); `mrm` marks a transition trace (a lower edge fraction on strong peaks, as for a very tall one).
pub fn lr_edges(
    xs: &[f64],
    ys: &[f64],
    x: f64,
    reference: f64,
    mrm: bool,
    pk: &LrParams,
) -> Option<(f64, f64)> {
    let n = xs.len().min(ys.len());
    let (xs, ys) = (&xs[..n], &ys[..n]);
    let ic = near_idx(xs, x);
    if n < 9 {
        return Some((xs[ic], xs[ic]));
    }
    let (hi_i, lo_i) = ((n - 1).min(ic + 50), ic.saturating_sub(50));
    let step = (xs[hi_i] - xs[lo_i])
        / (if hi_i == lo_i {
            1.0
        } else {
            (hi_i - lo_i) as f64
        });
    let dt = if step == 0.0 || step.is_nan() {
        0.01
    } else {
        step
    };
    let a0 = near_idx(xs, x - 2.0);
    let y = &ys[a0..=(n - 1).min(near_idx(xs, x + 2.0))];
    let nn = y.len();
    let (m0, m) = apex_and_window(y, xs, a0, x, dt, pk);
    let sm = sg_filter(y, m, false);
    let d = sg_filter(y, m, true);
    let from = m0.saturating_sub(m);
    let mut k = from;
    for j in from..=(nn - 1).min(m0 + m) {
        if sm[j] > sm[k] {
            k = j;
        }
    }
    let span = round(pk.win / dt).max(0.0) as usize;
    let (w0, w1) = (k.saturating_sub(span), (nn - 1).min(k + span));
    let res = sorted((w0..=w1).map(|j| (y[j] - sm[j]).abs()));
    let noise = noise_of(&res, xs, ys, pk);
    let low = sorted(sm[w0..=w1].iter().copied());
    let base = low[(low.len() as f64 * 0.1).floor() as usize];
    let h = (sm[k] - base).max(0.0);
    let frac = if mrm || (noise > 0.0 && h / noise > 800.0) {
        (pk.edge_frac / 100.0).min((5.0 * noise / h.max(1.0)).max(0.005))
    } else {
        pk.edge_frac / 100.0
    };
    let lim = base + (frac * h).max(pk.area_noise * noise);
    if pk.sn_min > 0.0 && h < pk.sn_min * noise {
        return None;
    }
    let need = round(m as f64 * pk.multi / 100.0).max(2.0) as i64;
    let dth = noise / (m as f64).max(1.0) * 0.3;
    let maxw = round(pk.tail / dt);
    let e0 = xs[a0
        + walk(&sm, &d, k, -1, lim, need, dth, maxw)
            .min(k as i64)
            .max(0) as usize];
    let e1 = xs[a0
        + walk(&sm, &d, k, 1, lim, need, dth, maxw)
            .max(k as i64)
            .min(nn as i64 - 1) as usize];
    Some(constrain(e0, e1, xs[a0 + k], reference, pk))
}

/// «Constrain the width»: the base width stays within +- `constrain` % of the reference peak, keeping the apex at the same relative position.
fn constrain(e0: f64, e1: f64, apex: f64, reference: f64, pk: &LrParams) -> (f64, f64) {
    if !(pk.constrain > 0.0 && reference > 0.0) {
        return (e0, e1);
    }
    let w = e1 - e0;
    let (lo, hi) = (
        reference * (1.0 - pk.constrain / 100.0),
        reference * (1.0 + pk.constrain / 100.0),
    );
    let w2 = hi.min(lo.max(w));
    if w > 0.0 && w2 != w {
        let a = apex - (apex - e0) / w * w2;
        return (a, a + w2);
    }
    (e0, e1)
}
