//! Parameters of the algorithms `join` of `features.align` and `peak_finder` of `features.gapfill`, and of the isotope groupings `grouper` and
//! `finder` of `features.detect`: the join aligner, the peak finder, the isotope grouper (mzmine 3.9.0) and the isotope finder (mzmine 4.10) of
//! `untargeted/`. Plain values with no feature gate, so the recipe and the descriptors exist in every build; the run is in `untargeted::flow_align`,
//! `untargeted::flow_iso` and `untargeted::flow_isofinder` (feature `untargeted`). Defaults are those of the engines.

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

/// The settings of the isotope grouper of mzmine (`features.detect`, `isotopes` = `grouper`); it replaces the averagine grouping of `isotopes.rs`.
#[derive(Debug, Clone, PartialEq)]
pub struct IsoGrouper {
    /// m/z tolerance: the larger of the ppm part and the Da part wins.
    pub tol_ppm: f64,
    pub tol_da: f64,
    /// Largest RT distance to the start row, minutes.
    pub rt_tol: f64,
    pub max_charge: u8,
    /// Only look above the start row.
    pub monotonic: bool,
    /// The lowest m/z of a group is its representative instead of the most intense row.
    pub lowest_mz: bool,
}

impl Default for IsoGrouper {
    fn default() -> Self {
        IsoGrouper {
            tol_ppm: 5.0,
            tol_da: 0.0015,
            rt_tol: 0.05,
            max_charge: 2,
            monotonic: false,
            lowest_mz: false,
        }
    }
}

/// The settings of the isotope finder of mzmine 4.10 (`features.detect`, `isotopes` = `finder`): the pattern of each feature in its apex spectrum.
#[derive(Debug, Clone, PartialEq)]
pub struct IsoFinder {
    /// m/z tolerance: the larger of the ppm part and the Da part wins.
    pub tol_ppm: f64,
    pub tol_da: f64,
    pub max_charge: u8,
    /// Elements whose isotopes are looked for, comma separated.
    pub elements: String,
    /// A charge counts only with a 13C peak.
    pub require_c13: bool,
    /// The heavy elements seen in the spectrum widen the expected envelope too.
    pub auto: bool,
}

impl Default for IsoFinder {
    fn default() -> Self {
        IsoFinder {
            tol_ppm: 15.0,
            tol_da: 0.005,
            max_charge: 3,
            elements: "H,C,N,O,S".into(),
            require_c13: true,
            auto: true,
        }
    }
}
