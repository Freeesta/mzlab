//! The `step/1` descriptors of the identification chain (`package: attributio`): shared helpers and the list of the nine steps. Every step requires high
//! resolution (principle 1: no formula, library match or level at unit resolution). The defaults are the ones of the engine structs and of the keys of
//! `campaign.toml`; `tests/steps_attributio.rs` compares them with the code (feature `attributio`). Nothing here is called by a `.wasm`.

use mzlab_core::step::{Algorithm, Applicability, Facets, RunsIn, Status, Step};

pub(super) use super::ident_annot::{hazard, level, regulatory};
pub(super) use super::ident_formula::{ms1, ms2, rt_predict};
pub(super) use super::ident_spectra::{chimeric, insilico, match_spectra};

/// The fused `.wasm` of Attributio (manifest entries `attributio`, `attributio_rev`, `attributio_export`, `attributio_ms2`) and its fixed ceiling in
/// bytes (`limit_attributio` of `tools/build_feat_wasm.sh`).
pub(super) const ATTRIBUTIO_WASM: &str = "attributio";
pub(super) const ATTRIBUTIO_CEILING: u64 = 1_388_544;

/// Every step of the identification chain, in the order of the chain.
pub fn registry() -> Vec<Step> {
    vec![
        chimeric(),
        ms1(),
        ms2(),
        match_spectra(),
        insilico(),
        rt_predict(),
        level(),
        regulatory(),
        hazard(),
    ]
}

/// A step of the `attributio` package that runs in the fused module of Attributio.
pub(super) fn attributio_step(id: &'static str, title: &'static str, doc: &'static str) -> Step {
    Step {
        package: "attributio",
        runs_in: RunsIn {
            wasm: Some(ATTRIBUTIO_WASM),
            wasm_ceiling_bytes: Some(ATTRIBUTIO_CEILING),
            native: true,
        },
        ..Step::core(id, title, doc)
    }
}

/// High resolution data: the facet every port of the chain requires.
pub(super) fn hr() -> Facets {
    Facets {
        resolution: vec!["hr"],
        ..Facets::default()
    }
}

/// High resolution MS2 spectra.
pub(super) fn hr_ms2() -> Facets {
    Facets {
        ms_levels: vec![2],
        ..hr()
    }
}

/// An algorithm at version 1 without references or parameters; the caller fills them with struct update syntax.
pub(super) fn algorithm(
    id: &'static str,
    title: &'static str,
    doc: &'static str,
    status: Status,
) -> Algorithm {
    Algorithm {
        id,
        version: 1,
        title,
        doc,
        references: Vec::new(),
        applicability: Applicability {
            resolution: vec!["hr"],
            ..Applicability::default()
        },
        params: Vec::new(),
        status,
    }
}
