//! The presets of `features.detect` (high resolution, unit resolution, nominal resolution) and the unit-axis grid merge reach.
//! The values are the constants of `recipe_lr.rs`; `recipe_map` is the one mapping to the recipe.

use mzlab_core::step::{Facets, Param, Preset};

use super::detect::{num, text};
use crate::axis::LR_TOUCH;
use crate::recipe_lr::{NOMINAL_FLOOR, NOMINAL_GRID_TOUCH, NOMINAL_MIN_HEIGHT};

pub(super) fn presets() -> Vec<Preset> {
    vec![preset_hr(), preset_lr(), preset_nominal()]
}

/// The reach of the merge of the two grids of the unit axis (`Params::grid_touch`); high resolution merges by ppm and ignores it.
pub(super) fn grid_touch_param() -> Param {
    Param::number("grid_touch", "Grid merge distance")
        .min(0.0)
        .max(0.5)
        .default_num(LR_TOUCH)
        .unit("Da")
        .doc("Unit axis only: two rows of the two grids at one retention time are one ion when their centroids are within this distance; a scattering centroid splits an ion above the default")
        .tip("Rows of the two grids within this distance are one ion; raise it if centroids scatter.")
        .advanced()
}

fn preset_hr() -> Preset {
    Preset {
        id: "hr",
        title: "High resolution",
        when: Facets {
            resolution: vec!["hr"],
            ..Facets::default()
        },
        params: vec![
            ("axis", text("ppm")),
            ("floor", num(20_000.0)),
            ("min_height", num(200_000.0)),
        ],
        evidence: "defaults of trace::Params",
        algorithm: Some("masstrace"),
    }
}

fn preset_lr() -> Preset {
    Preset {
        id: "lr",
        title: "Unit resolution",
        when: Facets {
            resolution: vec!["lr"],
            ..Facets::default()
        },
        params: vec![
            ("axis", text("Da")),
            ("floor", num(3_000.0)),
            ("min_height", num(7_000.0)),
            ("min_snr", num(3.0)),
            ("area_from", text("channel")),
            ("stitch", text("strict")),
        ],
        evidence: "LR_FLOOR, LR_MIN_HEIGHT and LR_MIN_SNR of recipe_lr.rs; the preview proposes the values of each file; on profile scans the floor is the mode of the noise lump",
        algorithm: Some("masstrace"),
    }
}

/// Nominal-resolution triple quadrupole or QTRAP data with low-mass products and a scattering centroid: the `lr` values with a lower floor and height, and a wider grid
/// merge. It is never picked from the data (empty facets: a lower floor lets noise rows in, which the user weighs); it is a choice the report names.
fn preset_nominal() -> Preset {
    Preset {
        id: "nominal",
        title: "Nominal resolution, low-mass products",
        when: Facets::default(),
        params: vec![
            ("axis", text("Da")),
            ("floor", num(NOMINAL_FLOOR)),
            ("min_height", num(NOMINAL_MIN_HEIGHT)),
            ("min_snr", num(3.0)),
            ("area_from", text("channel")),
            ("stitch", text("strict")),
            ("grid_touch", num(NOMINAL_GRID_TOUCH)),
        ],
        evidence: "NOMINAL_FLOOR, NOMINAL_MIN_HEIGHT and NOMINAL_GRID_TOUCH of recipe_lr.rs; synthetic proof tests/lr_nominal.rs",
        algorithm: Some("masstrace"),
    }
}

#[cfg(test)]
#[path = "detect_presets_tests.rs"]
mod tests;
