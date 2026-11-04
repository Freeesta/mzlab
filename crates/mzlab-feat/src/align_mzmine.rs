//! Parameters of the algorithms `join` of `features.align` and `peak_finder` of `features.gapfill`: the join aligner and the peak finder of
//! `untargeted/` (mzmine 3.9.0). Plain numbers with no feature gate, so the recipe and the descriptors exist in every build; the run is
//! `untargeted::flow::join_table` and `untargeted::flow::peak_finder` (feature `untargeted`). Defaults are those of `untargeted::join::JoinParams` and
//! `untargeted::gapfill::GapParams`; the m/z tolerance in ppm is the `ppm` of the step.

/// The settings of `join` that the common parameters of `features.align` do not carry.
#[derive(Debug, Clone, PartialEq)]
pub struct JoinAlign {
    /// Weights of the m/z and the RT terms of the score of a pair.
    pub mz_weight: f64,
    pub rt_weight: f64,
    /// Half width of the RT range, minutes.
    pub rt_tol: f64,
    /// Absolute lower bound of the m/z tolerance, Da (the larger of it and the `ppm` of the step wins).
    pub tol_da: f64,
}

impl Default for JoinAlign {
    fn default() -> Self {
        JoinAlign {
            mz_weight: 3.0,
            rt_weight: 1.0,
            rt_tol: 0.2,
            tol_da: 0.0015,
        }
    }
}

/// The settings of `peak_finder` that the common parameters of `features.gapfill` (`floor` = noise of the mass list, `ppm`, `rt_tol`,
/// `min_points`) do not carry.
#[derive(Debug, Clone, PartialEq)]
pub struct PeakFinder {
    /// A point joins the peak while it is not above its neighbour toward the apex by more than this fraction.
    pub intensity_tol: f64,
    /// Absolute lower bound of the m/z tolerance, Da.
    pub tol_da: f64,
}

impl Default for PeakFinder {
    fn default() -> Self {
        PeakFinder {
            intensity_tol: 0.2,
            tol_da: 0.0015,
        }
    }
}
