//! `spectra.chimeric`, `spectra.match` and `spectra.insilico`: what is done to the MS2 spectra of the features. The parameters are the fields of
//! `ReviewSettings` (the keys of `[review]` in `campaign.toml`), `SearchOpts` and `InSilicoOpts`, and the keys of the requests of `ms2_engine::run`.

use mzlab_core::step::{
    Algorithm, Applicability, DataType, Param, Port, Reference, RunsIn, Status, Step,
};

use super::ident::{algorithm, attributio_step, hr, hr_ms2};

/// `spectra.chimeric`: co-isolated ions of each MS2 spectrum (`fill_purity`, `ms2_engine::chimeric::intruders`, `review::raw::ms1_metrics`).
pub(super) fn chimeric() -> Step {
    let features = Port {
        optional: true,
        ..Port::new(
            "features",
            DataType::Table,
            "Aligned feature table (for the intruders)",
        )
        .requires(hr())
    };
    Step {
        ins: vec![
            Port::new("spectra", DataType::Spectra, "MS2 spectra").requires(hr_ms2()),
            Port::new("index", DataType::Mzidx, "Scan index (MS1 around each MS2)").many(),
            features,
        ],
        outs: vec![Port::new("spectra", DataType::Spectra, "Spectra with purity and intruders")],
        algorithms: vec![purity_flag(), intruders(), ms1_window(), library_deconvolution()],
        default_algorithm: Some("purity_flag"),
        errors: vec!["error.memoryBudget"],
        ..attributio_step(
            "spectra.chimeric",
            "Handle chimeric MS2 spectra",
            "Finds the other ions co-isolated in the precursor window of each MS2 spectrum, writes the purity of the precursor and the intruders, and leaves the spectrum as it is: the scores that read it discount the peaks an intruder can explain. Nothing is removed from the spectrum.",
        )
    }
}

fn purity_flag() -> Algorithm {
    algorithm(
        "purity_flag",
        "Purity flag",
        "Share of the MS1 signal in the isolation window that belongs to the precursor (within 5 ppm, PURITY_PPM), taken in the MS1 scans before and after the MS2 scan; the spectrum is kept and flagged (fill_purity in crates/mzlab-feat/src/interp/ms2link.rs).",
        Status::Available,
    )
}

fn intruders() -> Algorithm {
    Algorithm {
        params: vec![
            Param::number("window", "Isolation window")
                .min(0.0)
                .unit("Da")
                .estimate()
                .doc("Absent: the instrument of the campaign decides, 3.0 for a Fusion and 1.5 for the others (default_window); a feature is co-isolated within half of it"),
            Param::number("rt_tol", "RT distance")
                .min(0.0)
                .default_num(0.1)
                .unit("min")
                .doc("Features of the same polarity farther apart in RT are not co-isolated"),
        ],
        ..algorithm(
            "intruders",
            "Intruder features",
            "Lists the features of the same polarity inside the isolation window and within the RT distance, the nearest in m/z first, so that the fragments they explain can be discounted (ms2_engine/chimeric.rs). Evidence for the chemist: no level changes.",
            Status::Available,
        )
    }
}

fn ms1_window() -> Algorithm {
    Algorithm {
        params: vec![
            Param::number("chim_rel", "Co-isolated ion")
                .min(0.0)
                .max(1.0)
                .default_num(0.10)
                .unit("fraction")
                .doc("An ion of the MS1 isolation window this large against the precursor peak is listed as co-isolated"),
            Param::number("chim_cap", "Cap of a proposal")
                .min(0.0)
                .max(1.0)
                .default_num(0.50)
                .unit("fraction")
                .doc("A co-isolated ion this large caps the proposal of the review (propose.rs); the level stays the chemist's"),
        ],
        ..algorithm(
            "ms1_window",
            "MS1 isolation window",
            "Reads the raw MS1 scan of the precursor and lists the ions of the window that are neither the precursor nor its M+1 and M+2 (ms1_metrics in crates/mzlab-feat/src/attributio/review/raw.rs).",
            Status::Available,
        )
    }
}

fn library_deconvolution() -> Algorithm {
    Algorithm {
        references: vec![Reference {
            doi: "10.1038/s41592-021-01195-3",
            citation_key: "stancliffe2021decoid",
            note: "library-assisted deconvolution of chimeric MS2 spectra",
        }],
        applicability: Applicability {
            resolution: vec!["hr"],
            ms_levels: vec![2],
            needs: vec!["library"],
            note: "Data-dependent MS2 with wide windows and a library that covers the intruders",
            ..Applicability::default()
        },
        ..algorithm(
            "library_deconvolution",
            "Library-assisted deconvolution",
            "Explains the spectrum as a non-negative mix of the library spectra of the precursor and of the intruders and returns the residual as the cleaned spectrum. Not written yet.",
            Status::Planned,
        )
    }
}

/// `spectra.match`: scores each query spectrum against a library (`mzlab_lib::search`, `scores.rs`, `ms2_engine::matcher`).
pub(super) fn match_spectra() -> Step {
    Step {
        ins: vec![
            Port::new("query", DataType::Spectra, "Query spectra").requires(hr_ms2()),
            Port::new("library", DataType::Library, "Spectral library"),
        ],
        outs: vec![Port::new("hits", DataType::Candidates, "Library candidates per feature")],
        params: vec![
            Param::number("tol_da", "Fragment tolerance")
                .min(0.0)
                .default_num(0.02)
                .unit("Da")
                .doc("Fragment matching between spectra, the GNPS convention"),
            Param::number("precursor_ppm", "Precursor tolerance")
                .min(0.0)
                .default_num(10.0)
                .unit("ppm")
                .doc("Library precursor against the precursor of the feature"),
            Param::integer("top_n", "Hits kept").min(1.0).default_num(5.0),
        ],
        runs_in: RunsIn {
            wasm: Some("lib"),
            wasm_ceiling_bytes: None,
            native: true,
        },
        algorithms: vec![spectral_entropy(), cosine(), modified_cosine()],
        default_algorithm: Some("spectral_entropy"),
        per_item: true,
        errors: vec!["error.memoryBudget"],
        ..attributio_step(
            "spectra.match",
            "Compare MS2 spectra",
            "Scores each query MS2 spectrum against a spectral library inside the precursor window and keeps the best candidates per feature. A candidate is a hit with its scores and shared peaks, never an identity: the level is decided by annot.level.",
        )
    }
}

fn spectral_entropy() -> Algorithm {
    Algorithm {
        references: vec![Reference {
            doi: "10.1038/s41592-021-01331-z",
            citation_key: "li2021spectral",
            note: "entropy similarity and its intensity weighting",
        }],
        params: vec![
            Param::number("min_score", "Minimum score")
                .min(0.0)
                .max(1.0)
                .default_num(0.6)
                .doc("A hit below this entropy similarity is dropped"),
            Param::integer("min_shared", "Minimum shared ions")
                .min(0.0)
                .default_num(3.0)
                .doc("Matched ions between query and library spectrum; two peaks can score high on a sparse spectrum"),
        ],
        ..algorithm(
            "spectral_entropy",
            "Spectral entropy",
            "Entropy similarity of the merged spectrum after the cleaning of ms_entropy (entropy_from_pairs in crates/mzlab-lib/src/scores.rs); the search of ms2_engine/matcher.rs.",
            Status::Available,
        )
    }
}

fn cosine() -> Algorithm {
    algorithm(
        "cosine",
        "Cosine",
        "One-to-one peak pairs within the tolerance, the highest intensity product first (cosine_from_pairs in crates/mzlab-lib/src/scores.rs); the review reads it with the GNPS preparation (gnps_cosine).",
        Status::Available,
    )
}

fn modified_cosine() -> Algorithm {
    Algorithm {
        references: vec![Reference {
            doi: "10.1073/pnas.1203689109",
            citation_key: "watrous2012mass",
            note: "modified cosine: peaks matched directly or shifted by the precursor mass difference",
        }],
        applicability: Applicability {
            resolution: vec!["hr"],
            note: "Analogues and transformation products of a known parent",
            ..Applicability::default()
        },
        params: vec![
            Param::number("analog_window_da", "Analogue window")
                .min(0.0)
                .default_num(250.0)
                .unit("Da")
                .doc("Library precursors within this distance of the feature are candidate analogues"),
            Param::integer("analog_prefilter", "Candidates scored")
                .min(1.0)
                .default_num(200.0)
                .doc("Candidates that pass the prefilter and are scored with the modified cosine")
                .advanced(),
            Param::number("analog_min_shift_da", "Smallest shift")
                .min(0.0)
                .default_num(0.5)
                .unit("Da")
                .doc("A smaller precursor difference is the same ion, not an analogue"),
            Param::number("exact_ppm", "Exact match")
                .min(0.0)
                .default_num(10.0)
                .unit("ppm")
                .doc("A precursor difference within this is an exact match, not an analogue")
                .advanced(),
            Param::number("delta_formula_mda", "Delta formula")
                .min(0.0)
                .default_num(5.0)
                .unit("mDa")
                .doc("Free search of a CHNOPS formula for the precursor difference")
                .advanced(),
        ],
        ..algorithm(
            "modified_cosine",
            "Modified cosine",
            "Peaks may also pair at the precursor difference, for analogues that differ by one modification (modified_cosine in crates/mzlab-lib/src/scores.rs).",
            Status::Available,
        )
    }
}

/// `spectra.insilico`: ranks the candidate structures of a local list by the fragments they explain (`ms2_engine::insilico`).
pub(super) fn insilico() -> Step {
    Step {
        ins: vec![
            Port::new("spectra", DataType::Spectra, "MS2 spectra").requires(hr_ms2()),
            Port::new("suspects", DataType::Candidates, "Candidate structures (local list)"),
        ],
        outs: vec![Port::new("candidates", DataType::Candidates, "Structures ranked by the fragments they explain")],
        params: vec![
            Param::number("db_ppm", "Mass tolerance")
                .min(0.0)
                .default_num(5.0)
                .unit("ppm")
                .doc("Candidates within this of the neutral mass of the precursor"),
            Param::number("frag_ppm", "Fragment tolerance")
                .min(0.0)
                .default_num(10.0)
                .unit("ppm"),
            Param::integer("max_bonds", "Bonds broken")
                .min(1.0)
                .default_num(200.0)
                .doc("A candidate is broken bond by bond up to this many bonds")
                .advanced(),
            Param::integer("top", "Candidates kept").min(1.0).default_num(5.0),
        ],
        algorithms: vec![fragmenter()],
        default_algorithm: Some("fragmenter"),
        per_item: true,
        errors: vec!["error.memoryBudget"],
        ..attributio_step(
            "spectra.insilico",
            "In silico fragmentation",
            "Breaks each candidate structure of a local list bond by bond and ranks it by the weight of the peaks its fragments explain. The result is a hypothesis with its proofs (peaks explained, mass error), never an identity. No network: the list is read from the campaign folder.",
        )
    }
}

fn fragmenter() -> Algorithm {
    Algorithm {
        references: vec![
            Reference {
                doi: "10.1186/1471-2105-11-148",
                citation_key: "wolf2010in",
                note: "bond-wise in silico fragmentation of candidate structures",
            },
            Reference {
                doi: "10.1186/s13321-016-0115-9",
                citation_key: "ruttkies2016metfrag",
                note: "weights of the explained peaks (mass 0.421, intensity 0.1307)",
            },
        ],
        ..algorithm(
            "fragmenter",
            "Bond-wise fragmenter",
            "Score = sum of mz^0.421 * intensity^0.1307 of the explained peaks over the sum of all the peaks (fragmenter_score in ms2_engine/insilico.rs). Not MetFrag: no bond-energy term and no 0.005 Da + 10 ppm tolerance.",
            Status::Available,
        )
    }
}
