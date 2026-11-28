//! `quant.kinetics@1`: a first-order rate per ion and series from the areas of fixed RT windows (`quant.targets`), the times of `samples/1` and, when
//! wired, the UV peaks of a reference (`chrom.uv_peaks`) as the injection check. The text is `mzlab_ops::run::table::kinetics::kinetics_text`, shared
//! with the browser (`flowkin`); this file only reads and writes the files. Called by `flow.rs`.

use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};
use mzlab_ops::run::table::kinetics::kinetics_text;
use mzlab_ops::run::table::kinetics_read::{DEFAULT_BOOT, DEFAULT_UV_RT_TOL, DEFAULT_UV_TOL};

use crate::flow::{io, read_all, read_one, Exec, FileRunner, Made};

pub(crate) const EXECUTORS: &[(&str, Exec)] = &[("quant.kinetics", Exec::Made(kinetics_job))];

/// The descriptor of `quant.kinetics@1`: areas CSV, samples and optional UV peaks in, the `kinetics-series/1` JSON out.
pub fn kinetics_step() -> Step {
    Step {
        ins: vec![
            Port::new("areas", DataType::Report, "Areas per file and ion as CSV: file (or sample), id (or analyte, name), area; optional state (the table of quant.targets)"),
            Port::new("samples", DataType::Samples, "Roles, times and series of the files (a samples/1 document); t0 files join every series"),
            Port { optional: true, ..Port::new("uv", DataType::Peaks, "UV peaks of the files (chrom.uv_peaks): the peak of the reference, one per file, is the injection check") },
        ],
        outs: vec![Port::new("kinetics", DataType::Report, "Normalisation factor per file, the points, the first-order fit of every ion and series with its interval, the choices and the warnings (kinetics-series/1)")
            .format("kinetics-series/1")],
        params: vec![
            Param::text("ids", "Ions to fit")
                .default_str("")
                .doc("Ids of the areas table separated by semicolons; empty = every id."),
            Param::boolean("use_t0", "Start every series at t0")
                .default_bool(true)
                .doc("The files with role t0 join every series at their time (0 when the samples give none; logged)."),
            Param::choice("missing", "A file with no peak", &["skip", "zero"])
                .default_str("skip")
                .doc("skip: the point is left out and counted; zero: its area is 0. Either way the points list says so."),
            Param::choice("uv_mode", "UV reference", &["normalise", "check"])
                .default_str("normalise")
                .doc("normalise: every area is divided by the UV area of its file over the median UV area; check: the factors are reported and the areas stay as they are. Without the uv input nothing is divided."),
            Param::number("uv_rt", "UV reference RT")
                .min(0.0)
                .default_num(0.0)
                .unit("min")
                .doc("Apex of the UV peak that does not react (an internal reference, or the parent in a dark control); 0 = the median apex of the largest peak of each file, logged."),
            Param::number("uv_rt_tol", "UV RT tolerance")
                .min(0.0)
                .default_num(DEFAULT_UV_RT_TOL)
                .unit("min")
                .doc("A file whose UV peak is farther than this from the reference RT has no reference peak."),
            Param::number("uv_tol", "UV factor tolerance")
                .min(0.0)
                .default_num(DEFAULT_UV_TOL)
                .unit("%")
                .doc("A file whose factor differs from 1 by more than this is flagged uv_off and warned; it is never dropped."),
            Param::integer("boot", "Bootstrap resamples")
                .min(10.0)
                .default_num(DEFAULT_BOOT as f64)
                .doc("Residual bootstrap of k for its 95 % interval."),
            Param::integer("seed", "Bootstrap seed").min(0.0).default_num(0.0).advanced(),
        ],
        runs_in: RunsIn {
            wasm: Some("flowkin"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec![
            "err.flow.kinetics.areas",
            "err.flow.kinetics.uv",
            "err.flow.kinetics.row",
            "err.flow.kinetics.param",
            "err.flow.kinetics.samples",
        ],
        ..Step::core(
            "quant.kinetics",
            "First-order rate",
            "Joins the areas of fixed RT windows to the times of the series, divides them by the UV area of a reference peak per file when a UV table is wired, and fits a first-order decay per ion and series: k, its 95 % interval, half-life and R2, with the factor of every file and every choice in the output.",
        )
    }
}

fn kinetics_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let (areas, samples) = (read_one(r, job, "areas")?, read_one(r, job, "samples")?);
    let uv = read_all(r, job, "uv")?;
    let uv = uv.first().map_or(&[][..], |u| &u.1[..]);
    let (json, warnings) = kinetics_text(&job.resolved.params, &areas.1, &samples.1, uv)?;
    let path = r.work.join(format!("{}.json", job.instance.id));
    std::fs::write(&path, json).map_err(io)?;
    let mut made = Made::new("kinetics", r.keep(path, DataType::Report)?);
    made.warnings = warnings;
    Ok(made)
}

#[cfg(test)]
#[path = "flow_kinetics_tests.rs"]
mod tests;
