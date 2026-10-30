//! `features.align`, `features.gapfill`, `table.filter` and `export.table_csv`: the fields of `align::AlignParams`, `gapfill::FillParams`,
//! `filters::Rule`, `norm::Method` and the kinds of `export` that the table of the engine reads today.

use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use super::{FEAT_CEILING, FEAT_WASM};

/// The steps that run in the `feat (detect)` module.
fn feat_runs_in() -> RunsIn {
    RunsIn {
        wasm: Some(FEAT_WASM),
        wasm_ceiling_bytes: Some(FEAT_CEILING),
        native: true,
    }
}

/// The parameters of the family link of `famalign::link` (family mode only).
fn family_link_params() -> Vec<Param> {
    vec![
        Param::number("link_rt", "Family link RT")
            .min(0.0)
            .default_num(0.2)
            .unit("min")
            .doc("Family mode: largest RT distance between an ion and the root of its family in one file for the two rows to be joined")
            .advanced(),
        Param::number("max_span", "Family row span")
            .min(0.0)
            .default_num(0.5)
            .unit("min")
            .doc("Family mode: largest RT span of the groups joined in one row")
            .advanced(),
        Param::integer("min_dependent", "Family link files")
            .min(1.0)
            .default_num(1.0)
            .unit("files")
            .doc("Family mode: an ion that is mostly the base of its own family joins a family only if it follows its root in this many files")
            .advanced(),
    ]
}

/// `tol_da` of the steps that match m/z (`features.align`, `features.gapfill`, `features.ms2_link`): the tolerance on the unit axis, in Da.
pub(super) fn tol_da() -> Param {
    Param::number("tol_da", "m/z tolerance (Da)")
        .min(0.05)
        .max(1.0)
        .unit("Da")
        .doc("Unit axis only; left empty, the recipe of the file (measured window or 0.35 Da)")
        .estimate()
        .advanced()
}

/// `features.align`: joins the features of the files into the rows of the table (`famalign::align_families`, `align::align`).
pub(super) fn align() -> Step {
    let mut params = vec![
        Param::choice("mode", "Rows", &["family", "ion"])
            .default_str("family")
            .doc("One row per family of ions (isotopologues, adducts, in-source fragments) or one per ion"),
        Param::number("ppm", "m/z tolerance")
            .min(0.0)
            .default_num(5.0)
            .unit("ppm")
            .doc("At unit resolution the engine reads a negative value as Da"),
        tol_da(),
        Param::number("rt_gap", "RT gap")
            .min(0.0)
            .default_num(0.1)
            .unit("min")
            .doc("Largest gap of RT between neighbouring features of one row"),
        Param::number("max_ppm_span", "Largest m/z span")
            .min(0.0)
            .default_num(10.0)
            .unit("ppm")
            .advanced(),
        Param::boolean("drop_artefacts", "Drop artefacts")
            .default_bool(true)
            .advanced(),
        Param::number("split_gap", "Split gap")
            .min(0.0)
            .default_num(0.03)
            .unit("min")
            .doc("A row with two features of one file (close isomers) is cut at the RT gaps above this")
            .advanced(),
        Param::choice("rt_warp", "RT correction", &["off", "piecewise", "loess", "ransac"])
            .default_str("off")
            .doc("Brings the RT of every file to a common one before the features are grouped, from anchors (peaks every file shows) found by a coarse alignment: piecewise (medians by blocks), loess (local regression), ransac (a line robust to wrong anchors); off: the RT as the files give it"),
        Param::number("rt_coarse_gap", "Coarse RT gap")
            .min(0.0)
            .default_num(0.5)
            .unit("min")
            .doc("RT gap of the coarse alignment that finds the anchors, and the widest spread of an anchor over the files: the drift between files the correction can follow")
            .advanced(),
        Param::number("rt_span", "LOESS span")
            .min(0.05)
            .max(1.0)
            .default_num(0.5)
            .doc("Fraction of the anchors each local fit takes (loess)")
            .advanced(),
        Param::number("rt_inlier", "RANSAC distance")
            .min(0.0)
            .default_num(0.05)
            .unit("min")
            .doc("Largest distance of an anchor from the line to agree with it (ransac)")
            .advanced(),
        Param::integer("weak_files", "Weak tier files")
            .min(0.0)
            .default_num(5.0)
            .unit("files")
            .doc("Features from 0.3 of the minimum height up are kept as rows only if this many files in a run show them; 0 = no weak tier")
            .advanced(),
    ];
    params.extend(family_link_params());
    Step {
        ins: vec![Port::new("peaks", DataType::Peaks, "Features of each file").many()],
        outs: vec![Port::new("table", DataType::Table, "Aligned feature table")],
        params,
        runs_in: feat_runs_in(),
        errors: vec!["error.memoryBudget", "error.newerFormat", "err.file.index"],
        ..Step::core(
            "features.align",
            "Align features",
            "Groups the features of all the files into rows by m/z and RT and writes the aligned table. Today one call (series::align_and_fill) does this and the filling of the gaps.",
        )
    }
}

/// `features.gapfill`: integrates the raw traces where a file has no feature (`gapfill`).
pub(super) fn gapfill() -> Step {
    let params = vec![
        Param::number("floor", "Noise floor")
            .min(0.0)
            .default_num(20_000.0)
            .unit("counts")
            .estimate(),
        Param::number("ppm", "m/z tolerance")
            .min(0.0)
            .default_num(5.0)
            .unit("ppm")
            .doc("At unit resolution the engine reads a negative value as Da"),
        tol_da(),
        Param::number("rt_tol", "RT tolerance")
            .min(0.0)
            .default_num(0.1)
            .unit("min")
            .doc("Half width added to the window of RT"),
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
    ];
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Aligned feature table"),
            Port::new("index", DataType::Mzidx, "Scan index").many(),
        ],
        outs: vec![Port::new("table", DataType::Table, "Table with the gaps filled")],
        params,
        runs_in: feat_runs_in(),
        errors: vec!["error.memoryBudget", "err.file.index"],
        ..Step::core(
            "features.gapfill",
            "Fill gaps",
            "Where a file shows no feature of a row, integrates its raw trace in the window of the row and marks the cell as filled.",
        )
    }
}

/// `table.filter`: the rules of `filters::Rule` and the normalisation of `norm::Method` as a view over the table; the table is not changed.
pub(super) fn filter() -> Step {
    let mut params = vec![
        Param::integer("min_detected", "Minimum files")
            .min(0.0)
            .unit("files")
            .doc("Peaks found by the detection in at least this many files that are not blanks; left empty, the rule is off"),
        Param::number("min_replicate_fraction", "Minimum replicate fraction")
            .min(0.0)
            .max(1.0)
            .unit("fraction")
            .doc("A row is kept if, in at least one group of replicates (same series, role, time and concentration, or the same group when it is set), it is detected in at least this fraction of the files, rounded to a whole file and at least one; left empty, the rule is off")
            .tip("Kept if some replicate group has the row detected in at least this fraction of its files."),
        Param::number("min_area", "Minimum area")
            .min(0.0)
            .unit("counts")
            .doc("The largest area over the files is at least this; left empty, the rule is off"),
        Param::number("min_height", "Minimum height")
            .min(0.0)
            .unit("counts")
            .doc("The largest height over the files is at least this; left empty, the rule is off"),
        Param::number("blank_ratio", "Sample to blank ratio")
            .min(0.0)
            .unit("fraction")
            .doc("The largest area of a sample is at least this many times the mean area of the blanks; needs a blank in the samples; left empty, the rule is off"),
        Param::choice("blank_quantity", "Blank quantity", &["area", "height", "norm_area"])
            .default_str("area")
            .doc("What is compared with the blank: area, height or normalised area; used with the sample to blank ratio only"),
        Param::choice("blank_stat", "Blank level", &["mean", "max"])
            .default_str("mean")
            .doc("The blank level of a row: mean or maximum over the blank files (a blank without a peak counts as 0)"),
        Param::choice("blank_scope", "Blank comparison", &["max_feature", "each_feature"])
            .default_str("max_feature")
            .doc("max_feature judges the row by its best sample; each_feature keeps it only if every sample with a peak reaches the threshold"),
        Param::boolean("blank_subtract", "Subtract the blank")
            .doc("The view shows sample minus ratio times the blank level, not below 0, instead of only filtering; area or normalised area only; the ratio is at least 1 when any blank option is set"),
        Param::choice("blank_match", "Blank matching", &["all", "batch", "matrix"])
            .default_str("all")
            .doc("all compares every sample with all blanks; batch or matrix compares each sample only with the blanks of its batch or matrix (campioni/2 or /3), and a sample with none falls back to all blanks, listed per file in the counts as bianco_ripiego; not with the subtraction")
            .tip("Compare each sample only with the blanks of its batch or matrix; others use all blanks."),
        Param::boolean("blank_keep_istd", "Keep the internal standard")
            .doc("The internal-standard row skips the blank rule, because it is spiked in the blanks too; needs the normalisation istd"),
        Param::boolean("qc_zero", "Count missing QC as 0")
            .doc("The QC CV counts the QC files without a peak as 0 and the row needs no minimum of them; the series still needs the minimum number of qc files"),
        Param::number("qc_cv_max", "Maximum QC CV")
            .min(0.0)
            .unit("percent")
            .doc("CV of the areas over the files with the role qc is at most this; a row needs a peak in at least the minimum number of them; fewer qc files than that is the error error.qc.tooFew; left empty, the rule is off"),
        Param::integer("qc_min_files", "Minimum QC files")
            .min(2.0)
            .default_num(3.0)
            .unit("files")
            .doc("QC files with a peak that the row needs for the QC rule, and QC files that the series needs"),
    ];
    params.extend(filter_tail_params());
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Feature table"),
            Port::new("samples", DataType::Samples, "Roles and times of the files"),
        ],
        outs: vec![Port::new("table", DataType::Table, "Table view with the rows kept")],
        params,
        runs_in: RunsIn {
            wasm: Some("table"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["error.badView", "error.badCampioni", "error.newerFormat", "error.qc.tooFew"],
        ..Step::core(
            "table.filter",
            "Filter table",
            "Keeps the rows that pass an ordered list of rules over the samples (campioni/2); blanks and dark files never count as samples, and a rule that cannot be evaluated is an error.",
        )
    }
}

/// The last parameters of `table.filter`: duplicates, MS2, normalisation and its internal standard.
fn filter_tail_params() -> Vec<Param> {
    vec![
        Param::number("dup_ppm", "Duplicate m/z window")
            .min(0.0)
            .unit("ppm")
            .doc("Of the rows within this many ppm in m/z and the duplicate RT window of each other only the one with the largest area is kept; left empty, the rule is off"),
        Param::number("dup_da", "Duplicate m/z window (Da)")
            .min(0.05)
            .max(1.0)
            .unit("Da")
            .doc("Unit axis only: of the rows within this many Da in m/z and the duplicate RT window of each other only the one with the largest area is kept; dup_ppm on a unit-resolution table is an error; left empty, the rule is off")
            .tip("Of rows this many Da apart in m/z and close in RT, only the strongest is kept."),
        Param::number("dup_rt", "Duplicate RT window")
            .min(0.0)
            .default_num(0.1)
            .unit("min")
            .doc("RT distance under which two rows may be duplicates; used with the duplicate m/z window only"),
        Param::boolean("require_ms2", "Require MS2").doc("An MS2 is linked to the peak; left empty, the rule is off"),
        Param::choice("normalize", "Normalisation", &["none", "sum", "median", "istd"])
            .default_str("none")
            .doc("Scale of the areas of each file: total area or median area")
            .tip("Scale each file by its total area, its median area or the area of the internal standard."),
        Param::integer("istd_id", "Internal standard row")
            .min(0.0)
            .doc("Id of the row of the internal standard; used with the normalisation istd"),
        Param::number("istd_mz", "Internal standard m/z")
            .min(0.0)
            .doc("m/z of the internal standard, with its RT: the nearest row (5 ppm, 0.2 min) is the standard; used with the normalisation istd when no row id is set")
            .tip("m/z of the internal standard; with its RT, the nearest row is the standard."),
        Param::number("istd_rt", "Internal standard RT")
            .min(0.0)
            .unit("min")
            .doc("RT of the internal standard; used with its m/z"),
    ]
}

/// `table.drift`: the correction of the drift of the areas along the order of injection (`drift::correct`), a view over the table.
pub(super) fn drift() -> Step {
    let params = vec![
        Param::choice("method", "Curve", &["linear", "loess"])
            .default_str("linear")
            .doc("Line or local line (LOESS) through the areas of the QC files of each batch, against the position of the injection"),
        Param::number("span", "LOESS span")
            .min(0.1)
            .max(1.0)
            .default_num(0.75)
            .unit("fraction")
            .doc("Share of the QC files of a batch in the neighbourhood of each fit; used by the local line only"),
    ];
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Feature table"),
            Port::new("samples", DataType::Samples, "Roles and injection order of the files"),
            Port::new("design", DataType::Design, "Batches of the files"),
        ],
        outs: vec![Port::new("table", DataType::Table, "Table view with a factor for every cell")],
        params,
        runs_in: RunsIn {
            wasm: Some("flowtab"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["error.badView", "error.drift.tooFewQc", "error.drift.qcNotAtEnds"],
        ..Step::core(
            "table.drift",
            "Correct drift",
            "Brings the areas of each batch to the level of the QC files along the order of injection; refused with fewer than 5 QC files in a batch or QC files that are not at the start and the end of it. The measured areas are never overwritten.",
        )
    }
}

/// `export.table_csv`: the table file of `export::csv_full` and its siblings.
pub(super) fn export_csv() -> Step {
    let params = vec![Param::choice(
        "kind",
        "Format",
        &["full", "full_mzmine", "quant", "csv_semicolon"],
    )
    .default_str("full")
    .doc("full: one row per feature with the summaries; full_mzmine: the same with the words of mzmine for the filled and targeted cells; quant: the areas for FBMN; csv_semicolon: the sheet of features with ;")];
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Feature table"),
            Port::new("samples", DataType::Samples, "Roles and times of the files"),
        ],
        outs: vec![Port::new("report", DataType::Report, "Table as CSV")],
        params,
        runs_in: RunsIn {
            wasm: Some("table"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["error.badView", "error.badCampioni", "error.newerFormat"],
        ..Step::core(
            "export.table_csv",
            "Export table as CSV",
            "Writes the feature table, under the view of the filter step when one is wired, as a CSV file; the column names carry the file names, so equal names give equal bytes.",
        )
    }
}

/// `export.inclusion`: the inclusion list of the products expected in the plan, to inject the samples again in targeted MS2 (`export_inclusione`).
pub(super) fn export_inclusion() -> Step {
    let params = vec![
        Param {
            list: Some("adducts"),
            ..Param::text("adducts", "Adducts")
        }
        .default_str("[M+H]+")
            .doc("Adducts of the ion table separated by commas, or a list of your own as a CSV with the header name;delta;charge;comment; one row per product and adduct"),
        Param::number("rt_window", "RT half window")
            .min(0.0)
            .default_num(0.5)
            .unit("min")
            .doc("Each row spans the expected RT of the product plus and minus this; the start is never below 0"),
    ];
    Step {
        ins: vec![Port::new("tp", DataType::Tp, "Expected products with their RT")],
        outs: vec![Port::new("report", DataType::Report, "Inclusion list as CSV")],
        params,
        runs_in: RunsIn {
            wasm: Some("table"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["error.export.adduct", "error.newerFormat"],
        ..Step::core(
            "export.inclusion",
            "Export inclusion list",
            "Writes the m/z and RT windows of the expected products as a CSV in the columns of the inclusion list of mzFinder, sorted by RT; it adds no level and no verdict.",
        )
    }
}

/// `export.mztab`: the table as mzTab-M 2.0 (`export_mztab`), a format that other programs read.
pub(super) fn export_mztab() -> Step {
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Feature table"),
            Port::new("samples", DataType::Samples, "Roles and times of the files"),
        ],
        outs: vec![Port::new("report", DataType::Report, "Table as mzTab-M")],
        runs_in: RunsIn {
            wasm: Some("table"),
            wasm_ceiling_bytes: Some(FEAT_CEILING),
            native: true,
        },
        errors: vec!["error.badView", "error.badCampioni", "error.newerFormat"],
        ..Step::core(
            "export.mztab",
            "Export table as mzTab-M",
            "Writes the feature table, under the view of the filter step when one is wired, as an mzTab-M 2.0 file: one small molecule and one feature per row, areas per file and per group, and an evidence for a row whose identity names a hypothesis. Every row is reliability 4 (unknown compound): the export adds no identity and raises no level.",
        )
    }
}
