//! The fit of the error in ppm of the reference ions: the observations, the model (the terms of the error: none, a constant, a line of the m/z, a line of the m/z and the RT) and the
//! robust method (how outliers are kept from bending the fit). Each model and each method is one module (`offset`, `linear_mz`, `linear_mz_rt`; `ols`, `huber`, `ransac`), dispatched here by enum.

use super::{huber, linear_mz, linear_mz_rt, offset, ols, ransac};

/// The m/z where the line of the error in m/z has its constant, and the m/z step of its slope (ppm per `MZ_SCALE`).
pub const MZ_PIVOT: f64 = 400.0;
pub const MZ_SCALE: f64 = 100.0;
/// The RT (minutes) where the term in the RT is zero.
pub const RT_PIVOT: f64 = 5.0;

/// One measured error: of a reference at `mz` (its theoretical m/z), seen at `rt` (minutes), in ppm, with the weight it counts for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obs {
    pub mz: f64,
    pub rt: f64,
    pub err: f64,
    pub w: f64,
}

/// The terms of the error that the correction models.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    None,
    Offset,
    LinearMz,
    LinearMzRt,
}

impl Model {
    pub fn parse(s: &str) -> Option<Model> {
        Some(match s {
            "none" => Model::None,
            "offset" => Model::Offset,
            "linear_mz" => Model::LinearMz,
            "linear_mz_rt" => Model::LinearMzRt,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Model::None => "none",
            Model::Offset => "offset",
            Model::LinearMz => "linear_mz",
            Model::LinearMzRt => "linear_mz_rt",
        }
    }

    /// Number of coefficients.
    pub fn n_params(self) -> usize {
        match self {
            Model::None => 0,
            Model::Offset => 1,
            Model::LinearMz => 2,
            Model::LinearMzRt => 3,
        }
    }

    /// The names of the coefficients, in order.
    pub fn coef_names(self) -> &'static [&'static str] {
        match self {
            Model::None => &[],
            Model::Offset => &["offset_ppm"],
            Model::LinearMz => &["offset_ppm", "slope_ppm_per_100_mz"],
            Model::LinearMzRt => &["offset_ppm", "slope_ppm_per_100_mz", "slope_ppm_per_min"],
        }
    }

    pub(super) fn basis(self, mz: f64, rt: f64) -> [f64; 3] {
        match self {
            Model::None => [0.0; 3],
            Model::Offset => offset::basis(mz, rt),
            Model::LinearMz => linear_mz::basis(mz, rt),
            Model::LinearMzRt => linear_mz_rt::basis(mz, rt),
        }
    }

    /// The error in ppm the model gives at a point.
    pub fn predict(self, coef: &[f64; 3], mz: f64, rt: f64) -> f64 {
        let x = self.basis(mz, rt);
        (0..3).map(|i| coef[i] * x[i]).sum()
    }

    /// The model with one term fewer, for a file whose references cannot tell the terms apart.
    pub fn simpler(self) -> Model {
        match self {
            Model::LinearMzRt => Model::LinearMz,
            Model::LinearMz => Model::Offset,
            _ => Model::None,
        }
    }

    /// Whether the model reads the RT, so that each scan of a reference is an observation.
    pub fn uses_rt(self) -> bool {
        self == Model::LinearMzRt
    }
}

/// How the fit keeps an outlier from bending it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Robust {
    Huber,
    Ransac,
    Plain,
}

impl Robust {
    pub fn parse(s: &str) -> Option<Robust> {
        Some(match s {
            "huber" => Robust::Huber,
            "ransac" => Robust::Ransac,
            "none" => Robust::Plain,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Robust::Huber => "huber",
            Robust::Ransac => "ransac",
            Robust::Plain => "none",
        }
    }
}

/// A fitted model: its coefficients, which observations agree with it and the spread of the residuals (ppm).
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub coef: [f64; 3],
    pub inlier: Vec<bool>,
    pub scale: f64,
    pub model: Model,
}

/// Fits `model` to `obs` with `robust`; `None` when there are too few observations or their m/z (RT) cannot tell the terms apart.
pub fn fit(model: Model, robust: Robust, obs: &[Obs]) -> Option<Fit> {
    if model == Model::None || obs.len() < model.n_params() {
        return None;
    }
    match robust {
        Robust::Huber => huber::fit(model, obs),
        Robust::Ransac => ransac::fit(model, obs),
        Robust::Plain => ols::fit(model, obs),
    }
}

/// `fit`, and when the terms cannot be told apart the same with the next simpler model: the fit of the model that could be told, never a guess.
pub fn fit_down(model: Model, robust: Robust, obs: &[Obs]) -> Option<Fit> {
    let mut m = model;
    while m != Model::None {
        if let Some(f) = fit(m, robust, obs) {
            return Some(f);
        }
        m = m.simpler();
    }
    None
}
