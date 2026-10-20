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

/// The steps a plan can use here: the registry of the flagship chain and `features.table`.
pub fn steps() -> Vec<Step> {
    let mut all = mzlab_feat::steps::registry();
    all.push(table_step());
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

impl Runner for FileRunner {
    fn run(&self, job: &Job) -> Result<Outcome> {
        if let Some(m) = self.meter {
            (m.reset)();
        }
        let start = Instant::now();
        let (port, artifact) = match job.step.id {
            "convert.mzml" => ("mzml", convert_job(self, job)?),
            "features.table" => ("report", table_job(self, job)?),
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
    let problems = check(&plan, &reg, Platform::current());
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
