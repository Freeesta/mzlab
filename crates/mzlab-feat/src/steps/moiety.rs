//! The `step/1` descriptor of `structure.moiety@1`: the candidates of a table whose structure contains a moiety, or lacks it (`chem/moiety.rs`). In `molkit`
//! in the browser (`wasm-feat/molkit/`, `flow_moiety`) and native in `flow run`; exported with the steps of `mzlab-cli` (`docs/schemi/steps-cli-1.json`).

use super::FEAT_CEILING;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

/// `structure.moiety`: a candidate table filtered by a substructure.
pub fn moiety() -> Step {
    Step {
        ins: vec![Port::new(
            "candidates",
            DataType::Candidates,
            "Candidate table: CSV with a SMILES column (a suspect list or an export with structures)",
        )],
        outs: vec![Port::new(
            "candidates",
            DataType::Candidates,
            "The rows that contain (or lack) the moiety, with moiety_matches as the last column",
        )],
        params: vec![
            Param::text("query", "Moiety")
                .default_str("")
                .doc("SMILES of the moiety searched for (the selection of the Draw editor gives one); explicit H atoms must be matched")
                .tip("The substructure every row kept must contain."),
            Param::text("column", "SMILES column")
                .default_str("smiles")
                .doc("Header of the column with the SMILES of each row, compared without case")
                .advanced(),
            Param::boolean("stereo", "Match stereo")
                .default_bool(false)
                .doc("The stereo of the moiety must match too; off, the stereo of both is ignored"),
            Param::choice("keep", "Rows kept", &["containing", "lacking"])
                .default_str("containing")
                .doc("containing: the rows whose structure contains the moiety; lacking: the rows whose structure does not. Rows without a readable structure are left out either way")
                .tip("Keep the rows with the moiety, or the rows without it."),
        ],
        runs_in: RunsIn {
            wasm: Some("molkit"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["error.sub.query", "error.sub.column", "error.sub.db", "error.sub.dbSize", "error.sub.request"],
        ..Step::core(
            "structure.moiety",
            "Contains moiety",
            "Keeps the rows of a candidate table whose structure contains a moiety (or the ones that lack it) and writes the number of its matches in a last column. A row kept is a candidate that holds the group, never an identification.",
        )
    }
}
