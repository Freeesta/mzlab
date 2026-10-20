//! The `step/1` descriptors of the Tabui chain (`package: tabui`): `tp.generate` and `tp.filter` here, `tp.link` and `tp.position` in `tabui_link.rs`
//! (`structure.reactivity` is owned by `mzlab-chem`, which this crate cannot see). The defaults of `tp.generate` are read from `HypoParams::default()`;
//! the constants of the `interp` feature are compared in `tests/steps_tabui_parity.rs`. Nothing here is called by a `.wasm`.

use mzlab_core::step::{
    Algorithm, Applicability, DataType, Facets, Param, Port, Reference, RunsIn, Status, Step,
};

use crate::hypo::HypoParams;

pub(super) use super::tabui_link::{link, position};

/// The `.wasm` that runs the hypotheses (the `hr` group of mzFinder) and its fixed ceiling in bytes (`limit_hr` of `tools/build_feat_wasm.sh`).
const HR_WASM: &str = "hr";
const HR_CEILING: u64 = 1_212_416;
/// The `.wasm` of the interpretation of the rows (`limit_interp`).
pub(super) const INTERP_WASM: &str = "interp";
pub(super) const INTERP_CEILING: u64 = 512_000;

/// Every step of the Tabui chain of `mzlab-feat`, in the order of the chain.
pub fn registry() -> Vec<Step> {
    vec![generate(), filter(), link(), position()]
}

/// A step of the `tabui` package that runs in the given `.wasm`.
pub(super) fn tabui_step(
    id: &'static str,
    title: &'static str,
    doc: &'static str,
    wasm: &'static str,
    ceiling: u64,
) -> Step {
    Step {
        package: "tabui",
        runs_in: RunsIn {
            wasm: Some(wasm),
            wasm_ceiling_bytes: Some(ceiling),
            native: true,
        },
        ..Step::core(id, title, doc)
    }
}

/// High resolution data (Tabui names every product by a formula, so unit resolution has no hypothesis to prove).
pub(super) fn hr() -> Facets {
    Facets {
        resolution: vec!["hr"],
        ..Facets::default()
    }
}

/// An algorithm at version 1 without references or parameters; the caller fills them with struct update syntax.
pub(super) fn algorithm(
    id: &'static str,
    title: &'static str,
    doc: &'static str,
    status: Status,
) -> Algorithm {
    Algorithm {
        id,
        version: 1,
        title,
        doc,
        references: Vec::new(),
        applicability: Applicability::default(),
        params: Vec::new(),
        status,
    }
}

/// `tp.generate`: the ions that the chemistry says could be there (`hypo::generate`, registry `ipotesi/1`). A place to look, never a verdict.
fn generate() -> Step {
    let d = HypoParams::default();
    Step {
        ins: vec![
            Port::new("parent", DataType::Structure, "Parent compound (formula, or SMILES for the rules with groups)"),
            Port {
                optional: true,
                ..Port::new("features", DataType::Table, "Feature table (families already found)")
            },
        ],
        outs: vec![Port::new("hypotheses", DataType::Transform, "Hypotheses: formula, m/z, adduct, source and weakest prediction level")],
        params: vec![
            Param::choice_num("polarity", "Polarity", &[1.0, -1.0]).default_num(f64::from(d.polarity)),
            Param::integer("max_steps", "Chained reactions")
                .min(1.0)
                .max(4.0)
                .default_num(f64::from(d.max_steps))
                .doc("Most reactions chained from the parent; one hypothesis per net change of formula, the fewest steps first"),
            Param::number("min_mz", "Lowest m/z")
                .min(0.0)
                .default_num(d.min_mz)
                .unit("m/z"),
            Param::integer("max_suspects", "Most suspects")
                .min(0.0)
                .default_num(d.max_suspects as f64)
                .doc("Hypotheses taken from the suspect index, the best supported first; the number left out is written in the registry")
                .advanced(),
            Param::boolean("other_polarity", "Other polarity too")
                .default_bool(d.other_polarity)
                .doc("Expected members of a family in the other polarity too ([M-H]- of a positive base and the reverse)")
                .advanced(),
            Param::boolean("nominal", "Unit resolution")
                .default_bool(d.nominal)
                .doc("Two formulas of the same nominal m/z and adduct are one hypothesis, the others its alternatives")
                .advanced(),
        ],
        algorithms: vec![rule_chain()],
        default_algorithm: Some("rule_chain"),
        errors: Vec::new(),
        references: vec![Reference {
            doi: "10.1186/s13321-020-00477-w",
            citation_key: "helmus2021patroon",
            note: "transformation products generated from a parent and screened as suspects in a non-target workflow",
        }],
        ..tabui_step(
            "tp.generate",
            "Hypotheses of the products",
            "Lists the ions that the chemistry says could be there, each with the source that proposed it: the parent times the photocatalytic rules (one or two steps), the offline suspect index, the expected members of the families already found, and lists the user typed or imported. One pass, fixed order, byte-identical on every run. A hypothesis is a place to look and what the search finds is graded afterwards.",
            HR_WASM,
            HR_CEILING,
        )
    }
}

fn rule_chain() -> Algorithm {
    Algorithm {
        applicability: Applicability {
            note: "Rules with groups need the structure of the parent; without it the fallback rules are used",
            ..Applicability::default()
        },
        ..algorithm(
            "rule_chain",
            "Rules, suspects, families and lists",
            "Parent times the rules of TP_Mine/data/regole_tp.json (profile of the campaign), merged with the other sources by net formula change; ids follow the order of m/z (crates/mzlab-feat/src/hypo/sources.rs). Prediction level 2 literature, 3 class, 4 generic.",
            Status::Available,
        )
    }
}

/// `tp.filter`: the five filters that hide a row of the table of the products, and the decoys that count the matches by chance (`interp::filtri`).
fn filter() -> Step {
    Step {
        ins: vec![Port::new("table", DataType::Table, "Feature table of the treated samples and the controls").requires(hr())],
        outs: vec![Port::new("table", DataType::Table, "Rows that pass F1-F5, with the filter that hid the others")],
        params: vec![
            Param::number("tol_min", "Lowest formula tolerance")
                .min(0.0)
                .default_num(3.0)
                .unit("ppm")
                .doc("Floor of the tolerance of a formula; the tolerance of a file is its measured error, kept between the two"),
            Param::number("tol_max", "Highest formula tolerance")
                .min(0.0)
                .default_num(5.0)
                .unit("ppm"),
            Param::number("decoy_step", "Decoy mass defect")
                .min(0.0)
                .default_num(0.0137)
                .unit("Da")
                .advanced(),
            Param::integer("decoy_k", "Decoys per rule")
                .min(0.0)
                .default_num(5.0)
                .advanced(),
            Param::number("decoy_ppm", "Decoy tolerance")
                .min(0.0)
                .default_num(10.0)
                .unit("ppm")
                .doc("A decoy lies beyond this many ppm of every true delta, and a row matches a decoy within it")
                .advanced(),
        ],
        algorithms: vec![stages()],
        default_algorithm: Some("stages"),
        per_item: true,
        references: vec![Reference {
            doi: "10.1038/nmeth1019",
            citation_key: "elias2007target",
            note: "decoys give the number of matches by chance",
        }],
        ..tabui_step(
            "tp.filter",
            "Filters of the products",
            "Hides a row for the first filter it fails: F1 peak quality, F2 isotopologue of another row, F3 as strong in the controls as in the treated samples, F4 constant ratio to the parent (in-source fragment or adduct), F5 formula error beyond the tolerance of the file. Rows are marked, not deleted. The decoys (rule deltas shifted by a mass defect) say how many matches are by chance.",
            INTERP_WASM,
            INTERP_CEILING,
        )
    }
}

fn stages() -> Algorithm {
    algorithm(
        "stages",
        "Five stages in a fixed order",
        "The first filter that hides the row wins (first_hidden in crates/mzlab-feat/src/interp/filtri.rs); the registry filtri/1 holds the text and the counts of each stage.",
        Status::Available,
    )
}
