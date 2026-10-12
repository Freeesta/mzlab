//! The four shapes a product can follow over the treatment time, fitted with `bank::Bank`: formation `1 - exp(-k t)`, rise and fall
//! (Bateman A -> B -> C), flat and decay `exp(-k t)`. The shape is chosen by `AICc`; the report is the shape and the gap to the second best.

use super::bank::{aicc, grid, Bank, Hit, Row};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Formation = 0,
    Bateman = 1,
    Flat = 2,
    Decreasing = 3,
}

impl Form {
    pub const ALL: [Form; 4] = [Form::Formation, Form::Bateman, Form::Flat, Form::Decreasing];

    pub fn name(self) -> &'static str {
        ["formation", "rise_fall", "flat", "decay"][self as usize]
    }

    fn n_rates(self) -> usize {
        [1, 2, 0, 1][self as usize]
    }
}

/// One fitted shape: rate constants (1/min), scale in the square-root space of `y / ymax`, `AICc`.
#[derive(Clone, Copy, Debug)]
pub struct FormFit {
    pub form: Form,
    pub rates: [f64; 2],
    pub scale: f64,
    pub ymax: f64,
    pub sse: f64,
    pub aicc: f64,
}

/// Concentration of the intermediate B of A -> B -> C (parent 1, rates `k1`, `k2`) at time `t`; the limit `k1 t exp(-k1 t)` when the rates are equal.
pub fn bateman(k1: f64, k2: f64, t: f64) -> f64 {
    let d = k2 - k1;
    if d == 0.0 {
        k1 * t * libm::exp(-k1 * t)
    } else {
        k1 * libm::exp(-k1 * t) * (-libm::expm1(-d * t)) / d
    }
}

fn curve(r: &Row, t: f64) -> f64 {
    match r.group {
        0 => -libm::expm1(-r.p[0] * t),
        1 => bateman(r.p[0], r.p[1], t),
        2 => 1.0,
        _ => libm::exp(-r.p[0] * t),
    }
}

/// Model curves of the four shapes on a fixed vector of times; a fit can use any subset of the columns (leave one out, bootstrap).
pub struct FormBank {
    bank: Bank,
}

impl FormBank {
    pub fn new(times: &[f64]) -> FormBank {
        let g = grid();
        let mut rows = Vec::with_capacity(g.len() * (g.len() + 1) / 2 + 2 * g.len() + 1);
        let row = |group: u8, a: f64, b: f64| Row {
            group,
            p: [a, b, 0.0],
        };
        rows.extend(g.iter().map(|&k| row(0, k, 0.0)));
        for (i, &a) in g.iter().enumerate() {
            rows.extend(g[i..].iter().map(|&b| row(1, a, b))); // the shape is symmetric in its two rates: ascending pairs
        }
        rows.push(row(2, 0.0, 0.0));
        rows.extend(g.iter().map(|&k| row(3, k, 0.0)));
        let t = times.to_vec();
        FormBank {
            bank: Bank::new(rows, t.len(), |r, j| curve(r, t[j])),
        }
    }

    pub fn n_times(&self) -> usize {
        self.bank.n_times()
    }

    /// Best fit of every shape on the columns `cols` of `y`, ordered by `AICc` (ties keep the order of `Form::ALL`). Empty under 3 points.
    pub fn fit_cols(&self, y: &[f64], cols: &[usize]) -> Vec<FormFit> {
        if cols.len() < 3 {
            return Vec::new();
        }
        let (yt, ymax) = Bank::normalise(y, cols);
        let hits = self.bank.fit_groups(cols, &yt, 4);
        let mut out: Vec<FormFit> = Form::ALL
            .iter()
            .filter_map(|&f| hits[f as usize].map(|h| self.make(f, h, ymax, cols.len())))
            .collect();
        out.sort_by(|a, b| a.aicc.total_cmp(&b.aicc));
        out
    }

    fn make(&self, form: Form, h: Hit, ymax: f64, n: usize) -> FormFit {
        let p = self.bank.rows[h.row].p;
        FormFit {
            form,
            rates: [p[0], p[1]],
            scale: h.scale,
            ymax,
            sse: h.sse,
            aicc: aicc(h.sse, n, form.n_rates() + 1),
        }
    }

    /// The best `AICc` of the shape `Bateman` over the whole grid (the first-generation model of `generation`) and its rates.
    pub fn best_bateman(&self, y: &[f64]) -> Option<FormFit> {
        let cols: Vec<usize> = (0..y.len()).collect();
        self.fit_cols(y, &cols)
            .into_iter()
            .find(|f| f.form == Form::Bateman)
    }

    /// All the shapes on all the points.
    pub fn fit(&self, y: &[f64]) -> Vec<FormFit> {
        let cols: Vec<usize> = (0..y.len()).collect();
        self.fit_cols(y, &cols)
    }
}

/// Fits the four shapes to `y` at `times` (builds a bank: reuse a `FormBank` for many profiles on the same times).
pub fn fit_forms(times: &[f64], y: &[f64]) -> Vec<FormFit> {
    if times.len() != y.len() {
        return Vec::new();
    }
    FormBank::new(times).fit(y)
}

/// Shape chosen and the `AICc` gap to the second best (`f64::INFINITY` when there is only one).
pub fn choice(fits: &[FormFit]) -> Option<(Form, f64)> {
    let best = fits.first()?;
    Some((
        best.form,
        fits.get(1).map_or(f64::INFINITY, |s| s.aicc - best.aicc),
    ))
}
