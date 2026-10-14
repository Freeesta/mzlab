//! The profile `lr` of the recipe: the same pipeline on the unit axis (low-resolution full scan of a triple quadrupole). The index says which profile
//! a file needs (`Index::hr1`); every tolerance of m/z is in Da (`axis::da`), the lobes of the Fourier transform are not looked for, the isotopes
//! are only a +1 registration and the +2 step of Cl/Br, the families have the nominal steps of the adducts and the losses.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::axis::{da, estimate_defect, MassAxis, LR_TOL, LR_TRACE};
use crate::recipe::Recipe;
use crate::trace::Params;

/// Noise floor and minimum height of the unit profile (counts per second of a triple quadrupole); the preview proposes the ones of each file.
pub const LR_FLOOR: f64 = 2e3;
pub const LR_MIN_HEIGHT: f64 = 2e4;
/// Width of the bins of the comparison of two MS2 spectra at unit resolution, m/z.
const LR_MS2_BIN: f64 = 0.5;

impl Params {
    /// Detection on the unit axis with the mass defect `defect` of the file.
    pub fn lr(defect: f64) -> Params {
        Params {
            floor: LR_FLOOR,
            min_height: LR_MIN_HEIGHT,
            axis: MassAxis::Da { defect },
            ..Params::default()
        }
    }
}

impl Recipe {
    /// The recipe of the unit profile: `defect`, `floor` and `min_height` come from the preview of the file.
    pub fn lr(defect: f64, floor: f64, min_height: f64) -> Recipe {
        let mut r = Recipe {
            detect: Params {
                floor,
                min_height,
                ..Params::lr(defect)
            },
            ..Recipe::default()
        };
        r.steps.quality.window_ppm = da(LR_TRACE);
        r.steps.resolve.window_ppm = da(LR_TRACE);
        r.steps.resolve.bin = LR_MS2_BIN;
        r.steps.iso.tol_ppm = da(LR_TOL);
        r.steps.iso.max_charge = 1;
        r.steps.ms2.tol_ppm = da(LR_TOL);
        r.steps.ms2.default_half_window = 0.5;
        r.fam.trace_ppm = da(LR_TRACE);
        r.fam.rules.ppm = 0.0;
        r.fam.rules.abs_da = LR_TOL;
        r.align.ion.ppm = da(LR_TOL);
        r.align.ion.max_ppm_span = da(2.0 * LR_TOL);
        r.rt.crowd_ppm = da(LR_TOL);
        r.rt.min_height = min_height;
        r.fill.ppm = da(LR_TOL);
        r.fill.floor = floor;
        r
    }

    /// The recipe the data of an index need: the default one at high resolution, the unit profile (mass defect measured here) otherwise.
    pub fn for_index<R: ReadAt>(index: &mut Index<R>) -> Result<Recipe> {
        let base = Recipe::default();
        let Some(si) = index.section_of(1, base.detect.polarity) else {
            return Ok(base);
        };
        if index.hr1() {
            return Ok(base);
        }
        let defect = estimate_defect(index, si, LR_FLOOR)?.unwrap_or(0.0);
        Ok(Recipe::lr(defect, LR_FLOOR, LR_MIN_HEIGHT))
    }
}
