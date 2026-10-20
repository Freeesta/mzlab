//! `formula.ms1`, `formula.ms2` and `rt.predict`: the molecular formula from the MS1 and from the MS2 spectrum, and the plausibility of a retention time. The
//! parameters are the fields of `untargeted::formula_id::Opts`, of `ReviewSettings` (keys of `[review]` in `campaign.toml`) and of `RtModel::train`.

use mzlab_core::step::{
    Algorithm, Applicability, DataType, Param, Port, Reference, RunsIn, Status, Step,
};

use super::ident::{algorithm, attributio_step, hr, hr_ms2};

/// The `.wasm` `untargeted` that runs `formula_id`, and its fixed ceiling (`limit` of `tools/build_feat_wasm.sh`).
const UNTARGETED_WASM: &str = "untargeted";
const UNTARGETED_CEILING: u64 = 512_000;

/// The ions `mzlab_core::composition::compose` knows (`ADDUCT_ATOMS`).
const IONS: [&str; 11] = [
    "[M+H]+",
    "[M+NH4]+",
    "[M+Na]+",
    "[M+K]+",
    "[M-H]-",
    "[M+Cl]-",
    "[M+HCOO]-",
    "M+.",
    "M-.",
    "[M]+",
    "[M]-",
];

/// `formula.ms1`: candidate formulas from the exact mass and the isotope pattern of the MS1 at the apex (`untargeted::formula_id::identify`).
pub(super) fn ms1() -> Step {
    let params = vec![
        Param::choice("ion", "Ion", &IONS).default_str("[M+H]+"),
        Param::integer("z", "Charge")
            .min(1.0)
            .estimate()
            .doc("Absent: the isotope finder decides, and 1 when it finds no pattern; above 1 the formulas are those of the ion itself"),
        Param::number("tol_ppm", "m/z tolerance")
            .min(0.0)
            .default_num(5.0)
            .unit("ppm"),
        Param::text("elements", "Elements")
            .estimate()
            .doc("Ranges as C:0-50,H:0-100. Absent: C:0-50,H:0-100,N:0-8,O:0-15,P:0-2,S:0-3 plus the heavy elements the finder sees in the spectrum; the text given is searched as written"),
        Param::choice("nitrogen", "Nitrogen rule", &["none", "odd", "even"]).default_str("none"),
        Param::number("rdb_min", "Smallest RDB").default_num(0.0),
        Param::integer("max_results", "Formulas kept").min(1.0).default_num(10.0),
        Param::integer("max_charge", "Highest charge tried")
            .min(1.0)
            .default_num(3.0)
            .doc("Charges the isotope finder looks at")
            .advanced(),
        Param::boolean("require_c13", "Require 13C peak")
            .default_bool(true)
            .doc("The isotope finder accepts a pattern only with the 13C peak")
            .advanced(),
    ];
    Step {
        ins: vec![
            Port::new("features", DataType::Table, "Aligned feature table")
                .accepts(&[DataType::Report])
                .requires(hr()),
            Port::new("index", DataType::Mzidx, "Scan index (MS1 at the apex)")
                .many()
                .accepts(&[DataType::Mzml]),
        ],
        outs: vec![Port::new("formulas", DataType::Formulas, "Ranked formulas per feature")],
        params,
        runs_in: RunsIn {
            wasm: Some(UNTARGETED_WASM),
            wasm_ceiling_bytes: Some(UNTARGETED_CEILING),
            native: true,
        },
        algorithms: vec![compose_isotopes()],
        default_algorithm: Some("compose_isotopes"),
        errors: vec!["err.formulaId.param", "err.formulaId.finder", "err.comp.ion"],
        ..attributio_step(
            "formula.ms1",
            "Formula from MS1",
            "Candidate molecular formulas of each feature from the exact mass and, when the isotope finder finds a pattern, from the isotope pattern. The list is ordered by score and by a confidence that is a share of the list, never an identification: the chosen formula is the chemist's.",
        )
    }
}

fn compose_isotopes() -> Algorithm {
    Algorithm {
        references: vec![Reference {
            doi: "10.1186/1471-2105-8-105",
            citation_key: "kind2007seven",
            note: "heuristic rules that filter molecular formulas of accurate mass (RDB, nitrogen rule, element ratios)",
        }],
        ..algorithm(
            "compose_isotopes",
            "Mass and isotope pattern",
            "Exact-mass search over the element ranges (compose in crates/mzlab-core/src/composition.rs), scored on the mass error and on the fit of the isotope pattern that the finder takes from the spectrum (untargeted/formula_id.rs). Without a pattern only the mass term counts.",
            Status::Available,
        )
    }
}

/// `formula.ms2`: explains the peaks of an MS2 spectrum with sub-formulas of the ion of a formula (`review::explain`, `ms2_engine::rearrange`).
pub(super) fn ms2() -> Step {
    Step {
        ins: vec![
            Port::new("spectra", DataType::Spectra, "MS2 spectra").requires(hr_ms2()),
            Port::new("formulas", DataType::Formulas, "Candidate formulas"),
        ],
        outs: vec![Port::new("formulas", DataType::Formulas, "Formulas with the peaks they explain")],
        params: vec![
            Param::number("ppm_ion", "Precursor tolerance")
                .min(0.0)
                .default_num(5.0)
                .unit("ppm")
                .doc("Precursor against the ion of the formula, the adduct with the smallest error of the polarity"),
            Param::number("ppm_frag", "Fragment tolerance")
                .min(0.0)
                .default_num(10.0)
                .unit("ppm")
                .doc("Fragment against a sub-formula of the ion (the MS2 tolerance of SIRIUS)"),
            Param::integer("max_subformulas", "Sub-formula limit")
                .min(1.0)
                .default_num(3_000_000.0)
                .doc("Above this many sub-formulas the table is not built")
                .advanced(),
        ],
        algorithms: vec![subformula(), h_rearrangement()],
        default_algorithm: Some("subformula"),
        per_item: true,
        errors: vec!["error.memoryBudget"],
        ..attributio_step(
            "formula.ms2",
            "Formula from MS2",
            "Assigns to each cleaned peak the closest sub-formula of the ion of the candidate formula and gives the neutral losses. A sub-formula supports the FORMULA, never the structure: isomers share it.",
        )
    }
}

fn subformula() -> Algorithm {
    algorithm(
        "subformula",
        "Sub-formulas",
        "Closest sub-formula of the ion within ppm_frag (RDBE at least -0.5; a Na or K adduct keeps its cation in every fragment). The same peaks shifted by 20, 35 and 50 mDa in both directions are assigned again: the share explained by chance (decoy) is reported (review/explain.rs).",
        Status::Available,
    )
}

fn h_rearrangement() -> Algorithm {
    Algorithm {
        references: vec![Reference {
            doi: "10.1021/acs.analchem.6b00770",
            citation_key: "tsugawa2016hydrogen",
            note: "hydrogen rearrangements between the two sides of a bond break",
        }],
        ..algorithm(
            "h_rearrangement",
            "Hydrogen rearrangements",
            "A peak that no sub-formula explains is tried again shifted by +1, -1, +2 and -2 hydrogens, the fragment taking those hydrogens. Score = sum of intensity * (1 - 0.25 * |shift|) over the sum of intensities (attribute in ms2_engine/rearrange.rs). The penalty and the score are our choice, not those of MS-FINDER.",
            Status::Available,
        )
    }
}

/// `rt.predict`: the plausibility of the retention time of a candidate (`review::rtlogp`, `ms2_engine::rtpredict`).
pub(super) fn rt_predict() -> Step {
    Step {
        ins: vec![
            Port::new("candidates", DataType::Candidates, "Candidates with a structure").requires(hr()),
            Port::new("standards", DataType::Table, "Identified features of the campaign with their RT"),
        ],
        outs: vec![Port::new("candidates", DataType::Candidates, "Candidates with the RT evidence")],
        algorithms: vec![logp_window(), forest()],
        default_algorithm: Some("logp_window"),
        per_item: true,
        errors: vec!["error.memoryBudget"],
        ..attributio_step(
            "rt.predict",
            "Retention time plausibility",
            "Compares the retention time of a candidate with what the campaign's own confident identifications predict. A warning aid that ranks or doubts a candidate: it never applies a level and does not tell isomers apart.",
        )
    }
}

fn logp_window() -> Algorithm {
    Algorithm {
        params: vec![
            Param::number("rt_logp_k", "Window width")
                .min(0.0)
                .default_num(3.0)
                .doc("Prediction window = plus or minus this many times the residual MAD"),
            Param::number("rt_logp_floor", "Window floor")
                .min(0.0)
                .default_num(0.0)
                .unit("min")
                .doc("0 = 3 %B with a gradient, 1.0 min without"),
            Param::number("lc_dwell_min", "Gradient delay")
                .min(0.0)
                .default_num(0.0)
                .unit("min")
                .doc("Dwell volume over flow; the gradient itself ([[t, %B], ...]) is read from the campaign")
                .advanced(),
        ],
        ..algorithm(
            "logp_window",
            "RT against logP",
            "Calibrates RT against the logP of the Level 1 and robust Level 2a features of one polarity, one logP source per fit, at least 15 calibrants (MIN_N), and checks a candidate against the window (calibrate and check in crates/mzlab-feat/src/attributio/review/rtlogp.rs).",
            Status::Available,
        )
    }
}

fn forest() -> Algorithm {
    Algorithm {
        references: vec![
            Reference {
                doi: "10.1023/A:1010933404324",
                citation_key: "breiman2001random",
                note: "regression forest: bootstrap samples, random split search",
            },
            Reference {
                doi: "10.1021/ci100050t",
                citation_key: "rogers2010extended",
                note: "circular (Morgan) fingerprint of the structure graph",
            },
        ],
        applicability: Applicability {
            resolution: vec!["hr"],
            note: "At least 10 standards whose structure is read; the model follows the column and gradient of the campaign",
            ..Applicability::default()
        },
        params: vec![Param::integer("seed", "Seed")
            .min(0.0)
            .default_num(42.0)
            .doc("Seed of the bootstrap; 100 trees over a 2048-bit fingerprint of radius 2 (N_TREES, N_BITS)")
            .advanced()],
        ..algorithm(
            "forest",
            "Random forest",
            "Random forest of regression trees on a Morgan-type fingerprint with our own hash (not the bits of RDKit), trained on the standards of the campaign (RtModel in ms2_engine/rtpredict.rs).",
            Status::Available,
        )
    }
}
