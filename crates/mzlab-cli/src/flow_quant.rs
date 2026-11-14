//! `quant.calibrate@1`: applies the calibration curves the user imported (slope, intercept, linear range per compound, made in a spreadsheet) to the
//! areas of a feature table and writes a long CSV of concentrations with a flag per cell. It never fits a line: the standards are not read.
//! The text is `mzlab_feat_table_wasm::quant::quant_text`, shared with the browser (`table`); this file only reads and writes the files. Called by `flow.rs`.

use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};
use mzlab_feat_table_wasm::quant::{quant_text, DEFAULT_MZ_TOL, DEFAULT_RT_TOL};

use crate::flow::{io, read_one, FileRunner, Made};

/// The descriptor of `quant.calibrate@1`: feature table, curves CSV and samples in, the concentrations as CSV out.
pub fn quant_step() -> Step {
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Feature table (.mzfeat)"),
            Port::new("curves", DataType::Report, "Calibration curves imported as CSV: name, mz, rt, slope, intercept, unit, range_min, range_max, dilution"),
            Port::new("samples", DataType::Samples, "Roles of the files (campioni/1 or campioni/2)"),
        ],
        outs: vec![Port::new("concentrations", DataType::Report, "Concentration of each compound in each sample as CSV")],
        params: vec![
            Param::number("mz_tol", "m/z tolerance to match a curve to a row")
                .default_num(DEFAULT_MZ_TOL)
                .min(0.0)
                .unit("Da"),
            Param::number("rt_tol", "RT tolerance to match a curve to a row")
                .default_num(DEFAULT_RT_TOL)
                .min(0.0)
                .unit("min"),
            Param::choice("signal", "Signal the curve was made with", &["area", "height"]).default_str("area"),
            Param::text("kinds", "Sample types to quantify, comma separated").default_str("sample"),
        ],
        runs_in: RunsIn {
            wasm: Some("table"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec![
            "err.flow.quant.curves",
            "err.flow.quant.curveRow",
            "err.flow.quant.duplicate",
            "err.flow.quant.kind",
        ],
        ..Step::core(
            "quant.calibrate",
            "Calibrate",
            "Applies imported calibration curves (signal = slope x concentration + intercept) to the signal of the matched rows of the table; concentrations outside the linear range or below zero are flagged. The line is not fitted here.",
        )
    }
}

pub(crate) fn quant_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let (table, curves, samples) = (
        read_one(r, job, "table")?,
        read_one(r, job, "curves")?,
        read_one(r, job, "samples")?,
    );
    let (text, warnings) = quant_text(&job.resolved.params, &table.1, &curves.1, &samples.1)?;
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    let mut made = Made::new("concentrations", r.keep(out, DataType::Report)?);
    made.warnings = warnings;
    Ok(made)
}

#[cfg(test)]
#[path = "flow_quant_tests.rs"]
mod tests;
