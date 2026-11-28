//! `features.pair`: the descriptor of the step that pairs the features of a positive and a negative table by their neutral mass and RT
//! (`crate::polarity_pair`, the engine of `mzlab-cli table pair`), to see which species show in both polarities. The adduct lists are parameters.

use crate::polarity_pair::{NEG_DEFAULT, POS_DEFAULT};
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use super::FEAT_CEILING;

pub(super) fn pair() -> Step {
    Step {
        ins: vec![
            Port::new("pos", DataType::Table, "Feature table of the positive-polarity method"),
            Port::new("neg", DataType::Table, "Feature table of the negative-polarity method"),
        ],
        outs: vec![
            Port::new("pairs", DataType::Report, "Every pair with the rule, mass error and RT difference; the rows without a partner last (CSV)").format("csv"),
            Port::new("pair", DataType::Report, "The pairs, the unmatched rows, the adducts used and every automatic choice (pair/1)").format("pair/1"),
        ],
        params: vec![
            Param::number("rt_tol", "RT tolerance").min(0.0).max(60.0).default_num(0.1).unit("min")
                .doc("Largest RT difference of the two ions of a pair; the RT axes of the two methods are compared as they are, with no map")
                .tip("Largest RT difference of a pair."),
            Param::number("mass_tol", "Mass tolerance").min(0.0).max(1000.0).default_num(5.0)
                .doc("Largest difference of the two neutral masses, in the unit below; a unit-mass table needs Da (0.4 is a usual choice)")
                .tip("Largest difference of the two neutral masses."),
            Param::choice("mass_unit", "Mass unit", &["ppm", "da"]).default_str("ppm")
                .doc("The unit of the mass tolerance: ppm of the neutral mass, or Da"),
            Param::text("pos_adducts", "Positive adducts").default_str(POS_DEFAULT)
                .doc("Comma-separated adducts of the positive table: names of the adduct table ([M+H]+, [M+Na]+, [M+NH4]+, ...) or LABEL:SHIFT, the mass the adduct adds to M for a singly charged ion")
                .tip("Adducts of the positive table, comma-separated."),
            Param::text("neg_adducts", "Negative adducts").default_str(NEG_DEFAULT)
                .doc("Comma-separated adducts of the negative table: names of the adduct table ([M-H]-, [M+HCOO]-, [M+Cl]-, ...) or LABEL:SHIFT (negative for a loss)")
                .tip("Adducts of the negative table, comma-separated."),
        ],
        runs_in: RunsIn {
            wasm: Some("flowtab"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["err.pair.param", "err.pair.empty", "err.pair.table"],
        ..Step::core(
            "features.pair",
            "Pair polarities",
            "Pairs the [M+H]+ and [M-H]- features of the two polarity methods by neutral mass and RT, to see which species behave as acids. Every pair within both tolerances is listed with its rule, mass error and RT difference, and the rows without a partner are listed too; nothing is merged or identified.",
        )
    }
}
