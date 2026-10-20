//! `tp.link` and `tp.position`: the MS2 of a product row and the place of its modification. The parameters are the constants of `interp::ms2link` and
//! `interp::sito`, compared with the code in `tests/steps_tabui_parity.rs` (feature `interp`).

use mzlab_core::step::{
    Algorithm, Applicability, DataType, Facets, Param, Port, Reference, Status, Step,
};

use super::tabui::{algorithm, hr, tabui_step, INTERP_CEILING, INTERP_WASM};

/// `tp.link`: every DDA MS2 of the series linked to a row, with the purity of the isolation, and the comparison with the parent (`interp/1`).
pub(super) fn link() -> Step {
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Rows that pass the filters").requires(hr()),
            Port::new("spectra", DataType::Spectra, "DDA MS2 of the series").many().requires(Facets {
                resolution: vec!["hr"],
                ms_levels: vec![2],
                ..Facets::default()
            }),
            Port::new("hypotheses", DataType::Transform, "Modification of each row (best hypothesis)"),
        ],
        outs: vec![Port::new("tp", DataType::Tp, "Rows with their modification, linked MS2 and comparison with the parent")],
        params: vec![
            Param::number("select_ppm", "Isolation centre")
                .min(0.0)
                .default_num(10.0)
                .unit("ppm")
                .doc("An MS2 scan belongs to a row when its isolation centre is within this many ppm of the m/z of the row"),
            Param::number("purity_ppm", "Purity window")
                .min(0.0)
                .default_num(5.0)
                .unit("ppm")
                .advanced(),
            Param::number("purity_min", "Lowest purity")
                .min(0.0)
                .max(1.0)
                .default_num(0.8)
                .doc("A scan below this purity is chimeric and does not vote"),
            Param::number("match_ppm", "Formula tolerance")
                .min(0.0)
                .default_num(5.0)
                .unit("ppm")
                .doc("Tolerance of the modification (neutral formula change) and of the comparison of the fragments with the parent")
                .advanced(),
        ],
        algorithms: vec![pooled_ms2(), isomer_traces()],
        default_algorithm: Some("pooled_ms2"),
        per_item: true,
        references: vec![Reference {
            doi: "10.1186/s13321-020-00477-w",
            citation_key: "helmus2021patroon",
            note: "components linking a parent and its products by MS2 similarity",
        }],
        ..tabui_step(
            "tp.link",
            "Link the MS2 of the products",
            "Gives each row that passes the filters the modification it carries (formula change from the parent and isotope check of the MS1 at the apex), every DDA MS2 of its series with the purity of the isolation, and the comparison of the fragments with the MS2 and the MSn tree of the parent. Evidence for the chemist: it never names the product.",
            INTERP_WASM,
            INTERP_CEILING,
        )
    }
}

fn pooled_ms2() -> Algorithm {
    algorithm(
        "pooled_ms2",
        "Pooled MS2 of the series",
        "Pools the clean scans whose isolation centre lies on the row into one consensus spectrum (select and consensus in crates/mzlab-feat/src/interp/ms2link.rs, as mzFinder pools the MS2).",
        Status::Available,
    )
}

fn isomer_traces() -> Algorithm {
    Algorithm {
        params: vec![
            Param::number("iso_corr", "Fragment correlation")
                .min(-1.0)
                .max(1.0)
                .default_num(0.5)
                .doc("Correlation between the traces of two fragments below which they belong to different isomers"),
            Param::integer("iso_min_scans", "Fewest clean scans")
                .min(1.0)
                .default_num(4.0),
            Param::integer("iso_fragments", "Fragments compared")
                .min(2.0)
                .default_num(5.0),
        ],
        applicability: Applicability {
            ms_levels: vec![2],
            note: "A row with fewer clean scans than the minimum is not split",
            ..Applicability::default()
        },
        ..algorithm(
            "isomer_traces",
            "Isomers from the fragment traces",
            "Compares the traces of the strongest fragments across the scans of one row and says when two groups of fragments rise and fall apart, so two isomers co-elute under one row.",
            Status::Available,
        )
    }
}

/// `tp.position`: the classes of sites of the parent compatible with the MS2 of a product (`interp::sito`, `chem::posizioni`, `posiz/1`).
pub(super) fn position() -> Step {
    Step {
        ins: vec![
            Port::new("tp", DataType::Tp, "Product row with its consensus MS2").requires(hr()),
            Port::new("parent", DataType::Structure, "Parent compound with its structure"),
        ],
        outs: vec![Port::new("sites", DataType::Transform, "Classes of sites with their support and the declared confidence")],
        params: vec![
            Param::number("match_ppm", "Fragment tolerance")
                .min(0.0)
                .default_num(10.0)
                .unit("ppm")
                .doc("Matching of the fragments with the cleavages of the parent (HR spectra)"),
            Param::number("min_rel", "Informative fragment")
                .min(0.0)
                .max(100.0)
                .default_num(5.0)
                .unit("%")
                .doc("A fragment is informative from this share of the base peak, if its formula is within 5 ppm"),
            Param::integer("min_informative", "Fewest informative fragments")
                .min(1.0)
                .default_num(3.0)
                .doc("Fewer informative fragments cannot tell one class from the others"),
            Param::number("margin", "Margin of the lead")
                .min(0.0)
                .default_num(2.0)
                .doc("Log-likelihood units a class must lead the second by to be called alone: two fragments' worth of evidence"),
        ],
        algorithms: vec![msn_tree(), cleavage_likelihood()],
        default_algorithm: Some("msn_tree"),
        per_item: true,
        references: vec![Reference {
            doi: "10.1007/s11419-018-0410-4",
            citation_key: "murakami2018differentiation",
            note: "an MS3 fragment that carries the modification separates positional isomers",
        }],
        ..tabui_step(
            "tp.position",
            "Position of the modification",
            "Says which classes of sites of the parent the MS2 of a product is compatible with, never which site it is: classes are excluded, supported or undistinguished, the sublevel is 3a only when at least one class is excluded and at most three remain, and the rest is 3b. The reactivity of the parent is shown beside as a prior and does not decide.",
            INTERP_WASM,
            INTERP_CEILING,
        )
    }
}

fn msn_tree() -> Algorithm {
    Algorithm {
        references: vec![Reference {
            doi: "10.1007/s11419-018-0410-4",
            citation_key: "murakami2018differentiation",
            note: "MSn of positional isomers",
        }],
        applicability: Applicability {
            needs: vec!["ms2"],
            note: "Votes from the MSn tree of the product when it was recorded (source tree)",
            ..Applicability::default()
        },
        ..algorithm(
            "msn_tree",
            "Votes of the MSn tree",
            "The likelihood of each class is read from the fragments of the MSn tree that carry the modification (votes_of in crates/mzlab-feat/src/interp/sito.rs).",
            Status::Available,
        )
    }
}

fn cleavage_likelihood() -> Algorithm {
    Algorithm {
        params: vec![
            Param::number("ll_support", "Supported fragment")
                .min(0.0)
                .default_num(1.0)
                .doc("Log-likelihood a fragment that a cleavage explains adds to a class"),
            Param::number("ll_against", "Unexplained peak")
                .min(0.0)
                .default_num(2.0)
                .doc("Cost of a peak a class cannot explain, twice the support: a wrong site is worse than an undecided one"),
        ],
        references: vec![Reference {
            doi: "10.1021/ac9911692",
            citation_key: "ramanathan2000liquid",
            note: "aliphatic OH loses water more than phenol; N-oxides give [M+H-O]+: observed losses are evidence shown, not a filter",
        }],
        ..algorithm(
            "cleavage_likelihood",
            "Likelihood from the cleavages",
            "Without the tree, the likelihood of a class is built from the cleavages of the parent that explain the peaks of the MS2 (match_spectrum in crates/mzlab-feat/src/chem/posizioni.rs).",
            Status::Available,
        )
    }
}
