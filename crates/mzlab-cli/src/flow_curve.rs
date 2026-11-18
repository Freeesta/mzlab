//! `quant.curve@1`: fits calibration lines to the integrated peaks of the standards and back-calculates standards and unknowns, with LOD/LOQ and a
//! log of every point left out. The texts are `mzlab_ops::run::table::curve::curve_text`, shared with the browser (`flowcal`); this file only reads
//! and writes the files. Called by `flow.rs`.

use mzlab_core::calib::Weight;
use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};
use mzlab_ops::run::table::curve::curve_text;

use crate::flow::{io, read_one, FileRunner, Made};

/// The descriptor of `quant.curve@1`: responses and standards CSV in, the `calibration/1` JSON and the concentrations CSV out.
pub fn curve_step() -> Step {
    Step {
        ins: vec![
            Port::new("responses", DataType::Report, "Integrated peaks as CSV: sample, name (or analyte, label, channel), area, height, snr"),
            Port::new("standards", DataType::Report, "Standards table as CSV: sample, level, concentration, unit, role; optional analyte, dilution"),
        ],
        outs: vec![
            Port::new("calibration", DataType::Report, "Lines, back-calculated standards, LOD/LOQ and the log of choices (calibration/1)").format("calibration/1"),
            Port::new("concentrations", DataType::Report, "Concentration of every analyte in every sample as CSV").format("csv"),
        ],
        params: vec![
            Param::choice("signal", "Signal of the peak", &["area", "height"]).default_str("area"),
            Param::choice("model", "Calibration model", &["linear", "quadratic"])
                .default_str("linear")
                .doc("quadratic: the ICH LOD/LOQ (3.3σ/b, 10σ/b) use b, the slope at zero concentration."),
            Param::choice("weight", "Weighting of the standards", &Weight::NAMES)
                .default_str("1/x")
                .doc("1/x and 1/x2 leave out a standard at zero concentration; the log says so."),
            Param::boolean("zero", "Force the line through zero")
                .default_bool(false)
                .doc("true or false; any other value fails the step."),
            Param::text("istd", "Internal standard")
                .default_str("")
                .doc("One name for every analyte, or analyte|IS pairs separated by semicolons; empty = none."),
            Param::text("exclude", "Points left out of the fit")
                .default_str("")
                .doc("analyte|sample|reason separated by semicolons; * as analyte = every analyte; each one is logged."),
        ],
        runs_in: RunsIn {
            wasm: Some("flowcal"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec![
            "err.flow.curve.responses",
            "err.flow.curve.standards",
            "err.flow.curve.row",
            "err.flow.curve.duplicate",
            "err.flow.curve.role",
            "err.flow.curve.param",
            "err.flow.curve.unit",
            "err.flow.curve.points",
            "err.flow.curve.singular",
            "err.flow.curve.value",
        ],
        ..Step::core(
            "quant.curve",
            "Calibration curve",
            "Fits weighted calibration lines to the standards, back-calculates every standard and unknown, gives LOD and LOQ.",
        )
    }
}

pub(crate) fn curve_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let (responses, standards) = (
        read_one(r, job, "responses")?,
        read_one(r, job, "standards")?,
    );
    let (json, csv, warnings) = curve_text(&job.resolved.params, &responses.1, &standards.1)?;
    let id = &job.instance.id;
    let (jpath, cpath) = (
        r.work.join(format!("{id}.json")),
        r.work.join(format!("{id}.csv")),
    );
    std::fs::write(&jpath, json).map_err(io)?;
    std::fs::write(&cpath, csv).map_err(io)?;
    let mut made = Made::new("calibration", r.keep(jpath, DataType::Report)?);
    made.extra = Some(("concentrations", r.keep(cpath, DataType::Report)?));
    made.warnings = warnings;
    Ok(made)
}

#[cfg(test)]
#[path = "flow_curve_tests.rs"]
mod tests;
