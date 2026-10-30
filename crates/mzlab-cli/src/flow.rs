//! A workflow file run on real files: the `features.table@1` step (the whole chain of `features` as one step), the registry that holds it, the one table
//! of the steps that run natively (`EXECUTORS`) and the `FileRunner` that gives `mzlab_core::flow::execute` its files and its clock. Used by
//! `mzlab-cli flow run`, the route `flow/run` of the desktop app, `examples/flow.rs` and the tests; native and WASI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use self::screen_suspects::{regulatory_job, screen_job};
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job, Mem, Outcome, Runner};
use mzlab_core::json::Value;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow_base::table_job;
use crate::flow_cache::Cache;
use crate::flow_digl::uv::uv_job;
use crate::flow_drift::drift_job;
use crate::flow_estimate::estimate_job;
use crate::flow_insilico::insilico_job;
use crate::flow_level::level_job;
pub(crate) use crate::flow_made::Made;
use crate::flow_msngraph::job as msn_graph_job;
pub use crate::flow_out::{probe_plan, run_plan_cached, run_plan_notify};
use crate::flow_pace::Watch;
use crate::flow_reactivity::reactivity_job;
use crate::flow_read::{blobs, load, named, Loaded};
pub use crate::flow_run::{fill_inputs, prov_text, run_plan};
use crate::flow_stats::stats_job;

#[path = "flow_live.rs"]
pub mod live;
#[path = "flow_plausible.rs"]
mod plausible;
#[path = "flow_rtpredict.rs"]
mod rtpredict;
#[path = "flow_screen_suspects.rs"]
mod screen_suspects;

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

// The two steps of `docs/agenti/flow-web.md` (qc.run_order, report.methods) live in files of their own, declared here so that lib.rs stays within its limit.
#[path = "flow_methods.rs"]
mod flow_methods;
#[cfg(test)]
#[path = "flow_qcreport_tests.rs"]
mod flow_qcreport_tests;
#[path = "flow_runorder.rs"]
mod flow_runorder;

/// The steps a plan can use here, every family of descriptors: the registry of the flagship chain, `features.table`, Attributio, the whole Tabui
/// family, `spectra.ms2` and `meta.compare`. Whether a step also runs here is `executable`.
pub fn steps() -> Vec<Step> {
    let mut all = mzlab_feat::steps::registry();
    all.push(table_step());
    all.extend(mzlab_feat::steps::attributio());
    all.extend(mzlab_chem::step::tabui());
    all.push(crate::flow_centroid::centroid_step());
    all.extend(crate::flow_digl::cli_steps());
    all.extend([
        crate::flow_quant::quant_step(),
        crate::flow_massacc::massacc_step(),
    ]);
    all.extend([mzlab_feat::steps::run_order(), mzlab_feat::steps::methods()]);
    all
}

/// The runners of `flow run` and of `run_plan_cached`, which keep a cache.
impl FileRunner {
    /// The runner of `flow run`: inputs found by name in `inputs`, one progress line per job for the instances in `order`, the cache of `work`
    /// unless `fresh`.
    pub(crate) fn located(
        work: &Path,
        inputs: HashMap<String, PathBuf>,
        order: Vec<String>,
        meter: Option<Meter>,
        fresh: bool,
    ) -> FileRunner {
        FileRunner {
            inputs,
            progress: Some(order),
            cache: Some(crate::flow_cache::Cache::open(work, fresh)),
            ..FileRunner::new(work, work, meter)
        }
    }

    /// The runner of `run_plan_cached`: `new` with the cache of `work` (`Some(fresh)`).
    pub(crate) fn cached(
        dir: &Path,
        work: &Path,
        meter: Option<Meter>,
        fresh: Option<bool>,
    ) -> FileRunner {
        FileRunner {
            cache: fresh.map(|f| Cache::open(work, f)),
            ..FileRunner::new(dir, work, meter)
        }
    }

    /// The runner that calls `done` after each job.
    pub(crate) fn with_done(mut self, done: Option<DoneFn>) -> FileRunner {
        self.done = done;
        self
    }

    /// Writes the index of the cache, when the runner keeps one.
    pub(crate) fn save_cache(&self) -> Result<()> {
        self.cache
            .as_ref()
            .map_or(Ok(()), crate::flow_cache::Cache::save)
    }
}

/// Hooks of the caller to measure the heap (the library has no allocator of its own): reset the peak, read it.
#[derive(Clone, Copy)]
pub struct Meter {
    pub reset: fn(),
    pub peak: fn() -> u64,
}

/// Told after each job of a run: instance id, item of a per-file step, milliseconds, taken from the cache, and the error text when it failed.
pub type DoneFn = Arc<dyn Fn(&str, Option<usize>, f64, bool, Option<String>) + Send + Sync>;

/// Runs the steps of this crate on files: inputs are found by name in `inputs`, else in `dir`; what steps make is written to `work`.
pub struct FileRunner {
    dir: PathBuf,
    pub(crate) work: PathBuf,
    inputs: HashMap<String, PathBuf>,
    meter: Option<Meter>,
    pub(crate) made: Mutex<HashMap<String, PathBuf>>,
    lineage: Mutex<HashMap<String, Lineage>>,
    /// Instance ids in the order of the run: one line per job on stderr when set.
    pub(crate) progress: Option<Vec<String>>,
    /// Called after each job (the desktop app shows the steps as they finish).
    done: Option<DoneFn>,
    /// The cache of `work` (`flow_cache.rs`); none: every job runs and nothing is kept.
    cache: Option<crate::flow_cache::Cache>,
    /// Only look in the cache: a job whose key is not there fails with `err.flow.probe.miss` instead of running (`flow_probe.rs`).
    pub(crate) probe: bool,
    /// Working memory in MB of the alignment (`--budget-mb`; else the automatic one of this computer). The bytes written do not depend on it.
    pub(crate) budget_mb: f64,
    /// A live run: the cancel flag and the progress of each job it runs (`flow_live.rs`).
    watch: Option<Arc<dyn Watch>>,
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
            done: None,
            cache: None,
            probe: false,
            budget_mb: mzlab_core::budget_auto::native_auto_mb(),
            watch: None,
        }
    }

    /// The runner with the working memory of the alignment set to `mb` (None: the automatic budget).
    pub(crate) fn with_budget(mut self, mb: Option<f64>) -> FileRunner {
        if let Some(mb) = mb {
            self.budget_mb = mb;
        }
        self
    }

    pub(crate) fn path(&self, a: &Artifact) -> PathBuf {
        let made = self.made.lock().unwrap_or_else(|e| e.into_inner());
        let label = a.label.clone().unwrap_or_default();
        made.get(&a.sha256)
            .or_else(|| self.inputs.get(&label))
            .cloned()
            .unwrap_or_else(|| self.dir.join(crate::flow_set::dataset_name(&label)))
    }
}

/// Where a file made by the run comes from: what the next feature step needs and the plan does not carry. The recipe is kept as the way to make it
/// again (the first index of the series and the parameters each feature step wrote on top), so that a step taken from the cache hands it on too.
#[derive(Clone, Default, Debug, PartialEq)]
pub(crate) struct Lineage {
    /// Name of the file the index was made from (the sample name of the detection).
    pub name: String,
    /// The instance and item that made an index.
    pub instance: String,
    pub item: Option<usize>,
    /// The index the profile of the recipe is read from.
    pub first: Option<PathBuf>,
    /// Step and parameters written by each feature step, in order.
    pub layers: Vec<(String, Vec<(String, Value)>)>,
    pub index: Vec<PathBuf>,
    pub peaks: Vec<PathBuf>,
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

pub(crate) fn one_input(r: &FileRunner, job: &Job, port: &str) -> Result<PathBuf> {
    let a = job
        .inputs
        .iter()
        .find(|(p, _)| p == port)
        .and_then(|(_, v)| v.first());
    a.map(|a| r.path(a))
        .ok_or_else(|| Error::new("err.flow.port.missing").with("port", port))
}

pub(crate) fn read_one(r: &FileRunner, job: &Job, port: &str) -> Result<Loaded> {
    load(one_input(r, job, port)?)
}

pub(crate) fn read_all(r: &FileRunner, job: &Job, port: &str) -> Result<Vec<Loaded>> {
    all_inputs(r, job, port).into_iter().map(load).collect()
}

pub(crate) fn write_made(
    r: &FileRunner,
    job: &Job,
    text: String,
    ty: DataType,
) -> Result<Artifact> {
    let out = r.work.join(format!("{}.json", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, ty)
}

fn match_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (query, library) = (read_one(r, job, "query")?, read_one(r, job, "library")?);
    let text = crate::flow_match::match_text(job, (&query.0, &query.1), (&library.0, &library.1))?;
    write_made(r, job, text, DataType::Candidates)
}

pub(crate) fn all_inputs(r: &FileRunner, job: &Job, port: &str) -> Vec<PathBuf> {
    let list = job
        .inputs
        .iter()
        .filter(|(p, _)| p == port)
        .flat_map(|(_, v)| v);
    list.map(|a| r.path(a)).collect()
}

fn formula_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (spectra, formulas) = (read_one(r, job, "spectra")?, read_all(r, job, "formulas")?);
    let text = crate::flow_formula::ms2_text(job, (&spectra.0, &spectra.1), &blobs(&formulas))?;
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

/// How a step runs natively: an executor that makes one file for `port`, or one that says itself what it made.
pub(crate) enum Exec {
    File(&'static str, fn(&FileRunner, &Job) -> Result<Artifact>),
    Made(fn(&FileRunner, &Job) -> Result<Made>),
}

/// The steps that run natively, the one list: `executable` and the runner read it.
const EXECUTORS: &[(&str, Exec)] = &[
    ("import.raw", Exec::Made(crate::flow_import::import_job)),
    ("convert.mzml", Exec::Made(crate::flow_base::convert_job)),
    ("spectra.centroid", Exec::Made(crate::flow_centroid::job)),
    ("index.mzidx", Exec::Made(crate::flow_base::index_job)),
    ("features.detect", Exec::Made(crate::flow_base::detect_job)),
    ("features.align", Exec::Made(crate::flow_base::align_job)),
    ("features.gapfill", Exec::Made(crate::flow_base::align_job)),
    ("features.table", Exec::File("report", table_job)),
    ("features.estimate", Exec::Made(estimate_job)),
    ("table.stats", Exec::File("report", stats_job)),
    ("table.kmd", Exec::File("report", crate::flow_kmd::kmd_job)),
    ("table.pca", Exec::File("report", crate::flow_pca::pca_job)),
    ("table.drift", Exec::File("table", drift_job)),
    ("spectra.match", Exec::File("hits", match_job)),
    ("spectra.insilico", Exec::File("candidates", insilico_job)),
    ("formula.ms1", Exec::Made(crate::flow_formula::ms1_job)),
    ("formula.ms2", Exec::File("formulas", formula_job)),
    ("spectra.ms2", Exec::File("spectra", ms2_job)),
    ("spectra.msn_graph", Exec::File("graph", msn_graph_job)),
    ("tp.dig_lr", Exec::File("dig", crate::flow_digl::dig_job)),
    ("chrom.uv_peaks", Exec::File("peaks", uv_job)),
    ("annot.level", Exec::File("annot", level_job)),
    ("screen.suspects", Exec::File("screen", screen_job)),
    ("annot.regulatory", Exec::File("annot", regulatory_job)),
    (
        "tp.network",
        Exec::File("network", crate::flow_network::network_job),
    ),
    ("tp.link", Exec::Made(crate::flow_link::link_job)),
    (
        "table.filter",
        Exec::File("table", crate::flow_table::filter_job),
    ),
    (
        "tp.generate",
        Exec::Made(crate::flow_generate::generate_job),
    ),
    (
        "tp.filter",
        Exec::File("table", crate::flow_refilter::refilter_job),
    ),
    (
        "tp.position",
        Exec::File("sites", crate::flow_position::position_job),
    ),
    (
        "structure.render",
        Exec::File("drawing", crate::flow_render::render_job),
    ),
    (
        "structure.reactivity",
        Exec::File("reactivity", reactivity_job),
    ),
    (
        "meta.compare",
        Exec::File("comparison", crate::flow_compare::compare_job),
    ),
    ("quant.calibrate", Exec::Made(crate::flow_quant::quant_job)),
    (
        "export.inclusion",
        Exec::File("report", crate::flow_inclusion::inclusion_job),
    ),
    (
        "export.table_csv",
        Exec::File("report", crate::flow_export::csv_job),
    ),
    (
        "export.mztab",
        Exec::File("report", crate::flow_export::mztab_job),
    ),
];

/// `EXECUTORS` and the steps that keep their own list (`flow_msnlib::EXECUTORS`, `flow_tablematch::EXECUTORS`, `flow_chimeric::EXECUTORS`, so that this file stays short).
fn all_executors() -> impl Iterator<Item = &'static (&'static str, Exec)> {
    EXECUTORS
        .iter()
        .chain(crate::flow_msnlib::EXECUTORS)
        .chain(crate::flow_refilter::review::EXECUTORS)
        .chain(crate::flow_msnfigura::EXECUTORS)
        .chain(crate::flow_tablematch::EXECUTORS)
        .chain(crate::flow_chimeric::EXECUTORS)
        .chain(crate::flow_lookup::EXECUTORS)
        .chain(rtpredict::EXECUTORS)
        .chain(plausible::EXECUTORS)
        .chain(crate::flow_massacc::EXECUTORS)
        .chain(flow_runorder::EXECUTORS)
        .chain(flow_methods::EXECUTORS)
}

/// Does the native runner execute the step `id`?
pub fn executable(id: &str) -> bool {
    all_executors().any(|(k, _)| *k == id)
}

fn dispatch(r: &FileRunner, job: &Job) -> Result<Made> {
    let exec = all_executors().find(|(k, _)| *k == job.step.id);
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
        let key = self.cache.as_ref().map(|_| Cache::key(job));
        let hit = self
            .cache
            .as_ref()
            .zip(key.as_deref())
            .and_then(|(c, k)| c.take(self, job, k));
        let cached = hit.is_some();
        let made = hit.map_or_else(
            || {
                if self.probe {
                    Err(Error::new("err.flow.probe.miss"))
                } else {
                    dispatch(self, job)
                }
            },
            Ok,
        );
        let made = made.map_err(|e| self.cancelled_or(job, e));
        if let (Ok(m), Some(c), Some(k), false) = (&made, &self.cache, key, cached) {
            c.put(k, m, self.lineage(&m.artifact));
        }
        let ms = start.elapsed().as_secs_f64() * 1e3;
        if let Some(order) = &self.progress {
            let line = crate::flow_run::progress_line(order, job, ms, cached, made.as_ref().err());
            eprintln!("{line}");
        }
        if let Some(done) = &self.done {
            done(
                &job.instance.id,
                job.item,
                ms,
                cached,
                made.as_ref().err().map(|e| e.to_string()),
            );
        }
        let mut made = made?;
        let mut outputs = vec![(made.port.to_string(), made.artifact)];
        outputs.extend(made.extra.take().map(|(p, a)| (p.to_string(), a)));
        Ok(Outcome {
            outputs,
            estimates: made.estimates,
            ms,
            heap_peak_bytes: self.meter.map(|m| (m.peak)()),
            mem: crate::flow_base::align_mem(self, job, made.mem.take()),
            warnings: made.warnings,
            preset: made.preset.map(str::to_string),
            file: made.file,
            cached,
        })
    }
}
