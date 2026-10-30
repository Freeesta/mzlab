//! `screen.suspects`: the descriptor of the suspect screening of the aligned feature table (`flow_screen_suspects`). The defaults are the ones of `flow_screen_suspects::Params`.

use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use super::ident::{attributio_step, hr};

/// The `.wasm` that reads feature tables and its fixed ceiling in bytes (`limit_table` of `tools/build_feat_wasm.sh`).
const TABLE_WASM: &str = "table";
const TABLE_CEILING: u64 = 737_280;

/// `screen.suspects`: mass and formula candidates from the lists of the user, with isomer groups and optional RT and MS2 fragment evidence.
pub(super) fn suspects() -> Step {
    let optional = |p: Port| Port {
        optional: true,
        ..p
    };
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Aligned feature table (.mzfeat)").requires(hr()),
            Port::new("suspects", DataType::Library, "Suspect lists: .mzsus files or CSV (name, InChIKey or SMILES, formula or neutral mass, optional RT and fragment m/z separated by ;)").many(),
            optional(Port::new("spectra", DataType::Spectra, "MS2 spectra of the same run (spectra.ms2): fragment evidence")),
            optional(Port::new("formulas", DataType::Formulas, "Candidate formulas (formula.ms1): a supported formula gives Level 4")),
        ],
        outs: vec![Port::new("screen", DataType::Candidates, "Candidates of each feature with isomer groups and evidence (screen/1)")],
        params: vec![
            Param::number("ppm", "Mass tolerance").min(0.0).default_num(5.0).unit("ppm"),
            Param::text("adducts", "Adducts").default_str("[M+H]+,[M+NH4]+,[M+Na]+,[M-H]-,[M+Cl]-").doc("Comma-separated; the ones of the polarity of the table are searched"),
            Param::number("rt_window_min", "RT window").min(0.0).unit("min").doc("Off when empty; a candidate whose listed RT is farther from the feature is dropped (only rows that list an RT)"),
            Param::integer("min_fragments", "Fragments needed").min(0.0).doc("Off when empty; keeps the candidates with at least this many of their listed fragments found in the MS2 of the feature"),
            Param::integer("max_hits_per_feature", "Formulas per feature").min(1.0).default_num(10.0).doc("Most formula groups kept per feature, the closest in mass first"),
            Param::number("ms2_rt_min", "MS2 RT tolerance").min(0.0).default_num(0.2).unit("min").doc("Without feature ids in the spectra: largest RT distance between a scan and the feature").advanced(),
        ],
        runs_in: RunsIn { wasm: Some(TABLE_WASM), wasm_ceiling_bytes: Some(TABLE_CEILING), native: true },
        errors: vec!["err.flow.screen.mzsus", "err.flow.screen.csv", "err.flow.screen.adduct", "err.flow.screen.polarity", "err.flow.port.missing", "error.badMzfeat"],
        ..attributio_step(
            "screen.suspects",
            "Find suspects",
            "Searches the aligned feature table in the suspect lists you give by neutral mass for each adduct. Listed structures with the same formula are one isomer group, never one name. Candidates with their evidence, Level 5 (mass and formula) or 4 (also a candidate formula); no verdict.",
        )
    }
}
