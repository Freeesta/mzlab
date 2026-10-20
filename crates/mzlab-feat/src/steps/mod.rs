//! The `step/1` descriptors of the feature steps of the flagship chain (`mzlab_core::step`) and the whole registry of the core package.
//! The parameters are the fields of `recipe::Recipe` that each step reads, with their defaults written out; `tests/steps.rs` builds the recipe
//! from these defaults and compares its fingerprint with `Recipe::default()`. The identification chain is in `ident*.rs` (`attributio()`).

mod detect;
mod ident;
mod ident_annot;
mod ident_formula;
mod ident_spectra;
pub mod records;
mod table;
mod tabui;
mod tabui_draw;
mod tabui_link;

use mzlab_core::step::{core_steps, Step};

/// The `.wasm` of mzFinder that runs detection, alignment and filling, and its fixed size ceiling in bytes.
pub(crate) const FEAT_WASM: &str = "feat (detect)";
pub(crate) const FEAT_CEILING: u64 = 512_000;

/// The steps of the identification chain (package `attributio`), apart from `registry` so that the export of the core package stays as it is.
pub fn attributio() -> Vec<Step> {
    ident::registry()
}

/// The steps of the Tabui chain that `mzlab-feat` owns (package `tabui`); `mzlab_chem::step::tabui()` adds `structure.reactivity`.
pub fn tabui() -> Vec<Step> {
    tabui::registry()
}

/// Every `core` step of the flagship chain, in the order of the chain: the ones of `mzlab-core` first, then the feature steps.
pub fn registry() -> Vec<Step> {
    let mut all = core_steps();
    all.extend([
        detect::estimate(),
        detect::detect(),
        table::align(),
        table::gapfill(),
        table::filter(),
        detect::ms2_link(),
        table::export_csv(),
        table::export_inclusion(),
        table::export_mztab(),
    ]);
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::Recipe;
    use crate::recipe_lr::{LR_FLOOR, LR_MIN_HEIGHT};
    use mzlab_core::step::find;

    /// The two presets of `features.detect` hold the constants of the two profiles (`Params::default`, `LR_FLOOR`, `LR_MIN_HEIGHT`).
    #[test]
    fn presets_are_the_constants_of_the_two_profiles() {
        let reg = registry();
        let detect = find(&reg, "features.detect").unwrap();
        let preset = |id: &str, n: &str| {
            let p = &detect.presets.iter().find(|p| p.id == id).unwrap().params;
            p.iter().find(|(k, _)| *k == n).unwrap().1.clone()
        };
        let hr = Recipe::default().detect;
        assert_eq!(preset("hr", "floor").f64(), Some(hr.floor));
        assert_eq!(preset("hr", "min_height").f64(), Some(hr.min_height));
        assert_eq!(preset("hr", "axis").str(), Some("ppm"));
        let lr = Recipe::lr(0.0, LR_FLOOR, LR_MIN_HEIGHT).detect;
        assert_eq!(preset("lr", "floor").f64(), Some(lr.floor));
        assert_eq!(preset("lr", "min_height").f64(), Some(lr.min_height));
        assert!(lr.axis.is_da());
        assert_eq!(preset("lr", "axis").str(), Some("Da"));
        assert_eq!(detect.default_preset, Some("hr"));
    }
}
