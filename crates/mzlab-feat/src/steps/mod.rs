//! The `step/1` descriptors of the feature steps of the flagship chain (`mzlab_core::step`) and the whole registry of the core package.
//! The parameters are the fields of `recipe::Recipe` that each step reads, with their defaults written out; `tests/steps.rs` builds the recipe
//! from these defaults and compares its fingerprint with `Recipe::default()`. The identification chain is in `ident*.rs` (`attributio()`).

mod companions;
mod detect;
mod detect_presets;
mod group;
mod ident;
mod ident_annot;
mod ident_formula;
mod ident_screen;
mod ident_spectra;
mod moiety;
pub use ident_spectra::table_match;
pub use moiety::moiety;
mod qcreport;
mod recipe_algo;
mod recipe_iso;
pub mod recipe_map;
pub mod records;
mod table_algo;
pub use qcreport::{methods, run_order};
mod stats;
mod table;
mod tabui;
mod tabui_draw;
mod tabui_link;
mod transitions;
mod union;
pub use transitions::{transitions, EXAMPLE_1};

use mzlab_core::step::{core_steps, Step};

/// The `.wasm` of mzFinder that runs detection, alignment and filling, and its fixed size ceiling in bytes.
pub(crate) const FEAT_WASM: &str = "feat (detect)";
pub(crate) const FEAT_CEILING: u64 = 512_000;

/// The steps of the identification chain (package `attributio`), apart from `registry` so that the export of the core package stays as it is.
pub fn attributio() -> Vec<Step> {
    ident::registry()
}

/// `export.annotations`, exported with the steps of `mzlab-cli` (`steps-cli-1.json`) like `qc.run_order`.
pub fn export_annotations() -> Step {
    table::export_annotations()
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
        table::drift(),
        group::group(),
        companions::companions(),
        union::union(),
        stats::stats(),
        stats::pca(),
        stats::kmd(),
        stats::compare(),
        stats::plausible(),
        detect::ms2_link(),
        table::export_csv(),
        table::export_inclusion(),
        table::export_mztab(),
    ]);
    all
}
