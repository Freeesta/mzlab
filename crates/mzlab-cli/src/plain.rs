//! The subcommands of `mzlab-cli` that take their arguments as a plain list, and the old command names still accepted for them.

use crate::{
    channels_cmd, flow_explain, ion_pattern_cmd, llm, manual_table_cmd, mol_cut_cmd, mol_id_cmd,
    mol_label_cmd, msn_rules, peaks_auto_cmd, peaks_edges_cmd, samples_cmd, steps_cmd, Outcome,
};

#[path = "dda_window_cmd.rs"]
mod dda_window_cmd;
#[path = "ion_mz_cmd.rs"]
mod ion_mz_cmd;
#[path = "peak_area_cmd.rs"]
pub(crate) mod peak_area_cmd;
#[path = "peak_window_cmd.rs"]
mod peak_window_cmd;

/// The subcommands that take their arguments as a plain list.
pub(crate) const PLAIN: [&str; 17] = [
    "llm",
    "steps",
    "explain-missing",
    "msn-rules",
    "channels",
    "dda-window",
    "ion-pattern",
    "ion-mz",
    "mol-label",
    "mol-cut",
    "mol-id",
    "samples",
    "peak-edges",
    "manual-table",
    "peak-auto",
    "peak-area",
    "peak-window",
];

/// Old command names still accepted on input, as `(old, new)`; they are never printed outside `--help`.
const ALIASES: [(&str, &str); 3] = [
    ("canali", "channels"),
    ("campioni", "samples"),
    ("peaks-edges", "peak-edges"),
];

pub(crate) fn canonical(command: &str) -> &str {
    ALIASES
        .iter()
        .find(|(old, _)| *old == command)
        .map_or(command, |(_, new)| new)
}

pub(crate) fn run(command: &str, args: Vec<String>) -> Outcome {
    match command {
        "llm" => llm::run(args),
        "steps" => steps_cmd::run(args),
        "explain-missing" => flow_explain::run(args),
        "msn-rules" => msn_rules::run(args),
        "channels" => channels_cmd::run(args),
        "dda-window" => dda_window_cmd::run(args),
        "ion-pattern" => ion_pattern_cmd::run(args),
        "ion-mz" => ion_mz_cmd::run(args),
        "mol-label" => mol_label_cmd::run(args),
        "mol-cut" => mol_cut_cmd::run(args),
        "mol-id" => mol_id_cmd::run(args),
        "samples" => samples_cmd::run(args),
        "manual-table" => manual_table_cmd::run(args),
        "peak-auto" => peaks_auto_cmd::run(args),
        "peak-area" => peak_area_cmd::run(args),
        "peak-window" => peak_window_cmd::run(args),
        _ => peaks_edges_cmd::run(args),
    }
}

#[cfg(test)]
#[path = "plain_tests.rs"]
mod tests;
