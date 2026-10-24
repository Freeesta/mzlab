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
    let params = vec![
        Param::integer("min_detected", "Minimum files")
            .min(0.0)
            .unit("files")
            .doc("Peaks found by the detection in at least this many files that are not blanks; left empty, the rule is off"),
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
        Param::number("qc_cv_max", "Maximum QC CV")
            .min(0.0)
            .unit("percent")
            .doc("CV of the areas over the files with the role qc is at most this; a row needs a peak in at least the minimum number of them; fewer qc files than that is the error error.qc.tooFew; left empty, the rule is off"),
        Param::integer("qc_min_files", "Minimum QC files")
            .min(2.0)
            .default_num(3.0)
            .unit("files")
            .doc("QC files with a peak that the row needs for the QC rule, and QC files that the series needs"),
        Param::number("dup_ppm", "Duplicate m/z window")
            .min(0.0)
            .unit("ppm")
            .doc("Of the rows within this many ppm in m/z and the duplicate RT window of each other only the one with the largest area is kept; left empty, the rule is off"),
        Param::number("dup_rt", "Duplicate RT window")
            .min(0.0)
            .default_num(0.1)
            .unit("min")
            .doc("RT distance under which two rows may be duplicates; used with the duplicate m/z window only"),
        Param::boolean("require_ms2", "Require MS2").doc("An MS2 is linked to the peak; left empty, the rule is off"),
        Param::choice("normalize", "Normalisation", &["none", "sum", "median"])
            .default_str("none")
            .doc("Scale of the areas of each file: total area or median area"),
    ];
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
            wasm: Some("table"),
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
        Param::list("adducts", "Adducts", "adducts")
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
