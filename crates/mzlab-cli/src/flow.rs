//! A workflow file run on real files: the `features.table@1` step (the whole chain of `features` as one step), the registry that holds it, the one table
//! of the steps that run natively (`EXECUTORS`) and the `FileRunner` that gives `mzlab_core::flow::execute` its files and its clock. Used by
//! `mzlab-cli flow run`, the route `flow/run` of the desktop app, `examples/flow.rs` and the tests; native and WASI.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{parse, Artifact, Job, Outcome, Plan, Run, Runner};
use mzlab_core::json::Value;
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow_match::Named;

pub(crate) fn io(e: std::io::Error) -> Error {
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

/// The steps a plan can use here: the registry of the flagship chain, `features.table`, the identification steps, `tp.network` and `spectra.ms2`.
pub fn steps() -> Vec<Step> {
    let mut all = mzlab_feat::steps::registry();
    all.push(table_step());
    all.extend(mzlab_feat::steps::attributio());
    all.extend(
        mzlab_feat::steps::tabui()
            .into_iter()
            .filter(|s| s.id == "tp.network"),
    );
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

/// Runs the steps of this crate on files: inputs are found by name in `inputs`, else in `dir`; what steps make is written to `work`.
pub struct FileRunner {
    dir: PathBuf,
    pub(crate) work: PathBuf,
    inputs: HashMap<String, PathBuf>,
    meter: Option<Meter>,
    made: Mutex<HashMap<String, PathBuf>>,
    lineage: Mutex<HashMap<String, Lineage>>,
    /// Instance ids in the order of the run: one line per job on stderr when set.
    progress: Option<Vec<String>>,
}

impl FileRunner {
    pub fn new(dir: &Path, work: &Path, meter: Option<Meter>) -> FileRunner {
        FileRunner {
            dir: dir.to_path_buf(),
            work: work.to_path_buf(),
            inputs: HashMap::new(),
            meter,
            made: Mutex::new(HashMap::new()),
            lineage: Mutex::new(HashMap::new()),
            progress: None,
        }
    }

    pub(crate) fn path(&self, a: &Artifact) -> PathBuf {
        let made = self.made.lock().unwrap_or_else(|e| e.into_inner());
        let label = a.label.clone().unwrap_or_default();
        made.get(&a.sha256)
            .or_else(|| self.inputs.get(&label))
            .cloned()
            .unwrap_or_else(|| self.dir.join(label))
    }

    pub(crate) fn keep(&self, path: PathBuf, ty: DataType) -> Result<Artifact> {
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

/// Where a file made by the run comes from: what the next feature step needs and the plan does not carry (the recipe, the indexes, the peaks).
#[derive(Clone, Default)]
pub(crate) struct Lineage {
    /// Name of the file the index was made from (the sample name of the detection).
    pub name: String,
    /// The instance and item that made an index.
    pub instance: String,
    pub item: Option<usize>,
    pub recipe: Option<mzlab_feat::recipe::Recipe>,
    pub index: Vec<PathBuf>,
    pub peaks: Vec<PathBuf>,
}

/// What an executor made: the output port, its file and the values it measured (name, value, proof).
pub(crate) struct Made {
    pub port: &'static str,
    pub artifact: Artifact,
    pub estimates: Vec<(String, Value, Value)>,
}

impl Made {
    pub fn new(port: &'static str, artifact: Artifact) -> Made {
        Made {
            port,
            artifact,
            estimates: Vec::new(),
        }
    }
}

/// The lineage of the files the run makes (what the feature steps hand on).
impl FileRunner {
    pub(crate) fn trace(&self, a: &Artifact, l: Lineage) {
        let mut map = self.lineage.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(a.sha256.clone(), l);
    }

    pub(crate) fn lineage(&self, a: &Artifact) -> Option<Lineage> {
        let map = self.lineage.lock().unwrap_or_else(|e| e.into_inner());
        map.get(&a.sha256).cloned()
    }

    /// The name of the file an index was made from, else the file name of `a`.
    pub(crate) fn source_name(&self, a: &Artifact) -> String {
        let l = self.lineage(a).map(|l| l.name).filter(|n| !n.is_empty());
        l.unwrap_or_else(|| a.label.clone().unwrap_or_default())
    }

    /// The runner of `flow run`: inputs found by name in `inputs`, one progress line per job for the instances in `order`.
    pub(crate) fn located(
        work: &Path,
        inputs: HashMap<String, PathBuf>,
        order: Vec<String>,
        meter: Option<Meter>,
    ) -> FileRunner {
        FileRunner {
            inputs,
            progress: Some(order),
            ..FileRunner::new(work, work, meter)
        }
    }

    /// The first index (item 0) made by the instance that made the index `a`.
    pub(crate) fn first_of_series(&self, a: &Artifact) -> Option<PathBuf> {
        let me = self.lineage(a)?;
        let map = self.lineage.lock().unwrap_or_else(|e| e.into_inner());
        let first = map
            .iter()
            .find(|(_, l)| l.instance == me.instance && l.item == Some(0))?;
        let made = self.made.lock().unwrap_or_else(|e| e.into_inner());
        made.get(first.0).cloned()
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

fn network_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (tp, table) = (read_one(r, job, "tp")?, read_one(r, job, "table")?);
    let has_samples = job.inputs.iter().any(|(p, _)| p == "samples");
    let samples = if has_samples {
        Some(read_one(r, job, "samples")?)
    } else {
        None
    };
    let text = crate::flow_network::network_json(
        job,
        &tp.1,
        &table.1,
        samples.as_ref().map(|s| s.1.as_slice()),
    )?;
    write_made(r, job, text, DataType::Transform)
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

/// How a step runs natively: an executor that makes one file for `port`, or one that says itself what it made.
enum Exec {
    File(&'static str, fn(&FileRunner, &Job) -> Result<Artifact>),
    Made(fn(&FileRunner, &Job) -> Result<Made>),
}

/// The steps that run natively, the one list: `executable` and the runner read it.
const EXECUTORS: &[(&str, Exec)] = &[
    ("convert.mzml", Exec::File("mzml", convert_job)),
    ("index.mzidx", Exec::Made(crate::flow_base::index_job)),
    ("features.detect", Exec::Made(crate::flow_base::detect_job)),
    ("features.align", Exec::Made(crate::flow_base::align_job)),
    (
        "features.gapfill",
        Exec::Made(crate::flow_base::gapfill_job),
    ),
    ("features.table", Exec::File("report", table_job)),
    ("spectra.match", Exec::File("hits", match_job)),
    ("formula.ms1", Exec::File("formulas", formula_job)),
    ("formula.ms2", Exec::File("formulas", formula_job)),
    ("spectra.ms2", Exec::File("spectra", ms2_job)),
    ("annot.level", Exec::File("annot", level_job)),
    ("tp.network", Exec::File("network", network_job)),
    ("meta.compare", Exec::File("comparison", compare_job)),
    ("export.inclusion", Exec::File("report", inclusion_job)),
];

/// Does the native runner execute the step `id`?
pub fn executable(id: &str) -> bool {
    EXECUTORS.iter().any(|(k, _)| *k == id)
}

fn dispatch(r: &FileRunner, job: &Job) -> Result<Made> {
    let exec = EXECUTORS.iter().find(|(k, _)| *k == job.step.id);
    match exec.map(|e| &e.1) {
        Some(Exec::File(port, f)) => Ok(Made::new(port, f(r, job)?)),
        Some(Exec::Made(f)) => f(r, job),
        None => Err(Error::new("err.flow.unknownStep").with("step", job.step.id)),
    }
}

impl Runner for FileRunner {
    fn run(&self, job: &Job) -> Result<Outcome> {
        if let Some(m) = self.meter {
            (m.reset)();
        }
        let start = Instant::now();
        let made = dispatch(self, job);
        let ms = start.elapsed().as_secs_f64() * 1e3;
        if let Some(order) = &self.progress {
            eprintln!(
                "{}",
                crate::flow_run::progress_line(order, job, ms, made.as_ref().err())
            );
        }
        let made = made?;
        Ok(Outcome {
            outputs: vec![(made.port.to_string(), made.artifact)],
            estimates: made.estimates,
            ms,
            heap_peak_bytes: self.meter.map(|m| (m.peak)()),
            ..Outcome::default()
        })
    }
}

/// The inputs of the plan that name no hash get it from the file `locate` gives; one that names a hash must match it.
pub(crate) fn fill_inputs(plan: &mut Plan, locate: impl Fn(&str) -> PathBuf) -> Result<()> {
    for f in &mut plan.inputs {
        let bytes = std::fs::read(locate(&f.name)).map_err(|e| io(e).with("input", &f.name))?;
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
    fill_inputs(&mut plan, |n| dir.join(n))?;
    let plan = crate::flow_run::prepare(plan, &steps())?.map_err(|problems| {
        let list: Vec<String> = problems.iter().map(crate::flow_run::problem_text).collect();
        Error::new("err.flow.check").with("problems", list.join(" "))
    })?;
    crate::flow_run::run_checked(&plan, text, &FileRunner::new(dir, work, meter), workers)
}
