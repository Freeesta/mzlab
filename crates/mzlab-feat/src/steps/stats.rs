//! Descriptors of the statistics steps over the feature table (`stat/`): `table.stats`, `table.pca`, `table.kmd`. They run native only: the `.wasm` `table`
//! does not carry them. Their defaults are those of `StatsOpts`, `PcaOpts` and `KmdOpts` (checked by `tests/stat.rs`).

use crate::plausible::Limits;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

fn native() -> RunsIn {
    RunsIn {
        wasm: None,
        wasm_ceiling_bytes: None,
        native: true,
    }
}

/// `table.stats`: Welch's t per comparison, one-way ANOVA, BH q-values and log2FC with a pseudocount (`stat::compare`).
pub(super) fn stats() -> Step {
    let params = vec![
        Param::number("pseudocount", "Pseudocount")
            .min(0.0)
            .unit("area")
            .doc("Added to every area before the logarithm and in the log2FC; without a value it is 3 x the median of the positive areas of the blanks, else of the reference, else the floor"),
        Param::number("floor", "Pseudocount floor")
            .min(0.0)
            .default_num(1000.0)
            .unit("area")
            .advanced()
            .doc("Smallest estimated pseudocount, and the one used when no file gives a noise level"),
    ];
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Feature table"),
            Port::new("samples", DataType::Samples, "Roles of the files"),
            Port::new("design", DataType::Design, "Conditions, reference and comparisons"),
        ],
        outs: vec![Port::new("report", DataType::Report, "Tests per comparison, ANOVA and heat map data (stat/1)")],
        params,
        runs_in: native(),
        errors: vec!["error.badStatInput", "error.stats.param", "error.stats.noReference", "error.stats.condition"],
        ..Step::core(
            "table.stats",
            "Compare conditions",
            "Welch's t on log2(area + pseudocount) for each comparison of the design and a one-way ANOVA over all the conditions, with Benjamini-Hochberg q-values and log2FC; refused with a reason and no number when a condition has fewer than 3 files. The p-values stay in this output: no view shows them.",
        )
    }
}

/// `table.pca`: the exploratory PCA of the table with the stability of the loadings (`stat::pca`).
pub(super) fn pca() -> Step {
    let params = vec![
        Param::number("factor", "Blank factor")
            .min(0.0)
            .default_num(3.0)
            .doc("A feature must exceed the blank level times this factor; skipped when no file is a blank"),
        Param::choice("stat", "Blank level", &["max", "mean"])
            .default_str("max")
            .doc("Level of the blank files: the maximum or the mean of their areas"),
        Param::integer("min_detect", "Files above the blank")
            .min(1.0)
            .default_num(1.0)
            .doc("Sample files that must exceed the blank threshold"),
        Param::number("min_intensity", "Minimum area")
            .min(0.0)
            .default_num(0.0)
            .unit("area")
            .doc("Smallest maximum area of a feature, and the area that counts as present"),
        Param::integer("min_samples", "Minimum samples present")
            .min(0.0)
            .default_num(2.0)
            .doc("Samples in which a feature must be present"),
        Param::choice("transform", "Transform", &["log", "none"])
            .default_str("log")
            .doc("log2 of the areas after the imputation, or the areas as they are"),
        Param::choice("scaling", "Scaling", &["pareto", "auto", "none"])
            .default_str("pareto")
            .doc("Pareto divides by the square root of the sd; auto divides by the sd and inflates the noise with few samples"),
        Param::integer("ncomp", "Components")
            .min(2.0)
            .max(20.0)
            .default_num(5.0)
            .doc("Components kept in scores and loadings"),
    ];
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Feature table"),
            Port::new("samples", DataType::Samples, "Roles of the files"),
        ],
        outs: vec![Port::new("report", DataType::Report, "Scores, loadings, explained variance, stability and ellipses")],
        params,
        runs_in: native(),
        errors: vec!["error.badStatInput", "err.stat.pca.few", "err.stat.pca.nofeatures", "err.stat.pca.param", "err.stat.blank.param"],
        ..Step::core(
            "table.pca",
            "Explore with PCA",
            "Principal components of the samples after the blank filter, the imputation and the scaling, with the variance explained and the stability of the loadings leaving one sample out; refused under 5 samples. A view: it tests nothing.",
        )
    }
}

/// `table.kmd`: the Kendrick mass defect and the candidate series of a repeating unit (`stat::analyse`).
pub(super) fn kmd() -> Step {
    let params = vec![
        Param::choice("unit", "Repeating unit", &["CF2", "CF2O", "C2F4", "CH2"])
            .default_str("CF2")
            .doc("Unit of the Kendrick mass"),
        Param::number("tol_mda", "KMD window")
            .min(0.0)
            .default_num(3.0)
            .unit("mDa")
            .doc("Width of the KMD window of a series"),
        Param::integer("min_members", "Minimum members")
            .min(2.0)
            .default_num(3.0)
            .doc("Distinct nominal Kendrick masses a series needs"),
    ];
    Step {
        ins: vec![Port::new("table", DataType::Table, "Feature table")],
        outs: vec![Port::new("report", DataType::Report, "Kendrick values per ion and the candidate series")],
        params,
        runs_in: native(),
        errors: vec!["error.badStatInput"],
        ..Step::core(
            "table.kmd",
            "Kendrick mass defect",
            "KM and KMD of every ion and the series of ions that share the KMD, with their members, steps and retention trend. A series is a candidate with its evidence, never an identification.",
        )
    }
}

/// `table.plausible`: the plausibility of the values the table shows (`plausible::to_json`); the defaults are those of `plausible::Limits`.
pub(super) fn plausible() -> Step {
    let d = Limits::default();
    let params = vec![
        Param::number("mz_min", "Lowest m/z").min(0.0).default_num(d.mz_min).unit("m/z").doc("A row below it is flagged: no LC-MS run reads there"),
        Param::number("mz_max", "Highest m/z").min(0.0).default_num(d.mz_max).unit("m/z").doc("A row above it is flagged"),
        Param::number("rt_max", "Longest run").min(0.0).default_num(d.rt_max).unit("min").doc("A row with an RT outside 0 and this value is flagged: check the unit of the times"),
        Param::number("width_max", "Widest peak").min(0.0).default_num(d.width_max).unit("min").advanced().doc("A detected peak wider than this, or with no width or apex, is flagged"),
        Param::number("ppm_max", "Largest ppm error").min(0.0).default_num(d.ppm_max).unit("ppm").doc("A row whose m/z is further than this from its neutral mass plus adduct is flagged; read only in a high-resolution table"),
        Param::number("mz_tol_da", "Alignment tolerance")
            .min(0.0)
            .unit("Da")
            .advanced()
            .doc("0.1 Da or more is unit resolution and the ppm error is not read; without a value the recipe stored in the table decides"),
    ];
    Step {
        ins: vec![Port::new("table", DataType::Table, "Feature table")],
        outs: vec![Port::new("report", DataType::Report, "Plausibility of the values (plausible/1)")],
        params,
        runs_in: RunsIn {
            wasm: Some("table"),
            wasm_ceiling_bytes: Some(super::FEAT_CEILING),
            native: true,
        },
        errors: vec!["error.badView"],
        ..Step::core(
            "table.plausible",
            "Plausibility of the values",
            "Seven checks of the numbers the table shows: finite values, m/z and RT inside a real run, no negative area or height, a width and an apex for every detected peak, rows with no peak in any file and the ppm error of the identity columns. Numbers only: a flagged row is a value to look at, never a verdict.",
        )
    }
}
