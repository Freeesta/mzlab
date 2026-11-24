//! `features.companions`: the descriptor of the search for the ions that co-elute with a row of a unit-mass table and differ from it by a known nominal
//! mass (`companions`). The defaults are a choice for UHPLC peaks and unit-mass heights (Pearson 0.9 over 3 files, RT within 0.1 min); the mass differences are
//! the lists of the ESI+ adducts and in-source losses that the raw-level companion check shares.

use mzlab_core::step::{DataType, Facets, Param, Port, RunsIn, Step};

use super::FEAT_CEILING;
use crate::companions::list::{ADDUCTS, LOSSES};

fn params() -> Vec<Param> {
    vec![
        Param::number("rt_tol", "RT tolerance")
            .min(0.0)
            .default_num(0.1)
            .unit("min")
            .doc("Largest distance between the RT of two rows that are companions"),
        Param::number("min_corr", "Height correlation")
            .min(-1.0)
            .max(1.0)
            .default_num(0.9)
            .doc("Smallest Pearson correlation of the heights of two rows over the files for an edge; a mass difference alone never gives one"),
        Param::number("weak_corr", "Weak correlation")
            .min(-1.0)
            .max(1.0)
            .default_num(0.5)
            .doc("Pairs whose mass difference fits a relation and whose correlation is between this and the edge level are listed as weak, with the same numbers")
            .advanced(),
        Param::integer("min_corr_samples", "Files for the correlation")
            .min(2.0)
            .default_num(3.0)
            .unit("files")
            .doc("The heights are compared only when both rows are measured in at least this many files")
            .advanced(),
        Param::number("da_tol", "Mass difference tolerance")
            .min(0.0)
            .default_num(0.4)
            .unit("Da")
            .doc("How far the m/z difference of two rows may be from a nominal difference of the lists (the isotope steps 1 and 2 included)"),
        Param::text("adducts", "Adduct differences")
            .default_str(ADDUCTS)
            .doc("Comma-separated nominal differences (Da) from [M+H]+ to its adducts: 17 NH4, 22 Na, 38 K, 41 H+acetonitrile, 63 Na+acetonitrile"),
        Param::text("losses", "In-source loss differences")
            .default_str(LOSSES)
            .doc("Comma-separated nominal differences (Da) between an ion and its in-source fragment: 17 NH3, 18 H2O, 28 CO, 42 C2H2O, 44 CO2, 46 HCOOH"),
        Param::integer("max_edges", "Largest edge count")
            .min(0.0)
            .default_num(20000.0)
            .doc("Edges kept at most; the others are counted in counts.dropped and never silently lost")
            .advanced(),
    ]
}

pub(super) fn companions() -> Step {
    let lr = Facets {
        resolution: vec!["lr"],
        ..Facets::default()
    };
    Step {
        ins: vec![Port::new("table", DataType::Table, "Per-ion feature table at unit resolution").requires(lr)],
        outs: vec![Port::new("companions", DataType::Report, "Companion edges with their evidence (companions/1)")],
        params: params(),
        runs_in: RunsIn {
            wasm: Some("flowtab"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["err.flow.companions.list"],
        ..Step::core(
            "features.companions",
            "Find companion ions",
            "Finds the ions that co-elute with each row of a unit-mass table and differ from it by an isotope step, an adduct or an in-source loss, with the RT offset, correlation and intensity ratio of each pair. Every row stays; companion_of names the parent of a row.",
        )
    }
}
