//! A workflow file run on real files: the `features.table@1` step (the whole chain of `features` as one step), the registry that holds it, and the
//! `FileRunner` that gives `mzlab_core::flow::execute` its files and its clock. Used by `examples/flow.rs` and by the tests; native and WASI.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{
    check, execute, parse, prov_json, Agent, Artifact, Job, Outcome, Plan, Platform, Run, Runner,
};
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow_match::Named;

fn io(e: std::io::Error) -> Error {
    Error::new("err.flow.io").with("detail", e)
}

/// `features.table@1`: the feature chain of `features` (index, detect, align, CSV "full") as one step; a parameter left out keeps the value the engine
/// takes from the profile of the first file.
pub fn table_step() -> Step {
    let number = |name, title| Param::number(name, title).min(0.0);
    Step {
        ins: vec![Port::new("source", DataType::Raw, "Vendor files or mzML, one series").many().accepts(&[DataType::Mzml])],
        outs: vec![Port::new("report", DataType::Report, "Feature table as CSV")],
        params: vec![
            Param::choice_num("polarity", "Polarity", &[-1.0, 1.0]),
            Param::choice("mode", "Grouping", &["family", "ion"]),
            number("floor", "Noise floor").unit("counts"),
            number("min_height", "Minimum height").unit("counts"),
            Param::integer("min_points", "Minimum points").min(1.0),
            number("ppm", "m/z tolerance").unit("ppm"),
            number("rt_tol", "RT tolerance").unit("min"),
        ],
        // the module is the WASI build of this crate, not a `.wasm` of the site
        runs_in: RunsIn {
            wasm: Some("wasip1 (mzlab-cli)"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.file.index", "error.badRecipe", "error.memoryBudget"],
        ..Step::core(
            "features.table",
            "Feature table",
            "Indexes the files, detects the features of each, aligns them and writes the table as CSV: the chain of `mzlab-cli features` as one step.",
        )
    }
}

/// The steps a plan can use here: the registry of the flagship chain, `features.table`, the identification steps and `spectra.ms2`.
pub fn steps() -> Vec<Step> {
    let mut all = mzlab_feat::steps::registry();
    all.push(table_step());
    all.extend(mzlab_feat::steps::attributio());
    all.push(crate::flow_ms2::ms2_step());
    all.push(crate::flow_compare::compare_step());
    all
}

/// Hooks of the caller to measure the heap (the library has no allocator of its own): reset the peak, read it.
#[derive(Clone, Copy)]
pub struct Meter {
    pub reset: fn(),
    pub peak: fn() -> u64,
}

/// Runs the steps of this crate on files: inputs are read from `dir`, what steps make is written to `work`.
pub struct FileRunner {
    dir: PathBuf,
    work: PathBuf,
    meter: Option<Meter>,
    made: Mutex<HashMap<String, PathBuf>>,
}

impl FileRunner {
    pub fn new(dir: &Path, work: &Path, meter: Option<Meter>) -> FileRunner {
        FileRunner {
            dir: dir.to_path_buf(),
            work: work.to_path_buf(),
            meter,
            made: Mutex::new(HashMap::new()),
        }
    }

    fn path(&self, a: &Artifact) -> PathBuf {
        let made = self.made.lock().unwrap_or_else(|e| e.into_inner());
        made.get(&a.sha256)
            .cloned()
            .unwrap_or_else(|| self.dir.join(a.label.clone().unwrap_or_default()))
    }

    fn keep(&self, path: PathBuf, ty: DataType) -> Result<Artifact> {
        let bytes = std::fs::read(&path).map_err(io)?;
        let sha256 = sha256_hex(&bytes);
        let label = path.file_name().map(|n| n.to_string_lossy().into_owned());
        self.made
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(sha256.clone(), path);
        Ok(Artifact {
            sha256,
            bytes: bytes.len() as u64,
            ty,
            label,
        })
    }
}

fn convert_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
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

fn table_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
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

fn one_input(r: &FileRunner, job: &Job, port: &str) -> Result<PathBuf> {
    let a = job
        .inputs
        .iter()
        .find(|(p, _)| p == port)
        .and_then(|(_, v)| v.first());
    a.map(|a| r.path(a))
        .ok_or_else(|| Error::new("err.flow.port.missing").with("port", port))
}

/// A file read whole: its name and its bytes (the executors take names and bytes, never paths).
type Loaded = (String, Vec<u8>);

fn load(path: PathBuf) -> Result<Loaded> {
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    Ok((name, std::fs::read(&path).map_err(io)?))
}

fn read_one(r: &FileRunner, job: &Job, port: &str) -> Result<Loaded> {
    load(one_input(r, job, port)?)
}

fn read_all(r: &FileRunner, job: &Job, port: &str) -> Result<Vec<Loaded>> {
    all_inputs(r, job, port).into_iter().map(load).collect()
}

fn named(v: &[Loaded]) -> Vec<Named<'_>> {
    v.iter().map(|f| (f.0.as_str(), f.1.as_slice())).collect()
}

fn blobs(v: &[Loaded]) -> Vec<&[u8]> {
    v.iter().map(|f| f.1.as_slice()).collect()
}

fn write_made(r: &FileRunner, job: &Job, text: String, ty: DataType) -> Result<Artifact> {
    let out = r.work.join(format!("{}.json", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, ty)
}

fn match_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (query, library) = (read_one(r, job, "query")?, read_one(r, job, "library")?);
    let text = crate::flow_match::match_text(job, (&query.0, &query.1), (&library.0, &library.1))?;
    write_made(r, job, text, DataType::Candidates)
}

fn all_inputs(r: &FileRunner, job: &Job, port: &str) -> Vec<PathBuf> {
    let list = job
        .inputs
        .iter()
        .filter(|(p, _)| p == port)
        .flat_map(|(_, v)| v);
    list.map(|a| r.path(a)).collect()
}

fn formula_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let text = if job.step.id == "formula.ms1" {
        let (table, index) = (read_one(r, job, "features")?, read_all(r, job, "index")?);
        crate::flow_formula::ms1_text(job, &table.1, &named(&index))?
    } else {
        let (spectra, formulas) = (read_one(r, job, "spectra")?, read_all(r, job, "formulas")?);
        crate::flow_formula::ms2_text(job, (&spectra.0, &spectra.1), &blobs(&formulas))?
    };
    write_made(r, job, text, DataType::Formulas)
}

fn ms2_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let files = read_all(r, job, "source")?;
    let has_table = job.inputs.iter().any(|(p, _)| p == "features");
    let table = if has_table {
        Some(read_one(r, job, "features")?)
    } else {
        None
    };
    let text =
        crate::flow_ms2::ms2_text(job, &named(&files), table.as_ref().map(|t| t.1.as_slice()))?;
    let out = r.work.join(format!("{}.mgf", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Spectra)
}

fn compare_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let text_of = |name: &str| {
        let found = job.resolved.params.iter().find(|(k, _)| k == name);
        found.and_then(|(_, v)| v.str()).unwrap_or("").to_string()
    };
    let (step, labels) = (text_of("step"), text_of("labels"));
    let files = read_all(r, job, "variants")?;
    let labels: Vec<&str> = labels.split(',').collect();
    let text = crate::flow_compare::compare_text(&step, &labels, &blobs(&files))?;
    write_made(r, job, text, DataType::Comparison)
}

fn level_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (hits, formulas) = (
        read_all(r, job, "candidates")?,
        read_all(r, job, "formulas")?,
    );
    let text = crate::flow_level::level_text(&blobs(&hits), &blobs(&formulas))?;
    write_made(r, job, text, DataType::Annot)
}

fn inclusion_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let find = |name: &str| {
        job.resolved
            .params
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
    };
    let adducts = find("adducts")
        .and_then(|v| v.str())
        .unwrap_or("[M+H]+")
        .to_string();
    let half = find("rt_window").and_then(|v| v.f64()).unwrap_or(0.5);
    let products = read_one(r, job, "tp")?;
    let text = crate::flow_inclusion::inclusion_text(&products.1, &adducts, half)?;
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Report)
}

impl Runner for FileRunner {
    fn run(&self, job: &Job) -> Result<Outcome> {
        if let Some(m) = self.meter {
            (m.reset)();
        }
        let start = Instant::now();
        let (port, artifact) = match job.step.id {
            "convert.mzml" => ("mzml", convert_job(self, job)?),
            "features.table" => ("report", table_job(self, job)?),
            "spectra.match" => ("hits", match_job(self, job)?),
            "formula.ms1" => ("formulas", formula_job(self, job)?),
            "formula.ms2" => ("formulas", formula_job(self, job)?),
            "spectra.ms2" => ("spectra", ms2_job(self, job)?),
            "annot.level" => ("annot", level_job(self, job)?),
            "meta.compare" => ("comparison", compare_job(self, job)?),
            "export.inclusion" => ("report", inclusion_job(self, job)?),
            id => return Err(Error::new("err.flow.unknownStep").with("step", id)),
        };
        Ok(Outcome {
            outputs: vec![(port.to_string(), artifact)],
            ms: start.elapsed().as_secs_f64() * 1e3,
            heap_peak_bytes: self.meter.map(|m| (m.peak)()),
            ..Outcome::default()
        })
    }
}

/// The inputs of the plan that name no hash get it from the file; one that names a hash must match it.
fn fill_inputs(plan: &mut Plan, dir: &Path) -> Result<()> {
    for f in &mut plan.inputs {
        let bytes = std::fs::read(dir.join(&f.name)).map_err(io)?;
        let sha = sha256_hex(&bytes);
        if !f.sha256.is_empty() && f.sha256 != sha {
            return Err(Error::new("err.flow.input.sha").with("input", &f.name));
        }
        f.sha256 = sha;
        f.bytes = Some(bytes.len() as u64);
    }
    Ok(())
}

/// Reads, checks and runs a workflow file; returns the run and its `prov/1` text. Files of the plan are read from `dir`, outputs written to `work`.
pub fn run_plan(
    text: &[u8],
    dir: &Path,
    work: &Path,
    workers: usize,
    meter: Option<Meter>,
) -> Result<(Run, String)> {
    let mut plan = parse(text)?;
    fill_inputs(&mut plan, dir)?;
    let reg = steps();
    // the comparisons are checked as written, then expanded and the whole plan is checked again
    let mut problems = crate::flow_compare::check(&plan, &reg, Platform::current());
    let plan = if problems.is_empty() {
        let plan = crate::flow_compare::expand(plan, &reg)?;
        problems = check(&plan, &reg, Platform::current());
        plan
    } else {
        plan
    };
    if !problems.is_empty() {
        let list: Vec<String> = problems
            .iter()
            .map(|p| {
                format!(
                    "{}{}{}:{}",
                    p.instance,
                    if p.port.is_empty() { "" } else { "." },
                    p.port,
                    p.error_key
                )
            })
            .collect();
        return Err(Error::new("err.flow.check").with("problems", list.join(" ")));
    }
    let run = execute(&plan, &reg, &FileRunner::new(dir, work, meter), workers)?;
    let agent = Agent {
        name: "mzLab".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        commit: None,
        platform: Platform::current().name(),
    };
    let prov = prov_json(&agent, Some(&sha256_hex(text)), &run);
    Ok((run, prov))
}
