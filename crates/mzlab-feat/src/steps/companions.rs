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
            .doc("Smallest height correlation over the files for an edge; mass alone never gives one"),
        Param::number("weak_corr", "Weak correlation")
            .min(-1.0)
            .max(1.0)
            .default_num(0.5)
            .doc("Pairs with a fitting mass difference and a correlation between this and the edge level are listed as weak")
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
            .doc("How far an m/z difference may be from a nominal difference of the lists"),
        Param::text("adducts", "Adduct differences")
            .default_str(ADDUCTS)
            .doc("Comma-separated nominal differences (Da) from [M+H]+ to its adducts: 17 NH4, 22 Na, 38 K, 41 H+acetonitrile, 63 Na+acetonitrile"),
        Param::text("losses", "In-source loss differences")
            .default_str(LOSSES)
            .doc("Comma-separated nominal differences (Da) of in-source fragments: 17 NH3, 18 H2O, 28 CO, 42 C2H2O, 44 CO2, 46 HCOOH"),
        Param::boolean("iso_group", "Group isotopes")
            .default_bool(false)
            .doc("Adds an isotopes section: M, M+1 and M+2 rows grouped by the rules below, and a +1 channel that holds spill_min to spill_max of M flagged as spill-over instead of an isotope")
            .tip("Groups M, M+1 and M+2 into isotopes and flags a spill-over +1 channel.")
            .advanced(),
        Param::number("iso_min_corr", "Isotope correlation floor")
            .min(-1.0)
            .max(1.0)
            .default_num(0.5)
            .doc("An ion at M+1 or M+2 whose height correlation with M is below this is never an isotope")
            .advanced(),
        Param::number("iso_ratio_min", "Smallest M+1/M")
            .min(0.0)
            .default_num(0.01)
            .doc("A +1 ion below this ratio to M is not grouped as an isotope")
            .advanced(),
        Param::number("iso_ratio_max", "Largest M+1/M")
            .min(0.0)
            .default_num(0.5)
            .doc("A +1 ion above this ratio to M (and outside the spill-over band) is not grouped as an isotope")
            .advanced(),
        Param::number("iso_m2_max", "Largest M+2/M")
            .min(0.0)
            .default_num(1.0)
            .doc("A +2 ion above this ratio to M is not grouped as an isotope")
            .advanced(),
        Param::number("spill_min", "Spill-over from")
            .min(0.0)
            .default_num(0.5)
            .doc("A correlated +1 channel holding at least this share of M is flagged as instrument spill-over, with its ratio and correlation")
            .tip("A correlated +1 channel with at least this share of M is flagged as spill-over.")
            .advanced(),
        Param::number("spill_max", "Spill-over up to")
            .min(0.0)
            .default_num(0.8)
            .doc("Upper end of the spill-over band of the +1 to M ratio")
            .advanced(),
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
