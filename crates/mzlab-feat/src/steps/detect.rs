//! `features.estimate`, `features.detect` and `features.ms2_link`: the parameters of `trace::Params`, `params::estimate` and `ms2::Ms2Params`.

use mzlab_core::step::{
    Algorithm, Applicability, DataType, Facets, Param, Port, Preset, Reference, RunsIn, Status,
    Step,
};

use mzlab_core::json::Value;

use super::{FEAT_CEILING, FEAT_WASM};

fn feat_runs_in() -> RunsIn {
    RunsIn {
        wasm: Some(FEAT_WASM),
        wasm_ceiling_bytes: Some(FEAT_CEILING),
        native: true,
    }
}

fn ms1() -> Facets {
    Facets {
        ms_levels: vec![1],
        ..Facets::default()
    }
}

fn polarity() -> Param {
    Param::choice_num("polarity", "Polarity", &[1.0, -1.0, 0.0])
        .default_num(1.0)
        .doc("1 positive, -1 negative, 0 all MS1 scans")
}

fn axis() -> Param {
    Param::choice("axis", "m/z axis", &["ppm", "Da"])
        .default_str("ppm")
        .advanced()
}

pub(super) fn estimate() -> Step {
    Step {
        ins: vec![Port::new("index", DataType::Mzidx, "Scan index")
            .many()
            .requires(ms1())],
        outs: vec![Port::new("estimates", DataType::Qc, "Measured parameters and their proofs").many()],
        params: vec![polarity(), axis()],
        runs_in: feat_runs_in(),
        errors: vec!["error.memoryBudget", "err.file.index"],
        per_item: true,
        ..Step::core(
            "features.estimate",
            "Estimate parameters",
            "Measures the noise floor, the m/z tolerance, the peak width and the dead time of each file from its own data and writes the proofs; it proposes values, the next step decides.",
        )
    }
}

pub(super) fn detect() -> Step {
    let params = vec![
        polarity(),
        Param::number("floor", "Noise floor")
            .min(0.0)
            .default_num(20_000.0)
            .unit("counts")
            .estimate()
            .doc("Points below are ignored; left empty, it is estimated from the intensity histogram"),
        Param::number("min_height", "Minimum height")
            .min(0.0)
            .default_num(200_000.0)
            .unit("counts"),
        axis(),
        Param::number("mass_defect", "Mass defect")
            .unit("Da")
            .estimate()
            .doc("Unit axis only: fractional part of m/z where the strong peaks pile up; measured on the file when left empty")
            .advanced(),
        Param::integer("budget", "Working memory")
            .min(1.0)
            .default_num(96_000_000.0)
            .unit("bytes")
            .noresult()
            .advanced(),
    ];
    Step {
        cv: Some("MS:1000035"),
        ins: vec![Port::new("index", DataType::Mzidx, "Scan index")
            .many()
            .requires(ms1())],
        outs: vec![Port::new("peaks", DataType::Peaks, "Features of each file").many()],
        params,
        runs_in: feat_runs_in(),
        errors: vec!["error.memoryBudget", "err.file.index", "error.badRecipe"],
        algorithms: algorithms(),
        default_algorithm: Some("masstrace"),
        per_item: true,
        presets: vec![preset_hr(), preset_lr()],
        default_preset: Some("hr"),
        ..Step::core(
            "features.detect",
            "Detect features",
            "Finds the chromatographic peaks of the MS1 scans of each file, with their quality, isotopologues and families. The parameters are common to every algorithm; each algorithm adds its own.",
        )
    }
}

fn preset_hr() -> Preset {
    Preset {
        id: "hr",
        title: "High resolution",
        when: Facets {
            resolution: vec!["hr"],
            ..Facets::default()
        },
        params: vec![
            ("axis", text("ppm")),
            ("floor", num(20_000.0)),
            ("min_height", num(200_000.0)),
        ],
        evidence: "defaults of trace::Params",
        algorithm: Some("masstrace"),
    }
}

fn preset_lr() -> Preset {
    Preset {
        id: "lr",
        title: "Unit resolution",
        when: Facets {
            resolution: vec!["lr"],
            ..Facets::default()
        },
        params: vec![
            ("axis", text("Da")),
            ("floor", num(2_000.0)),
            ("min_height", num(20_000.0)),
        ],
        evidence: "LR_FLOOR and LR_MIN_HEIGHT of recipe_lr.rs; the preview proposes the values of each file",
        algorithm: Some("masstrace"),
    }
}

fn algorithms() -> Vec<Algorithm> {
    vec![masstrace(), centwave(), threshold()]
}

fn masstrace() -> Algorithm {
    Algorithm {
        id: "masstrace",
        version: 1,
        title: "Mass traces",
        doc: "Builds mass traces scan by scan and cuts them at the minima; defaults and fields of trace::Params in crates/mzlab-feat/src/trace.rs.",
        references: Vec::new(),
        applicability: Applicability {
            resolution: vec!["hr", "lr"],
            spectrum_mode: vec!["centroid", "profile"],
            ..Applicability::default()
        },
        params: vec![
            Param::integer("min_points", "Minimum points")
                .min(2.0)
                .default_num(5.0)
                .advanced(),
            Param::integer("max_gap", "Maximum gap")
                .min(0.0)
                .default_num(2.0)
                .unit("scans")
                .advanced(),
            Param::integer("long_trace", "Long trace")
                .min(1.0)
                .default_num(400.0)
                .unit("scans")
                .doc("A trace longer than this is background and is not taken")
                .advanced(),
        ],
        status: Status::Available,
    }
}

fn centwave() -> Algorithm {
    Algorithm {
        id: "centwave",
        version: 1,
        title: "CentWave",
        doc: "Regions of interest in m/z, then a continuous wavelet transform over the expected peak widths.",
        references: vec![Reference {
            doi: "10.1186/1471-2105-9-504",
            citation_key: "tautenhahn2008highly",
            note: "CentWave: ROI detection and the wavelet peak model",
        }],
        applicability: Applicability {
            resolution: vec!["hr"],
            spectrum_mode: vec!["centroid"],
            peak_fwhm_s: Some((2.0, 60.0)),
            note: "High resolution centroided data with a known range of peak widths",
        },
        params: vec![
            Param::number("ppm", "m/z deviation").min(0.0).default_num(25.0).unit("ppm"),
            Param::pair("peak_width_s", "Peak width", 0.0)
                .default_pair(5.0, 30.0)
                .unit("s"),
            Param::number("snr", "Signal to noise").min(0.0).default_num(10.0),
        ],
        status: Status::Planned,
    }
}

fn threshold() -> Algorithm {
    Algorithm {
        id: "threshold",
        version: 1,
        title: "Threshold and derivative",
        doc: "Smooths each extracted chromatogram and keeps the apexes where the first derivative changes sign above the floor; fits unit resolution and few points per peak.",
        references: Vec::new(),
        applicability: Applicability {
            resolution: vec!["lr"],
            note: "Unit resolution full scan or MRM with few points per peak",
            ..Applicability::default()
        },
        params: vec![Param::integer("smooth", "Smoothing window")
            .min(1.0)
            .default_num(5.0)
            .unit("scans")],
        status: Status::Planned,
    }
}

/// `features.ms2_link`: the MS2 scans of each peak with the purity of the precursor (`ms2::link`, run today inside `pipeline::analyse`).
pub(super) fn ms2_link() -> Step {
    let ms2 = Facets {
        ms_levels: vec![2],
        ..Facets::default()
    };
    Step {
        ins: vec![
            Port::new("index", DataType::Mzidx, "Scan index").many().requires(ms2),
            Port::new("peaks", DataType::Peaks, "Features of each file").many(),
        ],
        outs: vec![Port::new("peaks", DataType::Peaks, "Features with their MS2 scans").many()],
        params: vec![
            Param::number("default_half_window", "Default isolation half window")
                .min(0.0)
                .default_num(0.6)
                .unit("m/z")
                .doc("Used when the index records no isolation window; the peak is then marked as estimated")
                .advanced(),
            Param::number("tol_ppm", "m/z tolerance")
                .min(0.0)
                .default_num(6.0)
                .unit("ppm")
                .doc("Window of the peak in the MS1 scan of the precursor; at unit resolution the engine reads a negative value as Da"),
            Param::number("weak", "Weak purity")
                .min(0.0)
                .max(1.0)
                .default_num(0.5)
                .unit("fraction")
                .doc("Purity below which an MS2 is weak evidence; it is not dropped"),
        ],
        runs_in: feat_runs_in(),
        errors: vec!["error.memoryBudget", "err.file.index"],
        per_item: true,
        ..Step::core(
            "features.ms2_link",
            "Link MS2 scans",
            "Attaches to each peak the MS2 scans whose precursor lies in the peak and in the isolation window, with the purity of the precursor.",
        )
    }
}

fn num(v: f64) -> Value {
    Value::Num(v)
}

fn text(v: &str) -> Value {
    Value::Str(v.to_string())
}
