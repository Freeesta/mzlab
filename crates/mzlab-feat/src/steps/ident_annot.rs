//! `annot.level`, `annot.regulatory` and `annot.hazard`: the identification level of each row and the records of the regulatory lists and of the aquatic
//! hazard attached to it. The level parameters are the fields of `annotate::AnnotationSettings` (the keys of `[annotation]` in `campaign.toml`).

use mzlab_core::step::{Algorithm, Applicability, DataType, Param, Port, Reference, Status, Step};

use super::ident::{algorithm, attributio_step, hr};

/// `annot.level`: the Schymanski level of each feature from the evidence of the sources (`annotate::annotate`, scale of `data/livelli-1.json`).
pub(super) fn level() -> Step {
    let optional = |p: Port| Port {
        optional: true,
        ..p
    };
    let mut params = vec![
        Param::number("spec_sim_min", "Library cosine")
            .min(0.0)
            .max(1.0)
            .default_num(0.70)
            .doc("Cosine of a library match, every spectral-library source"),
        Param::integer("spec_peaks_min", "Shared fragments")
            .min(0.0)
            .default_num(4.0)
            .doc("Shared fragment ions of a library match, every library (SIRIUS, GNPS2, mzMine)"),
        Param::choice("library_conflict", "Conflicting libraries", &["best_match", "level3"])
            .default_str("best_match")
            .doc("best_match: the libraries are one library; level3: sources naming different structures give Level 3"),
        Param::number("library_conflict_margin", "Conflict margin")
            .min(0.0)
            .default_num(0.05)
            .doc("The best match keeps Level 2a only if its cosine exceeds the best match of a different structure by this much"),
        Param::number("gnps_mass_tol_ppm", "GNPS mass tolerance")
            .min(0.0)
            .default_num(15.0)
            .unit("ppm"),
        Param::number("mzlib_mass_tol_ppm", "Library mass tolerance")
            .min(0.0)
            .default_num(15.0)
            .unit("ppm"),
        Param::number("cosmic_min", "COSMIC confidence")
            .min(0.0)
            .max(1.0)
            .default_num(0.64)
            .doc("Confidence of the Level 3 gate; 0.64 is the threshold of the COSMIC paper, roughly an FDR of 10 %, a practical guide and no guarantee"),
        Param::number("canopus_min", "Class probability")
            .min(0.0)
            .max(1.0)
            .default_num(0.70)
            .doc("A CANOPUS class is shown, as an annotation of its own that gives no level, at or above this probability"),
        Param::number("rt_group_tol_min", "RT group")
            .min(0.0)
            .default_num(0.20)
            .unit("min")
            .doc("Features of one identity farther apart in RT (consecutive) are different peaks"),
        Param::boolean("putative_library_level3", "Putative library entries")
            .default_bool(false)
            .doc("Library entries named Spectral Match to ... (propagated annotations) are taken to Level 3")
            .advanced(),
    ];
    params.extend(standard_params());
    Step {
        ins: vec![
            Port::new("candidates", DataType::Candidates, "Library, in silico and suspect candidates (candidates/1; screen/1 of screen.suspects gives suspect fragments)").requires(hr()).many(),
            Port::new("formulas", DataType::Formulas, "Candidate formulas"),
            optional(Port::new("standards", DataType::Library, "Your standards, CSV: name, InChIKey, RT in min, optional precursor m/z and adduct: a library hit on one with the RT, score and mass in range is Level 1")),
            optional(Port::new("spectra", DataType::Spectra, "MS2 spectra of the same run (spectra.ms2): the RT of each query, needed for the standards")),
        ],
        outs: vec![Port::new("annot", DataType::Annot, "Chosen annotation with its level and evidence")],
        params,
        algorithms: vec![cascade()],
        default_algorithm: Some("cascade"),
        per_item: true,
        references: vec![Reference {
            doi: "10.1021/es5002105",
            citation_key: "schymanski2014identifying",
            note: "the five-level scale of identification confidence",
        }],
        errors: vec!["err.flow.level.standards"],
        ..attributio_step(
            "annot.level",
            "Identification level",
            "Gives each feature the level its evidence reaches (1, 2a, 2b, 3a, 3b, 4, 5; 0 = detected, not annotated), with the basis and the reasons. Level 1 only with a standards file of yours; suspect fragments of screen.suspects give 3a. Level 2b is never given by the program: the chemist decides it. A CANOPUS class gives no level: it is a separate annotation.",
        )
    }
}

/// The Level 1 thresholds and the suspect fragments of the Flow: they replace the thresholds of the same gates in `data/livelli-1.json`.
fn standard_params() -> Vec<Param> {
    vec![
        Param::number("std_rt_max", "Standard RT window")
            .min(0.0)
            .default_num(0.20)
            .unit("min")
            .doc("Level 1: the feature RT is within this of the RT of the standard you gave"),
        Param::number("std_score_min", "Standard MS2 score")
            .min(0.0)
            .max(1.0)
            .default_num(0.70)
            .doc("Level 1: the MS2 score (entropy similarity, or cosine for a cosine hit) of the hit on the standard"),
        Param::number("std_ppm_max", "Standard mass tolerance")
            .min(0.0)
            .default_num(5.0)
            .unit("ppm")
            .doc("Level 1: the precursor of the hit on the standard"),
        Param::integer("suspect_fragments_min", "Suspect fragments")
            .min(1.0)
            .default_num(2.0)
            .doc("Level 3a: at least this many fragments listed for a suspect are found in the feature MS2"),
    ]
}

fn cascade() -> Algorithm {
    Algorithm {
        references: vec![Reference {
            doi: "10.1038/s41587-021-01045-9",
            citation_key: "hoffmann2021high",
            note: "COSMIC confidence score and its 0.64 threshold",
        }],
        applicability: Applicability {
            resolution: vec!["hr"],
            note: "A level needs MS2 or a standard; at unit resolution the ceiling is Level 4",
            ..Applicability::default()
        },
        ..algorithm(
            "cascade",
            "Cascade of the sources",
            "Reference standard, spectral library, CSI:FingerID with COSMIC, formula, mass, in this order of basis (basis_level in crates/mzlab-feat/src/attributio/annotate/settings.rs; gates in gates.rs, reasons in reasons.rs).",
            Status::Available,
        )
    }
}

/// `annot.regulatory`: membership of the identities in the regulatory and priority lists, and the screening of the lists as suspects.
pub(super) fn regulatory() -> Step {
    Step {
        ins: vec![
            Port::new("annot", DataType::Annot, "Annotated rows").requires(hr()),
            Port::new("lists", DataType::Library, "Regulatory package: a .mzsus with the listings extension (normative.mzsus)"),
            Port {
                optional: true,
                ..Port::new("imported_lists", DataType::Library, "Regulatory lists you imported from an official file: .mzsus files (Library, Import an official file); each is compared whole, apart from the package, and a list never raises a level by itself").many()
            },
        ],
        outs: vec![Port::new("annot", DataType::Annot, "Rows with the list memberships")],
        params: vec![
            Param::pair("window", "Mass window", 0.0)
                .unit("Da")
                .doc("Entries of the lists outside this range of monoisotopic mass are left out; absent: all the entries"),
            Param::text("lists", "Regulatory lists").doc("Ids of the lists to compare, comma-separated; empty: every list in force"),
            Param::text("include", "Also compare").doc("Statuses added to the lists in force: proposal, draft, reference; empty: none"),
        ],
        algorithms: vec![list_membership(), suspect_screen(), ms2_confirm()],
        default_algorithm: Some("list_membership"),
        per_item: true,
        errors: vec!["err.flow.regulatory.package", "err.flow.regulatory.annot", "err.flow.regulatory.list", "err.flow.regulatory.include", "err.flow.regulatory.imported", "err.flow.regulatory.algorithm"],
        ..attributio_step(
            "annot.regulatory",
            "Regulatory lists",
            "Compares the identities with the regulatory package by InChIKey: the same full key is a match, the same first block a flagged match, a name or a formula never. A list entry says what a compound is on a list, nothing about this sample; no network.",
        )
    }
}

fn list_membership() -> Algorithm {
    algorithm(
        "list_membership",
        "List membership",
        "B1: the identities against the entries of the lists by whole InChIKey; a match on the first block only is flagged as such (match_campaign in crates/mzlab-feat/src/attributio/reglists/b1.rs).",
        Status::Available,
    )
}

fn suspect_screen() -> Algorithm {
    algorithm(
        "suspect_screen",
        "Suspect screening",
        "B2: the list entries as suspects by exact mass (5 ppm over [M+H]+, [M+NH4]+, [M+Na]+, [M-H]- and [M+Cl]-), classes A-D and X, and a robust line of RT against logP (2.5 SD) when the registry has XLogP3 (screen_campaign in reglists/screen.rs).",
        Status::Available,
    )
}

fn ms2_confirm() -> Algorithm {
    Algorithm {
        applicability: Applicability {
            resolution: vec!["hr"],
            needs: vec!["ms2"],
            ..Applicability::default()
        },
        ..algorithm(
            "ms2_confirm",
            "MS2 confirmation",
            "B3: a suspect of class A, B or C is Level 2a with a SIRIUS or MetFrag cosine of at least 0.70 on 6 peaks, Level 3 with a COSMIC confidence of at least 0.64, otherwise rejected or unconfirmed; the chemist's decisions win (evaluate in reglists/confirm.rs).",
            Status::Available,
        )
    }
}

/// `annot.hazard`: the aquatic hazard record (acute and chronic) of each identity.
pub(super) fn hazard() -> Step {
    Step {
        ins: vec![Port::new("annot", DataType::Annot, "Annotated rows").requires(hr())],
        outs: vec![Port::new("annot", DataType::Annot, "Rows with the aquatic hazard record")],
        algorithms: vec![tiers()],
        default_algorithm: Some("tiers"),
        per_item: true,
        errors: Vec::new(),
        ..attributio_step(
            "annot.hazard",
            "Aquatic hazard",
            "One acute and one chronic aquatic hazard category per chemical identity, from versioned snapshots read from the campaign folder with their hash, no network. A hazard class is a property of a structure, not a risk: no concentration, no PNEC. A Level 3 identity carries the caveat of its candidate structure.",
        )
    }
}

fn tiers() -> Algorithm {
    algorithm(
        "tiers",
        "Tiers T1 to T5",
        "The first tier with a category wins, acute and chronic separately: T1 regulatory (CLP harmonised, curated), T2 measured (ECOTOX), T3 notified (PubChem GHS H-statements), T4 predicted (curated judgement, then ECOSAR 2.2), T5 not assessable. Degradability is unknown for almost every compound, so every chronic category assumes not rapidly degradable and says so (record in crates/mzlab-feat/src/attributio/tox/hazard.rs).",
        Status::Available,
    )
}
