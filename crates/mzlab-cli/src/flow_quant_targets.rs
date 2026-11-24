//! `quant.targets@1`: a table of (m/z, RT window) targets integrated in the scan index of every file, one pass per file
//! (`mzlab_feat::quant_targets::Job`, as `mzlab-cli quant targets` runs it, files on a pool of threads), the long table as CSV and the log of every choice as JSON.
//! Called by `flow.rs`; in the browser the same engine runs in the `.wasm` `quanttargets` (`flow-web-quanttargets.js`, parity: `tests/test_quant_targets_wasm.py`).

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::json::Value;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow::{all_inputs, io, read_one, Exec, FileRunner, Made};

pub(crate) const EXECUTORS: &[(&str, Exec)] = &[("quant.targets", Exec::Made(targets_job))];

/// The descriptor of `quant.targets@1`.
pub fn quant_targets_step() -> Step {
    Step {
        ins: vec![
            Port::new("raw", DataType::Mzidx, "Scan index of each file (.mzidx)").many(),
            Port::new("targets", DataType::Report, "Targets as CSV or JSON: id, mz, rt_min, rt_max, optional polarity (+ or -) and role (target, standard or parent)")
                .formats(&["csv", "json"]),
        ],
        outs: vec![
            Port::new("table", DataType::Report, "One row per file and target: area, height, S/N, apex RT, flag, state (detected, forced or absent) and the evidence of the peak (CSV)").format("csv"),
            Port::new("log", DataType::Report, "Parameters, the m/z offset of every file with its evidence, automatic choices and warnings (quant-targets-log/1)")
                .format("quant-targets-log/1"),
        ],
        params: vec![
            Param::number("da", "XIC half window").min(0.001).max(5.0).default_num(0.5).unit("Da").doc("Every trace sums the signal within this of the target m/z (after the offset)"),
            Param::number("min_sn", "Min. S/N").min(0.0).default_num(3.0)
                .doc("A peak under this is reported with its numbers and flagged below_threshold; it does not count as detected"),
            Param::integer("min_points", "Min. points per peak").min(1.0).default_num(5.0)
                .doc("A peak with fewer points between its edges is flagged bad_shape"),
            Param::number("min_shape", "Min. Gaussian shape").min(0.0).max(1.0).default_num(0.5)
                .doc("A peak whose correlation with a Gaussian of the same apex and width is under this is flagged bad_shape"),
            Param::text("offset", "m/z offset").default_str("0")
                .doc("Observed minus true m/z: 0, a number of Da (|D| <= 0.6), auto (measured in each file and polarity on the first standard or parent row; none: 0, logged) or auto:MZ[@RT]")
                .tip("Observed minus true m/z: 0, a number of Da, or auto to measure it on a standard."),
            Param::text("blank_pattern", "Blank files").default_str("")
                .doc("Comma list of case-insensitive globs over the file names; a target detected in a blank flags the samples in_blank"),
            Param::integer("ms_level", "MS level").min(1.0).max(10.0).default_num(1.0).advanced(),
            Param::integer("smooth_half", "Smoothing half window").min(0.0).max(30.0).default_num(0.0).unit("points")
                .doc("Savitzky-Golay half window of the detection copy (peak auto-lr); 0 = automatic per trace, written in the smooth_half column")
                .advanced(),
            Param::number("valley", "Valley depth").min(0.0).max(100.0).default_num(10.0).unit("%")
                .doc("Two peaks are split when the valley between them is under this % of the lower apex (peak auto-lr)")
                .advanced(),
        ],
        runs_in: RunsIn {
            native: true,
            wasm: Some("quanttargets"),
            ..RunsIn::default()
        },
        errors: vec![
            "err.quant.targets.table",
            "err.quant.targets.empty",
            "err.quant.targets.param",
            "err.quant.targets.noFiles",
        ],
        ..Step::core(
            "quant.targets",
            "Target list",
            "Integrates every (m/z, RT window) target of a table in every file with the peak engine of peak auto-lr, one pass over each file: area, height, S/N, apex RT, flags (no_peak, below_threshold, bad_shape, edge, in_blank, no_scans) and a state (detected, forced, absent). Every row is written; a peak under the thresholds keeps its numbers and its flag.",
        )
    }
}

fn targets_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let raw = job
        .inputs
        .iter()
        .find(|(n, _)| n == "raw")
        .map(|(_, v)| v.as_slice())
        .unwrap_or_default();
    let paths = all_inputs(r, job, "raw");
    let names: Vec<String> = raw.iter().map(|a| r.source_name(a)).collect();
    let targets = read_one(r, job, "targets")?.1;
    let params: Vec<(String, Value)> = job.resolved.params.clone();
    // one pass per file on the pool (at most MAX_WORKERS at once, each with its share of the memory budget), joined in file order: the
    // bytes of one worker and of many are the same
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, mzlab_feat::quant_targets::MAX_WORKERS);
    let q = mzlab_feat::quant_targets::prepare(&names, &targets, &params)?.with_workers(workers);
    let passes = crate::par::par_map_with(workers, q.files(), |k| match paths.get(k) {
        Some(path) => q.file(k, crate::flow_base::open(path)?),
        None => Err(Error::new("err.flow.port.missing").with("port", "raw")),
    });
    let out = q.finish(passes.into_iter().collect::<Result<Vec<_>>>()?);
    let csv = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&csv, &out.csv).map_err(io)?;
    let log = r.work.join(format!("{}.log.json", job.instance.id));
    std::fs::write(&log, &out.log).map_err(io)?;
    let mut made = Made::new("table", r.keep(csv, DataType::Report)?);
    made.extra = Some(("log", r.keep(log, DataType::Report)?));
    made.warnings = out.warnings;
    Ok(made)
}
