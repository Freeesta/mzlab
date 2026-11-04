//! The subcommands of `mzlab-cli` that take their arguments as a plain list, and the old command names still accepted for them.

use crate::{
    campioni_cmd, canali_cmd, flow_explain, ion_pattern_cmd, llm, mol_label_cmd, msn_rules,
    peaks_auto_cmd, peaks_edges_cmd, sim_table_cmd, steps_cmd, Outcome,
};

/// The subcommands that take their arguments as a plain list.
pub(crate) const PLAIN: [&str; 11] = [
    "llm",
    "steps",
    "explain-missing",
    "msn-rules",
    "channels",
    "ion-pattern",
    "mol-label",
    "samples",
    "peak-edges",
    "sim-table",
    "peak-auto",
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
        "channels" => canali_cmd::run(args),
        "ion-pattern" => ion_pattern_cmd::run(args),
        "mol-label" => mol_label_cmd::run(args),
        "samples" => campioni_cmd::run(args),
        "sim-table" => sim_table_cmd::run(args),
        "peak-auto" => peaks_auto_cmd::run(args),
        _ => peaks_edges_cmd::run(args),
    }
}

#[cfg(test)]
#[path = "plain_tests.rs"]
mod tests;
