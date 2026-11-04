//! The feature steps run on files: `convert.mzml`, `features.table` and the base chain of the Flow canvas (`index.mzidx`, `features.detect`,
//! `features.align`, `features.gapfill`). The recipe of the engine is
//! the one the Pipeline panel takes with no setting (`run::recipe_for`: the profile of the first file of the series) with the values the plan writes on
//! top (`steps::recipe_map`); a value the plan leaves out that differs from the canonical one is reported as an estimate. Called by `flow.rs`.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use mzlab_core::budget::MemBudget;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job, Mem};
use mzlab_core::hashio::HashWrite;
use mzlab_core::idxread::Index;
use mzlab_core::lettori::Limiti;
use mzlab_core::lettori_mzml::{Conversione, Picchi};
use mzlab_core::mzidx::DEFAULT_BUDGET;
use mzlab_core::mzmlwrite::{Opzioni, LIVELLO_ZLIB};
use mzlab_core::step::DataType;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::{self, Stored};
use mzlab_feat_wasm::run;
use mzlab_feat_wasm::run::meta::{recipe, written, Estimate};

#[cfg(test)]
#[path = "flow_record_tests.rs"]
mod flow_record_tests;

use crate::flow::{io, FileRunner, Lineage, Made};
use crate::flow_pace::{Gate, Gated, Stop, Stopping};

pub(crate) fn open(path: &PathBuf) -> Result<Index<File>> {
    let f = File::open(path).map_err(io)?;
    let size = f.metadata().map_err(io)?.len();
    Index::open(f, size)
}

fn item_path(r: &FileRunner, job: &Job) -> PathBuf {
    r.path(&job.inputs[0].1[0])
}

pub(crate) fn out_path(r: &FileRunner, job: &Job, ext: &str) -> PathBuf {
    match &job.item_root {
        Some(k) => r.work.join(format!("{}-{k}.{ext}", job.instance.id)),
        None => r.work.join(format!("{}.{ext}", job.instance.id)),
    }
}

/// `index.mzidx`: the index of one file (mzML or any single file a reader takes).
pub(crate) fn index_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let source = item_path(r, job);
    let budget = job.resolved.params.iter().find(|p| p.0 == "budget");
    let budget = budget
        .and_then(|p| p.1.f64())
        .map_or(DEFAULT_BUDGET, |b| b as usize);
    let out = out_path(r, job, "mzidx");
    let label = job.inputs[0].1[0].label.clone().unwrap_or_default();
    let gate = r.gate(job);
    let mut pace = gate.pace();
    let built =
        crate::flow_pace::build_index_with(&source, &label, &out, budget, &mut pace, &gate.stop);
    if let Err(e) = built {
        let _ = std::fs::remove_file(&out);
        return Err(e);
    }
    let name = r.source_name(&job.inputs[0].1[0]);
    let a = r.keep(out, DataType::Mzidx)?;
    r.trace(
        &a,
        Lineage {
            name,
            instance: job.instance.id.clone(),
            item: job.item,
            ..Lineage::default()
        },
    );
    Ok(Made::new("index", a))
}

/// The recipe a lineage means: the one of the panel on its first index, then the parameters of each feature step.
pub(crate) fn rebuild(lin: &Lineage) -> Result<Recipe> {
    let first = lin
        .first
        .as_ref()
        .ok_or_else(|| Error::new("err.flow.lineage"))?;
    run::recipe_of_layers(&mut open(first)?, &lin.layers)
}

/// `features.detect`: the features of one indexed file (`.mzfeatf`), named after the file the index was made from.
pub(crate) fn detect_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let input = &job.inputs[0].1[0];
    let path = r.path(input);
    let first = r.first_of_series(input).unwrap_or_else(|| path.clone());
    // Each file takes its own base recipe, as the page does (`detect_file`): on the unit axis the mass defect and the floor come from the file.
    // one open of the index serves the recipe and the detection
    let gate = r.gate(job);
    let mut index = open_gated(&path, &gate)?;
    let (recipe, estimates) = recipe(job, run::recipe_for_estimated("", &mut index)?)?;
    let name = r.source_name(input);
    let bytes = run::detect(&mut index, &name, &recipe)?;
    let out = out_path(r, job, "mzfeatf");
    // The weak candidates are made here, with the other files of the level at the same time, so the alignment and the gap filling do not detect again.
    let weak = run::detect_weak(&mut index, &recipe)?;
    std::fs::write(weak_path(&out), weak).map_err(io)?;
    std::fs::write(&out, bytes).map_err(io)?;
    let a = r.keep(out, DataType::Peaks)?;
    let layers = vec![(job.step.id.to_string(), written(job))];
    r.trace(
        &a,
        Lineage {
            first: Some(first),
            layers,
            index: vec![path],
            ..Lineage::default()
        },
    );
    Ok(Made {
        estimates,
        preset: recipe.detect.axis.is_da().then_some("lr"),
        ..Made::new("peaks", a)
    })
}

/// The index at `path` as a detection reads it: it reports its progress and stops once the flag of `gate` is up.
fn open_gated(path: &PathBuf, gate: &Gate) -> Result<Index<Gated<File>>> {
    let f = File::open(path).map_err(io)?;
    let size = f.metadata().map_err(io)?.len();
    Index::open(Gated::new(f, gate, size), size)
}

/// The `.mzfeatf` of a series on disk, one file at a time (the slabs of the alignment read them again).
pub(crate) struct Files(pub(crate) Vec<PathBuf>);

impl Stored for Files {
    fn count(&self) -> usize {
        self.0.len()
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        std::fs::read(&self.0[k]).map_err(io)
    }
    fn weak(&mut self, k: usize) -> Result<Option<Vec<u8>>> {
        Ok(std::fs::read(weak_path(&self.0[k])).ok())
    }
}

/// The file beside a `.mzfeatf` that holds the weak candidates of its file (`mzlab_feat::weakfile`).
fn weak_path(peaks: &Path) -> PathBuf {
    let mut p = peaks.as_os_str().to_owned();
    p.push(".weak");
    PathBuf::from(p)
}

/// Aligns the peaks of `lin` with `recipe` under the memory budget of the runner and writes the table of `job`; the table hands on `lin` with the
/// parameters of `job`.
fn table(r: &FileRunner, job: &Job, recipe: &Recipe, mut lin: Lineage) -> Result<Made> {
    let budget = MemBudget::mb(r.budget_mb);
    let mut bytes = Vec::new();
    let mut src = Files(lin.peaks.clone());
    let gate = r.gate(job);
    let mut pace = gate.pace();
    run::align_to_par_progress(
        &mut src,
        &budget,
        recipe,
        &|k| open(&lin.index[k]),
        &mut bytes,
        &mut pace,
    )?;
    let out = out_path(r, job, "mzfeat");
    std::fs::write(&out, bytes).map_err(io)?;
    let a = r.keep(out, DataType::Table)?;
    lin.layers.push((job.step.id.to_string(), written(job)));
    r.trace(&a, lin);
    Ok(Made::new("table", a))
}

/// The recipe, the lineage and the estimates a step of the alignment runs with: `features.align` reads the peaks it is given, `features.gapfill` the ones
/// that made its table.
fn align_setup(r: &FileRunner, job: &Job) -> Result<(Recipe, Lineage, Vec<Estimate>)> {
    let mut lin = Lineage::default();
    let fill = job.step.id == "features.gapfill";
    if fill {
        let input = job
            .inputs
            .iter()
            .find(|(p, _)| p == "table")
            .and_then(|(_, v)| v.first());
        lin = input
            .and_then(|a| r.lineage(a))
            .filter(|l| l.first.is_some() && !l.peaks.is_empty())
            .ok_or_else(|| Error::new("err.flow.lineage").with("step", job.step.id))?;
    } else {
        for p in job.inputs.iter().flat_map(|(_, v)| v) {
            let l = r
                .lineage(p)
                .ok_or_else(|| Error::new("err.flow.lineage").with("step", job.step.id))?;
            if lin.first.is_none() {
                (lin.first, lin.layers) = (l.first, l.layers);
            }
            lin.index.extend(l.index);
            lin.peaks.push(r.path(p));
        }
    }
    // one table holds one polarity and one resolution, as in the browser (`align_series_to`)
    let indexes = lin.index.iter().map(open).collect::<Result<Vec<_>>>()?;
    mzlab_feat::nature::one_kind_of(&indexes)?;
    let base = rebuild(&lin).map_err(|e| e.with("step", job.step.id))?;
    let (mut recipe, estimates) = recipe(job, base)?;
    recipe.fill_gaps = fill;
    Ok((recipe, lin, estimates))
}

/// `features.align` (the table of the series without gap filling) and `features.gapfill` (the table of `features.align` with its gaps filled: the stored
/// table is not filled after the fact, alignment and filling run again on the peaks of that table, which gives the bytes of the page's single call).
pub(crate) fn align_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let (recipe, lin, estimates) = align_setup(r, job)?;
    let peaks = lin.peaks.clone();
    let made = table(r, job, &recipe, lin)?;
    Ok(Made {
        estimates,
        mem: mem_of(r, &recipe, peaks),
        ..made
    })
}

/// The memory the alignment of `job` asks for under the budget of the runner and the way it takes (`slabs::stima`); none for another step or when the
/// inputs cannot be read yet (the step itself reports why); `known` is the figure the step worked out while it ran, kept as it is.
pub(crate) fn align_mem(r: &FileRunner, job: &Job, known: Option<Mem>) -> Option<Mem> {
    if known.is_some() {
        return known;
    }
    if !matches!(job.step.id, "features.align" | "features.gapfill") {
        return None;
    }
    let (recipe, lin, _) = align_setup(r, job).ok()?;
    mem_of(r, &recipe, lin.peaks)
}

/// The memory the alignment of the `.mzfeatf` files `peaks` asks for under the budget of the runner.
fn mem_of(r: &FileRunner, recipe: &Recipe, peaks: Vec<PathBuf>) -> Option<Mem> {
    let rows = slabs::rows_of(&mut Files(peaks)).ok()?;
    let way = slabs::stima(&rows, recipe, (r.budget_mb * 1048576.0) as usize);
    Some(Mem {
        mb: way.bytes as f64 / 1048576.0,
        via: way.via.name(),
    })
}

/// The file `convert.mzml` writes in the work folder for the input whose file-name root is `root` (`Job::item_root`) of the instance `instance`.
pub(crate) fn convert_file(instance: &str, root: &str) -> String {
    format!("{instance}-{root}.mzML")
}

/// A parameter of the job as a number, `default` when the plan does not carry it.
fn number(job: &Job, name: &str, default: f64) -> f64 {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64()).unwrap_or(default)
}

/// `convert.mzml`: one sample of a dataset of the registry (a file, a `.wiff` with its `.wiff.scan`, a `.D`/`.d`/`.raw` folder) written as indexed mzML;
/// the input `NAME@ID` names the sample `ID`, else the first one is written.
pub(crate) fn convert_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let input = &job.inputs[0].1[0];
    let opz = Opzioni {
        livello: number(job, "zlib_level", f64::from(LIVELLO_ZLIB)) as u32,
        numpress: job
            .resolved
            .params
            .iter()
            .any(|(k, v)| k == "numpress" && v.bool() == Some(true)),
        dad: job
            .resolved
            .params
            .iter()
            .any(|(k, v)| k == "dad" && v.bool() == Some(true)),
        ..Opzioni::default()
    };
    let out = r.work.join(convert_file(
        &job.instance.id,
        job.item_root.as_deref().unwrap_or("0"),
    ));
    let peaks = match job
        .resolved
        .params
        .iter()
        .find(|p| p.0 == "peaks")
        .and_then(|p| p.1.str())
    {
        Some("profile") => Picchi::Profilo,
        Some("mzlab") => Picchi::Mzlab,
        _ => Picchi::File,
    };
    let gate = r.gate(job);
    let (path, label) = (r.path(input), input.label.as_deref());
    let mut pace = gate.pace();
    let (profile, bytes, sha256) =
        write_mzml_with(&path, label, &out, (peaks, opz), &mut pace, &gate.stop)?;
    crate::flow_set::remember_sha(&out, bytes, &sha256);
    let mut made = Made::new("mzml", r.keep_known(out, DataType::Mzml, bytes, sha256));
    if profile {
        made.warnings.push("warn.flow.profile".into());
    }
    Ok(made)
}

/// Writes the sample `label` (`ID` of a `NAME@ID` input; the first when empty) of the dataset at `path` to `out` as indexed mzML; true when the file keeps
/// its profile points. A failed write leaves no file.
pub(crate) fn write_mzml(
    path: &Path,
    label: Option<&str>,
    out: &Path,
    peaks: Picchi,
    opz: Opzioni,
) -> Result<bool> {
    write_mzml_with(path, label, out, (peaks, opz), &mut |_, _| {}, &None).map(|w| w.0)
}

/// `write_mzml` telling `step(done, total)` how many scans it has written, at most once per 1 %; the file is the same with any `step`. With the flag
/// `stop` up the write ends at the next piece with `err.flow.cancelled`. Gives whether the file keeps its profile points, and the size and SHA-256 of the file
/// written, hashed while it was written.
pub(crate) fn write_mzml_with(
    path: &Path,
    label: Option<&str>,
    out: &Path,
    (peaks, opz): (Picchi, Opzioni),
    step: &mut dyn FnMut(usize, usize),
    stop: &Stop,
) -> Result<(bool, u64, String)> {
    let mut pace = mzlab_core::pace::Pace::new(step);
    let mut reader = mzlab_lettori::apri(crate::flow_set::file_set(path)?, &Limiti::desktop())?;
    let ids: Vec<u32> = reader.campioni().iter().map(|s| s.id).collect();
    let sample = crate::flow_set::sample_index(label.unwrap_or(""), &ids)?;
    let lacking = reader.mancanti(sample);
    if lacking > 0 {
        let have = reader.indice(sample)?.len();
        eprintln!(
            "warn.file.truncated: {}: {have} of {} spectra read, the file ends before the rest",
            path.display(),
            have + lacking
        );
    }
    // The file keeps its profile points: the index of the next step picks the peaks, and the run says so.
    let profile = peaks == Picchi::File
        && !reader.indice(sample)?.is_empty()
        && reader.scansione(sample, 0)?.profile;
    let conv = Conversione {
        peaks,
        opz,
        steps: Some(&[]),
    };
    let written = File::create(out).map_err(io).and_then(|f| {
        let sink = BufWriter::new(HashWrite::new(Stopping::new(f, stop.clone())));
        let mut w = conv.scrivi(&mut *reader, sample, sink, &mut |d, t| pace.at(d, t))?;
        w.flush().map_err(io)?;
        let (_, bytes, sha256) = w.into_inner().map_err(|e| io(e.into_error()))?.finish();
        Ok((bytes, sha256))
    });
    match written {
        Ok((bytes, sha256)) => Ok((profile, bytes, sha256)),
        Err(e) => {
            let _ = std::fs::remove_file(out);
            Err(e)
        }
    }
}

/// `features.table`: the chain of `mzlab-cli features` on the files of the series, as CSV.
pub(crate) fn table_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let files: Vec<PathBuf> = job
        .inputs
        .iter()
        .flat_map(|(_, v)| v)
        .map(|a| r.path(a))
        .collect();
    let tmp = r.work.join(format!("{}.tmp", job.instance.id));
    std::fs::create_dir_all(&tmp).map_err(io)?;
    let csv = crate::features(&files, &job.resolved.text(), &tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, csv?).map_err(io)?;
    r.keep(out, DataType::Report)
}
