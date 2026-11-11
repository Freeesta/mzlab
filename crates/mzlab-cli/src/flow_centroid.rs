//! The native executor of `spectra.centroid@1`: an mzML with profile scans becomes an mzML with one point per peak (`mzlab_core::centroids`, the
//! conversion of the archive), the step before `index.mzidx` for a profile file. An mzML with no profile scan goes through byte for byte.

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use mzlab_core::centroids::{mzml_to_centroids_with, Estimate, Over};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::Value;
use mzlab_core::pace::Pace;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow::{io, FileRunner, Lineage, Made};
use crate::flow_pace::{Stop, Stopping};

/// `spectra.centroid@1`: a width left out is measured on at most 64 profile scans spread over the file (reported as an estimate only when measured).
pub fn centroid_step() -> Step {
    Step {
        cv: Some("MS:1000035"),
        ins: vec![Port::new("source", DataType::Mzml, "mzML, profile or centroids")
            .many()
            .accepts(&[DataType::Raw])],
        outs: vec![Port::new("mzml", DataType::Mzml, "mzML with one point per peak").many()],
        params: vec![
            Param::number("fwhm", "Peak width")
                .min(0.0)
                .unit("Da")
                .doc("FWHM of a peak; left out, it is measured on the isolated peaks of the file")
                .estimate(),
            Param::number("threshold", "Threshold")
                .min(0.0)
                .unit("sigma")
                .default_num(3.0)
                .doc("A peak is kept when it stands this many sigma over the background (Poisson Z for counts); left out, the value of the mode of the file"),
        ],
        runs_in: RunsIn {
            wasm: Some("wasip1 (mzlab-cli)"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.file.xml", "err.file.numpress", "error.memoryBudget"],
        per_item: true,
        cost: Some(1.0),
        ..Step::core(
            "spectra.centroid",
            "Centroid the spectra",
            "Reduces every profile scan of an mzML to one point per peak with the centroider of the archive; centroid scans, chromatograms and a file with no profile scan are left as they are.",
        )
    }
}

fn number(job: &Job, name: &str) -> Option<f64> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

/// `spectra.centroid`: the centroid mzML of one file, named after the file for the steps that follow.
pub(crate) fn job(r: &FileRunner, job: &Job) -> Result<Made> {
    let gate = r.gate(job);
    let mut pace = gate.pace();
    job_with(r, job, &mut pace, &gate.stop)
}

/// `job` telling `step(done, total)` how many scans it has written, at most once per 1 %; the file made is the same with any `step`. With the flag `stop`
/// up it ends at the next piece it reads or writes with `err.flow.cancelled` and leaves no file.
pub(crate) fn job_with(
    r: &FileRunner,
    job: &Job,
    step: &mut dyn FnMut(usize, usize),
    stop: &Stop,
) -> Result<Made> {
    let input = &job.inputs[0].1[0];
    let src = r.path(input);
    let name = r.source_name(input);
    let out = r.work.join(crate::flow_base::convert_file(
        &job.instance.id,
        job.item_root.as_deref().unwrap_or("0"),
    ));
    let over = Over {
        fwhm: number(job, "fwhm"),
        threshold: number(job, "threshold"),
    };
    let mut pace = Pace::new(step);
    let written = File::create(&out).map_err(io).and_then(|f| {
        let mut w = BufWriter::new(Stopping::new(f, stop.clone()));
        let used = mzml_to_centroids_with(
            || {
                let f = File::open(&src).map_err(io)?;
                Ok(BufReader::new(Stopping::new(f, stop.clone())))
            },
            &mut w,
            &name,
            over,
            &mut |d, t| pace.at(d, t),
        )?;
        w.flush().map_err(io)?;
        Ok(used)
    });
    let used = match written {
        Ok(u) => u,
        Err(e) => {
            let _ = std::fs::remove_file(&out);
            return Err(e);
        }
    };
    if used.is_none() {
        std::fs::copy(&src, &out).map_err(io)?;
    }
    let a = r.keep(out, DataType::Mzml)?;
    r.trace(
        &a,
        Lineage {
            name,
            instance: job.instance.id.clone(),
            item: job.item,
            ..Lineage::default()
        },
    );
    Ok(made_of(job, a, used))
}

/// Where the width used comes from: given in the plan, measured on the file, or the fallback (too few isolated peaks).
fn width_basis(given: bool, e: &Estimate) -> &'static str {
    match (given, e.peaks) {
        (true, _) => "given",
        (false, 0) => "fallback",
        _ => "measured",
    }
}

/// The file made and what the record says of it: `centroided` (with the values used and where the width comes from), or `passthrough` when the file had no
/// profile scan. Only a measured width is an estimate; a fallback width is never called estimated and raises `warn.centroid.widthFallback`.
fn made_of(job: &Job, a: Artifact, used: Option<Estimate>) -> Made {
    let mut made = Made::new("mzml", a);
    let Some(e) = used else {
        made.file = Some(Value::Obj(vec![(
            "via".into(),
            Value::Str("passthrough".into()),
        )]));
        return made;
    };
    let basis = width_basis(number(job, "fwhm").is_some(), &e);
    match basis {
        "measured" => made.estimates.push((
            "fwhm".into(),
            Value::Num(e.params.w),
            Value::Str(format!(
                "median FWHM of {} isolated peaks of the profile scans of the file",
                e.peaks
            )),
        )),
        "fallback" => made.warnings.push("warn.centroid.widthFallback".into()),
        _ => {}
    }
    made.file = Some(Value::Obj(vec![
        ("via".into(), Value::Str("centroided".into())),
        ("fwhm".into(), Value::Num(e.params.w)),
        ("fwhm_basis".into(), Value::Str(basis.into())),
        ("threshold".into(), Value::Num(e.params.k)),
    ]));
    made
}
