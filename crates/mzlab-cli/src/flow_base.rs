//! The feature steps run on files: `convert.mzml`, `features.table` and the base chain of the Flow canvas (`index.mzidx`, `features.detect`,
//! `features.align`, `features.gapfill`). The recipe of the engine is
//! the one the Pipeline panel takes with no setting (`run::recipe_for`: the profile of the first file of the series) with the values the plan writes on
//! top (`steps::recipe_map`); a value the plan leaves out that differs from the canonical one is reported as an estimate. Called by `flow.rs`.

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::idxread::Index;
use mzlab_core::json::Value;
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::mzidx::DEFAULT_BUDGET;
use mzlab_core::step::DataType;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::steps::recipe_map;
use mzlab_feat_wasm::run;

use crate::flow::{io, FileRunner, Lineage, Made};

/// A value the plan leaves out: name, value, proof.
type Estimate = (String, Value, Value);

fn open(path: &PathBuf) -> Result<Index<File>> {
    let f = File::open(path).map_err(io)?;
    let size = f.metadata().map_err(io)?.len();
    Index::open(f, size)
}

fn item_path(r: &FileRunner, job: &Job) -> PathBuf {
    r.path(&job.inputs[0].1[0])
}

fn out_path(r: &FileRunner, job: &Job, ext: &str) -> PathBuf {
    match job.item {
        Some(k) => r.work.join(format!("{}-{k}.{ext}", job.instance.id)),
        None => r.work.join(format!("{}.{ext}", job.instance.id)),
    }
}

/// What the plan writes for `job`: the values of its preset, then its own parameters (descriptor names).
fn written(job: &Job) -> Vec<(String, Value)> {
    let mut v: Vec<(String, Value)> = Vec::new();
    let preset = job.instance.preset.as_deref();
    let presets = job.step.presets.iter().filter(|p| Some(p.id) == preset);
    for (k, x) in presets.flat_map(|p| p.params.iter()) {
        v.push((k.to_string(), x.clone()));
    }
    for (k, x) in &job.instance.params {
        v.retain(|(n, _)| n != k);
        v.push((k.clone(), x.clone()));
    }
    v
}

/// The recipe of `job`: `base` with what the plan writes, and the values the plan leaves out that differ from the canonical ones (the estimates).
fn recipe(job: &Job, mut base: Recipe) -> Result<(Recipe, Vec<Estimate>)> {
    let w = written(job);
    recipe_map::apply(&mut base, job.step.id, &w)?;
    let resolved = &job.resolved.params;
    let proof = Value::Str("profile of the first file of the series".into());
    let estimates = recipe_map::values(&base, job.step.id)
        .into_iter()
        .filter(|(k, v)| {
            !w.iter().any(|x| x.0 == *k)
                && !job.resolved.noresult.contains(k)
                && resolved.iter().find(|x| x.0 == *k).map(|x| &x.1) != Some(v)
        })
        .map(|(k, v)| (k, v, proof.clone()))
        .collect();
    Ok((base, estimates))
}

/// `index.mzidx`: the index of one file (mzML or any single file a reader takes).
pub(crate) fn index_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let source = item_path(r, job);
    let budget = job.resolved.params.iter().find(|p| p.0 == "budget");
    let budget = budget
        .and_then(|p| p.1.f64())
        .map_or(DEFAULT_BUDGET, |b| b as usize);
    let out = out_path(r, job, "mzidx");
    crate::build_index(&source, &out, budget)?;
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
fn rebuild(lin: &Lineage) -> Result<Recipe> {
    let first = lin
        .first
        .as_ref()
        .ok_or_else(|| Error::new("err.flow.lineage"))?;
    let mut r = run::recipe_for("", &mut open(first)?)?;
    for (step, params) in &lin.layers {
        recipe_map::apply(&mut r, step, params)?;
    }
    Ok(r)
}

/// `features.detect`: the features of one indexed file (`.mzfeatf`), named after the file the index was made from.
pub(crate) fn detect_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let input = &job.inputs[0].1[0];
    let path = r.path(input);
    let first = r.first_of_series(input).unwrap_or_else(|| path.clone());
    let (recipe, estimates) = recipe(job, run::recipe_for("", &mut open(&first)?)?)?;
    let name = r.source_name(input);
    let bytes = run::detect(&mut open(&path)?, &name, &recipe)?;
    let out = out_path(r, job, "mzfeatf");
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
        ..Made::new("peaks", a)
    })
}

/// Aligns the peaks of `lin` with `recipe` and writes the table of `job`; the table hands on `lin` with the parameters of `job`.
fn table(r: &FileRunner, job: &Job, recipe: Recipe, mut lin: Lineage) -> Result<Made> {
    let files: Vec<Vec<u8>> = lin
        .peaks
        .iter()
        .map(|p| std::fs::read(p).map_err(io))
        .collect::<Result<_>>()?;
    let (tables, origin) = run::read_files(&files)?;
    let bytes = run::align(&tables, &origin, &recipe, |k| open(&lin.index[k]))?;
    let out = out_path(r, job, "mzfeat");
    std::fs::write(&out, bytes).map_err(io)?;
    let a = r.keep(out, DataType::Table)?;
    lin.layers.push((job.step.id.to_string(), written(job)));
    r.trace(&a, lin);
    Ok(Made::new("table", a))
}

/// `features.align`: the table of the series without gap filling (`features.gapfill` fills it).
pub(crate) fn align_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let peaks: Vec<_> = job.inputs.iter().flat_map(|(_, v)| v).collect();
    let mut lin = Lineage::default();
    for p in &peaks {
        let l = r
            .lineage(p)
            .ok_or_else(|| Error::new("err.flow.lineage").with("step", job.step.id))?;
        if lin.first.is_none() {
            (lin.first, lin.layers) = (l.first, l.layers);
        }
        lin.index.extend(l.index);
        lin.peaks.push(r.path(p));
    }
    let (mut recipe, estimates) =
        recipe(job, rebuild(&lin).map_err(|e| e.with("step", job.step.id))?)?;
    recipe.fill_gaps = false;
    Ok(Made {
        estimates,
        ..table(r, job, recipe, lin)?
    })
}

/// `features.gapfill`: the table of `features.align` with its gaps filled. The stored table is not filled after the fact: alignment and filling run
/// again on the peaks of that table (same run), which gives the bytes of the page's single call.
pub(crate) fn gapfill_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let input = job
        .inputs
        .iter()
        .find(|(p, _)| p == "table")
        .and_then(|(_, v)| v.first());
    let lin = input
        .and_then(|a| r.lineage(a))
        .filter(|l| l.first.is_some() && !l.peaks.is_empty());
    let lin = lin.ok_or_else(|| Error::new("err.flow.lineage").with("step", job.step.id))?;
    let (mut recipe, estimates) = recipe(job, rebuild(&lin)?)?;
    recipe.fill_gaps = true;
    Ok(Made {
        estimates,
        ..table(r, job, recipe, lin)?
    })
}

/// `convert.mzml`: one file of a reader of the registry written as indexed mzML.
pub(crate) fn convert_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let input = r.path(&job.inputs[0].1[0]);
    let name = input
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let dim = std::fs::metadata(&input).map_err(io)?.len();
    let fs = FileSet {
        membri: vec![Membro {
            nome: name,
            dim,
            dati: Box::new(File::open(&input).map_err(io)?),
            sha1: None,
        }],
    };
    let mut reader = mzlab_lettori::apri(fs, &Limiti::desktop())?;
    let out = r.work.join(format!(
        "{}-{}.mzML",
        job.instance.id,
        job.item.unwrap_or(0)
    ));
    let w = BufWriter::new(File::create(&out).map_err(io)?);
    mzlab_core::lettori_mzml::scrivi_indicizzato(
        &mut *reader,
        0,
        w,
        &mut |_, _| {},
        mzlab_core::mzmlwrite::Opzioni::default(),
        &[],
    )?;
    r.keep(out, DataType::Mzml)
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
