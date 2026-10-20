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

/// `features.align`: joins the features of the files into the rows of the table (`famalign::align_families`, `align::align`).
pub(super) fn align() -> Step {
    let params = vec![
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
        Param::integer("weak_files", "Weak tier files")
            .min(0.0)
            .default_num(5.0)
            .unit("files")
            .doc("Features from 0.3 of the minimum height up are kept as rows only if this many files in a run show them; 0 = no weak tier")
            .advanced(),
    ];
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
        errors: vec!["error.badView", "error.badCampioni", "error.newerFormat"],
        ..Step::core(
            "table.filter",
            "Filter table",
            "Keeps the rows that pass an ordered list of rules over the samples (campioni/2); blanks and dark files never count as samples, and a rule that cannot be evaluated is an error.",
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
        Param::text("adducts", "Adducts")
            .default_str("[M+H]+")
            .doc("Adducts of the ion table separated by commas; one row per product and adduct"),
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
