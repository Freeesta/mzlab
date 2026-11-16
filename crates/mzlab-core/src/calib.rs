//! Calibration lines of `quant.curve@1`: weighted least squares y = a + b·x (+ c·x²) with weights 1, 1/x, 1/x², 1/y, 1/y², the intercept optionally
//! forced to zero, the inverse prediction (back-calculation), R² and the residual standard deviations. Solved by Householder QR of the √w-scaled design
//! with x divided by its largest |x|, never the normal equations: a quadratic over three decades keeps its digits, a rank-deficient design is refused.
//! Sources: Miller & Miller, Statistics and Chemometrics for Analytical Chemistry, 6th ed. (2010) §5.4 and §5.10; Almeida, Castel-Branco, Falcão,
//! J. Chromatogr. B 774 (2002) 215-222 (weighting schemes 1/x, 1/x², 1/y, 1/y²). LOD/LOQ (ICH Q2(R2), 2023) are computed by the step text from `s_yx`.

/// The weighting schemes, by the names the step and its output use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    One,
    InvX,
    InvX2,
    InvY,
    InvY2,
}

impl Weight {
    pub const NAMES: [&'static str; 5] = ["1", "1/x", "1/x2", "1/y", "1/y2"];
    const ALL: [Weight; 5] = [
        Weight::One,
        Weight::InvX,
        Weight::InvX2,
        Weight::InvY,
        Weight::InvY2,
    ];

    pub fn parse(s: &str) -> Option<Weight> {
        Self::NAMES
            .iter()
            .position(|n| *n == s)
            .map(|i| Self::ALL[i])
    }

    pub fn name(self) -> &'static str {
        Self::NAMES[Self::ALL.iter().position(|w| *w == self).unwrap_or(0)]
    }

    /// The weight of a point; `None` where the scheme is undefined (x or y ≤ 0 under a 1/x or 1/y scheme): such a point cannot enter the fit.
    pub fn of(self, x: f64, y: f64) -> Option<f64> {
        let inv = |v: f64, sq: bool| (v > 0.0).then(|| if sq { 1.0 / (v * v) } else { 1.0 / v });
        match self {
            Weight::One => Some(1.0),
            Weight::InvX => inv(x, false),
            Weight::InvX2 => inv(x, true),
            Weight::InvY => inv(y, false),
            Weight::InvY2 => inv(y, true),
        }
    }
}

/// The calibration model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Linear,
    Quadratic,
}

/// Why a fit has no line, as the key the output names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitError {
    /// Fewer points than coefficients.
    TooFewPoints,
    /// Fewer distinct concentrations than the model needs (or all at zero).
    Singular,
    /// A coordinate or weight that is not a finite number, or a weight ≤ 0.
    NotFinite,
}

impl FitError {
    pub fn key(self) -> &'static str {
        match self {
            FitError::TooFewPoints => "err.flow.curve.points",
            FitError::Singular => "err.flow.curve.singular",
            FitError::NotFinite => "err.flow.curve.value",
        }
    }
}

/// A fitted line: y = a + b·x + c·x² (c = 0 for a linear model, a = 0 when forced through zero).
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    /// Points and coefficients of the fit.
    pub n: usize,
    pub p: usize,
    /// Weighted R² = 1 − Σw·r² / Σw·(y − ȳw)², centred also through zero; `None` when every y is the same.
    pub r2: Option<f64>,
    /// Residual standard deviation of the unweighted residuals, √(Σr² / (n − p)), in signal units: the σ of ICH Q2(R2); `None` when n = p.
    pub s_yx: Option<f64>,
    /// The same with the weights, √(Σw·r² / (n − p)).
    pub s_w: Option<f64>,
}

impl Fit {
    pub fn predict(&self, x: f64) -> f64 {
        self.a + self.b * x + self.c * x * x
    }

    /// The concentration of a signal. Linear: (y − a) / b. Quadratic: the root of c·x² + b·x + (a − y) = 0 that tends to (y − a) / b as c → 0,
    /// written 2(y − a) / (b + sign(b)·√(b² + 4c(y − a))) so it never cancels; `None` without a real root or with a zero slope.
    pub fn inverse(&self, y: f64) -> Option<f64> {
        let d = y - self.a;
        let x = if self.c == 0.0 {
            d / self.b
        } else {
            let disc = self.b * self.b + 4.0 * self.c * d;
            if disc < 0.0 {
                return None;
            }
            let q = self.b + self.b.signum() * disc.sqrt();
            2.0 * d / q
        };
        x.is_finite().then_some(x)
    }
}

/// A column whose remaining norm after the earlier reflections is below this fraction of its own norm is a linear combination of them.
const RANK_TOL: f64 = 1e-10;

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Least squares of `cols` (column-major, each of length n) against `rhs` by Householder QR; `None` when a column is (numerically) dependent.
fn qr_solve(mut cols: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let p = cols.len();
    let norms: Vec<f64> = cols.iter().map(|c| dot(c, c).sqrt()).collect();
    for k in 0..p {
        let rest = dot(&cols[k][k..], &cols[k][k..]).sqrt();
        if rest.is_nan() || rest <= RANK_TOL * norms[k] {
            return None;
        }
        let alpha = if cols[k][k] > 0.0 { -rest } else { rest };
        let mut v = cols[k][k..].to_vec();
        v[0] -= alpha;
        let vv = dot(&v, &v);
        for col in cols.iter_mut().skip(k) {
            let s = 2.0 * dot(&v, &col[k..]) / vv;
            col[k..].iter_mut().zip(&v).for_each(|(c, vi)| *c -= s * vi);
        }
        let s = 2.0 * dot(&v, &rhs[k..]) / vv;
        rhs[k..].iter_mut().zip(&v).for_each(|(c, vi)| *c -= s * vi);
    }
    let mut beta = vec![0.0; p];
    for k in (0..p).rev() {
        let mut s = rhs[k];
        for j in k + 1..p {
            s -= cols[j][k] * beta[j];
        }
        beta[k] = s / cols[k][k];
    }
    Some(beta)
}

/// u⁰, u¹ or u² as plain products, the same bits on every target.
fn pow(u: f64, e: i32) -> f64 {
    match e {
        0 => 1.0,
        1 => u,
        _ => u * u,
    }
}

/// Weighted least squares of `y` on `x` with the weights `w` (> 0).
pub fn fit(x: &[f64], y: &[f64], w: &[f64], model: Model, zero: bool) -> Result<Fit, FitError> {
    let n = x.len();
    if y.len() != n || w.len() != n {
        return Err(FitError::NotFinite);
    }
    let finite = x.iter().chain(y).chain(w).all(|v| v.is_finite());
    if !finite || w.iter().any(|v| *v <= 0.0) {
        return Err(FitError::NotFinite);
    }
    let powers: &[i32] = match (model, zero) {
        (Model::Linear, false) => &[0, 1],
        (Model::Linear, true) => &[1],
        (Model::Quadratic, false) => &[0, 1, 2],
        (Model::Quadratic, true) => &[1, 2],
    };
    let p = powers.len();
    if n < p {
        return Err(FitError::TooFewPoints);
    }
    let xs = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    if xs == 0.0 {
        return Err(FitError::Singular);
    }
    let sw: Vec<f64> = w.iter().map(|v| v.sqrt()).collect();
    let cols: Vec<Vec<f64>> = powers
        .iter()
        .map(|&e| (0..n).map(|i| sw[i] * pow(x[i] / xs, e)).collect())
        .collect();
    let rhs: Vec<f64> = (0..n).map(|i| sw[i] * y[i]).collect();
    let beta = qr_solve(cols, rhs).ok_or(FitError::Singular)?;
    let mut coef = [0.0f64; 3];
    for (&e, b) in powers.iter().zip(&beta) {
        coef[e as usize] = b / pow(xs, e);
    }
    let mut f = Fit {
        a: coef[0],
        b: coef[1],
        c: coef[2],
        n,
        p,
        r2: None,
        s_yx: None,
        s_w: None,
    };
    let r: Vec<f64> = (0..n).map(|i| y[i] - f.predict(x[i])).collect();
    let sum_w: f64 = w.iter().sum();
    let mean = dot(w, y) / sum_w;
    let sst: f64 = (0..n).map(|i| w[i] * (y[i] - mean) * (y[i] - mean)).sum();
    let ssr_w: f64 = (0..n).map(|i| w[i] * r[i] * r[i]).sum();
    if sst > 0.0 {
        f.r2 = Some(1.0 - ssr_w / sst);
    }
    if n > p {
        let dof = (n - p) as f64;
        f.s_yx = Some((dot(&r, &r) / dof).sqrt());
        f.s_w = Some((ssr_w / dof).sqrt());
    }
    Ok(f)
}

#[cfg(test)]
#[path = "calib_tests.rs"]
mod tests;
