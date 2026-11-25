//! `features.union`: the descriptor of the step that joins the feature tables of several series (conditions, replicates) into ONE target list for
//! `quant.targets` (`union`, the engine of `mzlab-cli quant union`). The defaults are the ones of the command; a value out of range is refused, never clamped.

use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use super::FEAT_CEILING;

pub(super) fn union() -> Step {
    Step {
        ins: vec![Port::new("table", DataType::Table, "One feature table per series, the first fixes the RT frame").many()],
        outs: vec![
            Port::new("targets", DataType::Report, "One target list for quant.targets: id, mz, rt_min, rt_max, polarity, role (CSV)").format("csv"),
            Port::new("union", DataType::Report, "The targets with the rows each came from, conflicts, dropped targets and every automatic choice (union/1)").format("union/1"),
        ],
        params: vec![
            Param::number("da", "m/z agreement").min(0.001).max(5.0).default_num(0.5).unit("Da")
                .doc("Rows of different series are one target when their m/z differ by no more than this"),
            Param::number("rt_tol", "RT agreement").min(0.001).max(5.0).default_num(0.1).unit("min")
                .doc("... and their RT, after the map, by no more than this; the window of a row without one is its RT plus and minus this")
                .tip("RT agreement after the map; also the half-window of a row without an RT range."),
            Param::choice("rt_warp", "RT map", &["off", "piecewise"]).default_str("off")
                .doc("piecewise maps every other series onto the first from the rows both hold (fewer than 3 anchors: no map, logged)"),
            Param::number("rt_gap", "Widest anchor gap").min(0.001).max(5.0).default_num(0.5).unit("min")
                .doc("Widest RT gap between two rows that may serve as an anchor of the map").advanced(),
            Param::integer("min_series", "Min. series").min(1.0).max(1000.0).default_num(1.0).unit("series")
                .doc("A target seen in fewer series is listed as dropped and not written; 1 keeps every row"),
            Param::number("pad", "Window padding").min(0.0).max(5.0).default_num(0.0).unit("min")
                .doc("Minutes added on both sides of every window"),
            Param::choice("polarity", "Polarity", &["any", "+", "-"]).default_str("any")
                .doc("The polarity of the rows (a table holds none): + or -, any = every scan; run one list per polarity")
                .tip("The polarity of the list: + or -, any = every scan; run one list per polarity."),
        ],
        runs_in: RunsIn {
            wasm: Some("flowtab"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["err.union.param", "err.union.empty", "err.union.table"],
        ..Step::core(
            "features.union",
            "Join series",
            "Joins the feature tables of several series into one target list, so a whole campaign is measured once by quant.targets and every cell is integrated or an explicit absent row. Rows of one series are never merged; conflicts, dropped targets and every automatic choice are listed with their numbers.",
        )
    }
}
