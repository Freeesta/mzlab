//! The `step/1` descriptors of `qc.run_order@1` (the facts and warnings of the injection sequence of the sample table, `runorder.rs`) and `report.methods@1` (the methods text of a run record,
//! `methods/`). Both are in `flowtab` in the browser (`wasm-feat/flowtab/`) and run natively in `flow run`; they are exported with the steps of `mzlab-cli` (`docs/schemi/steps-cli-1.json`).

use super::FEAT_CEILING;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

/// `qc.run_order`: reads the sample table and its batches; reports counts, the largest gap between QC injections, the blanks and the standard levels, with warnings and the files involved.
pub fn run_order() -> Step {
    let optional = |p: Port| Port {
        optional: true,
        ..p
    };
    Step {
        ins: vec![
            Port::new("samples", DataType::Samples, "Sample table: role, injection order, batch (campioni/2 or /3)"),
            optional(Port::new("design", DataType::Design, "Batches of the files; without it the batch of the sample table, else one batch")),
        ],
        outs: vec![Port::new("runorder", DataType::Qc, "Counts and warnings of the injection sequence (runorder/1)")],
        params: vec![
            Param::integer("qc_every_max", "Largest gap between QC injections")
                .default_num(10.0)
                .min(1.0)
                .unit("injections")
                .doc("A gap of more injections between two QC files of the same batch is reported; the gap is the difference of their injection positions")
                .tip("Report two QC files that are further apart than this."),
            Param::integer("std_levels_min", "Standard levels per batch")
                .default_num(2.0)
                .min(0.0)
                .doc("A batch whose standards and calibration files have fewer distinct concentrations is reported; 0 turns the check off")
                .tip("Report a batch with fewer distinct standard concentrations."),
            Param::boolean("blank_per_batch", "Blank in every batch")
                .default_bool(true)
                .doc("Report a batch without a blank file")
                .tip("Report a batch with no blank."),
        ],
        runs_in: RunsIn {
            wasm: Some("flowtab"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["error.badCampioni", "error.badDisegno"],
        ..Step::core(
            "qc.run_order",
            "Check run order",
            "Reads the sample table and reports, for each batch, the number of files of each role, the largest gap between two QC injections, the blanks and the distinct standard levels, with a warning for a gap above the limit, a batch without QC or blank, too few standard levels, and a missing or repeated injection order. It reports facts, never a verdict.",
        )
    }
}

/// `report.methods`: the methods text from the `corsa/1` record of a run.
pub fn methods() -> Step {
    Step {
        ins: vec![Port::new("run", DataType::Report, "Run record (corsa/1)")],
        outs: vec![Port::new("methods", DataType::Report, "Methods text (Markdown)")],
        params: vec![Param::choice("style", "Layout", &["paragraph", "list"])
            .default_str("paragraph")
            .doc("paragraph: one English paragraph per step; list: one bullet per step with every parameter")],
        runs_in: RunsIn {
            wasm: Some("flowtab"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["err.corsa.prov"],
        ..Step::core(
            "report.methods",
            "Methods text",
            "Writes the methods of a run in English from its run record: every step with its version, algorithm and parameters, and the version of mzLab. It states what was set, never a result.",
        )
    }
}
