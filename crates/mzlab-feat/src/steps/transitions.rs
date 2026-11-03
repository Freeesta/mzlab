//! The `step/1` descriptor of `chrom.transitions@1` (`transitions/`): the integration of the SIM ions and SRM transitions of a table in triple quadrupole
//! mzML files. In `flowtab` in the browser and native in `flow run`; exported with the steps of `mzlab-cli` (`docs/schemi/steps-cli-1.json`).

use super::FEAT_CEILING;
use mzlab_core::json::Value;
use mzlab_core::step::{DataType, Facets, Param, Port, Preset, RunsIn, Step};

/// The example table shipped with the program (`data/transitions-example-1.csv`): rows written as formula + adduct, no RT.
pub const EXAMPLE_1: &str = include_str!("../../data/transitions-example-1.csv");

fn example_1() -> Preset {
    Preset {
        id: "example_1",
        title: "Example table 1 (products, [M+H]+)",
        when: Facets::default(),
        params: vec![("transitions", Value::Str(EXAMPLE_1.to_string()))],
        evidence: "data/transitions-example-1.csv",
        algorithm: None,
    }
}

/// `chrom.transitions`: the table of SIM ions / SRM transitions integrated in each file, or the candidate peaks of the rows without an RT.
pub fn transitions() -> Step {
    Step {
        ins: vec![Port::new("source", DataType::Mzml, "mzML files of a triple quadrupole (SIM or SRM chromatograms)").many()],
        outs: vec![Port::new("peaks", DataType::Report, "Areas, heights and apex RT of each row in each file, or candidate peaks (CSV)")],
        params: vec![
            Param {
                list: Some("transitions"),
                ..Param::text("transitions", "Transition table")
            }
            .doc("CSV with the header name;q1;q3;polarity;rt;rt_window;isomer_group;note;formula;adduct: q3 empty for a SIM ion, q1 empty to take it from formula and adduct ([M+H]+ or [M-H]- by polarity), rt empty when not known yet")
            .label("Transition table")
            .tip("One row per SIM ion or SRM transition; leave RT empty to look for candidates."),
            Param::choice("output", "Output", &["peaks", "table", "candidates"])
                .default_str("peaks")
                .doc("peaks: one row per file and table row; table: one row per table row with the area and apex RT of each file; candidates: the peaks of the rows without an RT")
                .tip("Long list, one row per transition, or candidate peaks."),
            Param::number("mz_tol", "m/z tolerance")
                .min(0.0)
                .default_num(0.5)
                .unit("Da")
                .doc("A chromatogram belongs to a row when its Q1 (and Q3) are within this of the row's"),
            Param::number("rt_window", "RT half window")
                .min(0.0)
                .default_num(0.3)
                .unit("min")
                .doc("The peak is searched in RT plus and minus this when the row writes no window"),
            Param::number("snr_min", "Min. S/N")
                .min(0.0)
                .default_num(3.0)
                .doc("A peak under this signal-to-noise is reported as no_peak"),
            Param::choice_num("smooth", "Smoothing", &[1.0, 3.0, 5.0])
                .default_num(3.0)
                .unit("points")
                .doc("1 = none, 3 = 1-2-1, 5 = 1-4-6-4-1 before the apex and the edges are found")
                .advanced(),
            Param::integer("min_points", "Min. points per peak")
                .min(1.0)
                .default_num(3.0)
                .doc("A peak with fewer points above 5 % of its height is reported as no_peak")
                .advanced(),
            Param::integer("max_candidates", "Candidates per trace")
                .min(1.0)
                .default_num(5.0)
                .doc("Most candidate peaks taken from the trace of each file (candidates output)"),
            Param::number("group_rt", "RT gap of a group")
                .min(0.0)
                .default_num(0.1)
                .unit("min")
                .doc("Candidate apexes of different files closer than this are one candidate (candidates output)"),
        ],
        presets: vec![example_1()],
        runs_in: RunsIn {
            wasm: Some("flowtab"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["err.transitions.table", "err.transitions.empty", "err.transitions.noChannel", "err.transitions.manifest"],
        ..Step::core(
            "chrom.transitions",
            "Transition table",
            "Matches each row of a table of SIM ions or SRM transitions to the chromatograms of each file at unit resolution and integrates the peak inside its RT window: area, height, apex RT, S/N and a flag when there is no peak. For rows without an RT it lists candidate peaks sorted by area; it never says which isomer a peak is.",
        )
    }
}
