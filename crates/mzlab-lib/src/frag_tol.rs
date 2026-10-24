//! Fragment tolerance of the spectrum scores: a fixed window in Da, or a relative one in ppm of the query peak (HR spectra).
//! A plain `f64` converts to the Da form, so callers that pass Da keep their bits.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FragTol {
    pub da: f64,
    /// When > 0 the window is `ppm` of the m/z and `da` is ignored.
    pub ppm: f64,
}

impl FragTol {
    pub fn ppm(ppm: f64) -> FragTol {
        FragTol { da: 0.0, ppm }
    }

    /// The `tol_ppm` of a step when set, else its `tol_da`.
    pub fn from_params(da: f64, ppm: f64) -> FragTol {
        FragTol { da, ppm }
    }

    /// Half-width of the window around a peak at `mz`.
    #[inline]
    pub fn at(&self, mz: f64) -> f64 {
        if self.ppm > 0.0 {
            mz * self.ppm * 1e-6
        } else {
            self.da
        }
    }
}

impl From<f64> for FragTol {
    fn from(da: f64) -> FragTol {
        FragTol { da, ppm: 0.0 }
    }
}
