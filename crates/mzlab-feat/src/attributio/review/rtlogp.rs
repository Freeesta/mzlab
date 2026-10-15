// Ported from Attributio src/attributio/review/rtlogp.py.
//! Retention time versus logP, a calibrated plausibility check: a warning aid, never applied. A relative check CALIBRATED on the campaign's own confident
//! identifications spots gross outliers and ranks the candidates of a Level 3; it does not tell isomers apart. The calibrants are the Level 1 features and the
//! robust Level 2a ones of one polarity; ONE logP source per fit (the one most calibrants have). The logP values come from the sources of the campaign (no
//! network): a missing value is "not available" (`None`), never mixed in from another source.

use super::isotopes::round_to;
use std::collections::BTreeMap;

/// Minimum calibrants per polarity to trust a fit.
pub const MIN_N: usize = 15;

/// The organic fraction a compound eluting at `rt` has met: the largest %B of the gradient (`[[t, %B], ...]`, piecewise linear) up to `rt - dwell`.
pub fn organic_fraction(rt: f64, gradient: &[Vec<f64>], dwell: f64) -> f64 {
    let x = rt - dwell;
    let interp = {
        let g = gradient;
        if x <= g[0][0] {
            g[0][1]
        } else if x >= g[g.len() - 1][0] {
            g[g.len() - 1][1]
        } else {
            let j = g.iter().position(|p| p[0] > x).unwrap_or(g.len() - 1);
            let (a, b) = (&g[j - 1], &g[j]);
            a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0])
        }
    };
    gradient.iter().filter(|p| p[0] <= x).fold(interp, |m, p| m.max(p[1]))
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

fn mad_of(res: &[f64]) -> f64 {
    let m = median(res);
    1.4826 * median(&res.iter().map(|r| (r - m).abs()).collect::<Vec<_>>())
}

fn r2_of(y: &[f64], res: &[f64]) -> f64 {
    let ym = y.iter().sum::<f64>() / y.len() as f64;
    let ss: f64 = y.iter().map(|v| (v - ym).powi(2)).sum();
    if ss > 0.0 {
        1.0 - res.iter().map(|r| r * r).sum::<f64>() / ss
    } else {
        0.0
    }
}

/// The kind of a fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitKind {
    Line,
    Mlr,
}

/// `y = a + b·x` (`b` has one coefficient for a line, one per descriptor for the multiple regression).
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub a: f64,
    pub b: Vec<f64>,
    pub mad: f64,
    pub r2: f64,
    pub n: usize,
    pub kind: FitKind,
}

/// OLS `y = a + b x` with the residual MAD (× 1.4826) and r²; `None` below 3 points or with a constant `x`.
pub fn fit_line(x: &[f64], y: &[f64]) -> Option<Fit> {
    let ptp = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - v.iter().cloned().fold(f64::INFINITY, f64::min);
    if x.len() < 3 || ptp(x) == 0.0 {
        return None;
    }
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let sxx: f64 = x.iter().map(|v| (v - mx).powi(2)).sum();
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let b = sxy / sxx;
    let a = my - b * mx;
    let res: Vec<f64> = x.iter().zip(y).map(|(xv, yv)| yv - (a + b * xv)).collect();
    Some(Fit { a, b: vec![b], mad: mad_of(&res), r2: round_to(r2_of(y, &res), 3), n: x.len(), kind: FitKind::Line })
}

/// Minimum-norm least squares `X β ≈ y` by one-sided Jacobi SVD (the cutoff of `numpy.linalg.lstsq` with `rcond=None`).
fn lstsq(x: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    let (m, n) = (x.len(), x[0].len());
    let mut u: Vec<Vec<f64>> = (0..n).map(|j| (0..m).map(|i| x[i][j]).collect()).collect(); // columns
    let mut v: Vec<Vec<f64>> = (0..n).map(|j| (0..n).map(|i| if i == j { 1.0 } else { 0.0 }).collect()).collect();
    for _ in 0..60 {
        let mut rotated = false;
        for p in 0..n {
            for q in p + 1..n {
                let (mut a, mut b, mut c) = (0.0, 0.0, 0.0);
                for i in 0..m {
                    a += u[p][i] * u[p][i];
                    b += u[q][i] * u[q][i];
                    c += u[p][i] * u[q][i];
                }
                if c.abs() <= 1e-15 * (a * b).sqrt() || c == 0.0 {
                    continue;
                }
                rotated = true;
                let zeta = (b - a) / (2.0 * c);
                let t = zeta.signum() / (zeta.abs() + (1.0 + zeta * zeta).sqrt());
                let (cs, sn) = (1.0 / (1.0 + t * t).sqrt(), t / (1.0 + t * t).sqrt());
                for i in 0..m {
                    let (up, uq) = (u[p][i], u[q][i]);
                    u[p][i] = cs * up - sn * uq;
                    u[q][i] = sn * up + cs * uq;
                }
                for i in 0..n {
                    let (vp, vq) = (v[p][i], v[q][i]);
                    v[p][i] = cs * vp - sn * vq;
                    v[q][i] = sn * vp + cs * vq;
                }
            }
        }
        if !rotated {
            break;
        }
    }
    let sv: Vec<f64> = u.iter().map(|c| c.iter().map(|z| z * z).sum::<f64>().sqrt()).collect();
    let smax = sv.iter().cloned().fold(0.0, f64::max);
    let cutoff = f64::EPSILON * m.max(n) as f64 * smax;
    let mut beta = vec![0.0; n];
    for j in 0..n {
        if sv[j] <= cutoff {
            continue;
        }
        let proj: f64 = (0..m).map(|i| u[j][i] * y[i]).sum::<f64>() / (sv[j] * sv[j]);
        for (i, b) in beta.iter_mut().enumerate() {
            *b += v[j][i] * proj;
        }
    }
    beta
}

/// Multiple linear regression `y = X b + a`; `None` with too few rows or a constant `y`.
pub fn fit_mlr(x: &[Vec<f64>], y: &[f64]) -> Option<Fit> {
    let ptp = y.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - y.iter().cloned().fold(f64::INFINITY, f64::min);
    if x.is_empty() || x.len() < x[0].len() + 3 || ptp == 0.0 {
        return None;
    }
    let xm: Vec<Vec<f64>> = x.iter().map(|r| std::iter::once(1.0).chain(r.iter().cloned()).collect()).collect();
    let beta = lstsq(&xm, y);
    let res: Vec<f64> = xm.iter().zip(y).map(|(r, yv)| yv - r.iter().zip(&beta).map(|(a, b)| a * b).sum::<f64>()).collect();
    Some(Fit { a: beta[0], b: beta[1..].to_vec(), mad: mad_of(&res), r2: round_to(r2_of(y, &res), 3), n: x.len(), kind: FitKind::Mlr })
}

/// The logP and descriptors of a structure, each `None` when the sources of the campaign do not give it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LogPValues {
    pub xlogp3: Option<f64>,
    pub crippen: Option<f64>,
    pub class: Option<String>,
    pub mw: Option<f64>,
    pub tpsa: Option<f64>,
    pub hbd: Option<f64>,
    pub hba: Option<f64>,
    pub nrot: Option<f64>,
}

impl LogPValues {
    fn get(&self, source: &str) -> Option<f64> {
        if source == "xlogp3" {
            self.xlogp3
        } else {
            self.crippen
        }
    }

    fn vector(&self, x: f64) -> Vec<f64> {
        let z = |v: Option<f64>| v.unwrap_or(0.0);
        vec![x, z(self.mw), z(self.tpsa), z(self.nrot), z(self.hbd), z(self.hba)]
    }
}

/// logP of the structures of a campaign: values by the first block of the InChIKey, read from the sources (no network).
#[derive(Clone, Debug, Default)]
pub struct LogP {
    pub xl: BTreeMap<String, f64>,
}

impl LogP {
    /// The values of a structure by its first InChIKey block; descriptors the campaign does not give stay `None`.
    pub fn values(&self, ik1: Option<&str>) -> LogPValues {
        LogPValues { xlogp3: ik1.and_then(|k| self.xl.get(k)).map(|v| round_to(*v, 2)), ..Default::default() }
    }
}

/// A calibrant: the logP values of a confident identification and the organic fraction (or RT) at which it eluted.
#[derive(Clone, Debug, Default)]
pub struct CalPoint {
    pub logp: LogPValues,
    pub y: Option<f64>,
}

/// The model of a polarity.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub source: &'static str,
    pub n: usize,
    pub fit: Option<Fit>,
    pub per_class: BTreeMap<String, Option<Fit>>,
    pub window: Option<f64>,
    pub note: String,
}

fn fit_model(pts: &[&CalPoint], source: &str) -> Option<Fit> {
    let xs: Vec<Vec<f64>> = pts.iter().map(|p| p.logp.vector(p.logp.get(source).unwrap_or(0.0))).collect();
    let ys: Vec<f64> = pts.iter().map(|p| p.y.unwrap_or(0.0)).collect();
    fit_mlr(&xs, &ys).or_else(|| fit_line(&xs.iter().map(|r| r[0]).collect::<Vec<_>>(), &ys))
}

/// Chooses ONE logP source (the one most calibrants have), fits globally and per class where `min_n` allows.
pub fn calibrate(points: &[CalPoint], k: f64, floor: f64, min_n: usize) -> Model {
    let n3 = points.iter().filter(|p| p.logp.xlogp3.is_some()).count();
    let nc = points.iter().filter(|p| p.logp.crippen.is_some()).count();
    let source = if n3 as f64 >= f64::max(nc as f64 * 0.8, min_n as f64) { "xlogp3" } else { "crippen" };
    let use_: Vec<&CalPoint> = points.iter().filter(|p| p.logp.get(source).is_some() && p.y.is_some()).collect();
    if use_.len() < min_n {
        return Model {
            source,
            n: use_.len(),
            fit: None,
            per_class: BTreeMap::new(),
            window: None,
            note: format!("not enough calibrants ({} < {})", use_.len(), min_n),
        };
    }
    let fit = fit_model(&use_, source);
    let mut per = BTreeMap::new();
    for cls in ["acid", "base", "neutral"] {
        let sub: Vec<&CalPoint> = use_.iter().cloned().filter(|p| p.logp.class.as_deref() == Some(cls)).collect();
        if sub.len() >= min_n {
            per.insert(cls.to_string(), fit_model(&sub, source));
        }
    }
    let window = fit.as_ref().map(|f| f64::max(k * f.mad, floor));
    let mut note = if fit.as_ref().map_or(false, |f| f.kind == FitKind::Mlr) {
        "Multi-descriptor model (Retip-like) active.".to_string()
    } else {
        "Basic logP line active.".to_string()
    };
    if per.is_empty() {
        note.push_str(" One fit for every class (not enough calibrants per ionisation class).");
    }
    Model { source, n: use_.len(), fit, per_class: per, window, note }
}

/// Is the observed y consistent with the logP of a structure? `ok` is `None` ("n.a.") with no model, no logP of the model's source or no observation.
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub logp: Option<f64>,
    pub pred: Option<f64>,
    pub resid: Option<f64>,
    pub ok: Option<bool>,
    pub class: Option<String>,
}

pub fn check(model: &Model, logp: &LogPValues, y_obs: Option<f64>, k: f64, floor: f64) -> Check {
    let x = logp.get(model.source);
    let none = || Check { logp: x, pred: None, resid: None, ok: None, class: logp.class.clone() };
    let (Some(fit), Some(xv), Some(y)) = (model.fit.as_ref(), x, y_obs) else {
        return none();
    };
    let f = logp.class.as_ref().and_then(|c| model.per_class.get(c)).and_then(|f| f.as_ref()).unwrap_or(fit);
    let win = f64::max(k * f.mad, floor);
    let pred = match f.kind {
        FitKind::Mlr => f.a + f.b.iter().zip(logp.vector(xv)).map(|(b, v)| b * v).sum::<f64>(),
        FitKind::Line => f.a + f.b[0] * xv,
    };
    Check { logp: x, pred: Some(round_to(pred, 2)), resid: Some(round_to(y - pred, 2)), ok: Some((y - pred).abs() <= win), class: logp.class.clone() }
}
