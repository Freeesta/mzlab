//! `mzlab-cli features FILE... [--out PATH] [--preset auto|pos|neg | --params FILE.json]`: the feature engine of the app on the command line.
//! A thin wrapper: the index of each file is built as the desktop app builds it, the recipe comes from `run::recipe_for` (the one behind the panel
//! "Pipeline"), the steps are `run::detect` and `run::align`, and the CSV is the export "full" of the table (`core::table_file`), so the numbers are the ones of the page.

use std::fs::File;
use std::path::{Path, PathBuf};

use mzlab_core::budget::{sha256_hex, MemBudget};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::Index;
use mzlab_core::json::Out;
use mzlab_core::mzidx::DEFAULT_BUDGET;
use mzlab_feat_table_wasm::core::{samples_json, table_file};
use mzlab_feat_wasm::run;

mod canali_cmd;
pub mod convert;
mod convert_report;
pub mod flow;
mod flow_apply;
mod flow_base;
#[cfg(test)]
mod flow_bentazone_tests;
mod flow_cache;
mod flow_centroid;
#[cfg(test)]
mod flow_centroid_tests;
mod flow_chimeric;
#[cfg(test)]
mod flow_chimeric_tests;
mod flow_compare;
mod flow_digl;
mod flow_drift;
#[cfg(test)]
mod flow_drift_tests;
mod flow_estimate;
mod flow_explain;
mod flow_export;
#[cfg(test)]
mod flow_export_tests;
mod flow_formula;
mod flow_generate;
mod flow_hash;
#[cfg(test)]
mod flow_hr_tests;
mod flow_import;
#[cfg(test)]
mod flow_import_tests;
mod flow_inclusion;
mod flow_insilico;
#[cfg(test)]
mod flow_insilico_tests;
mod flow_json;
mod flow_kmd;
mod flow_level;
mod flow_link;
#[cfg(test)]
mod flow_link_tests;
mod flow_lookup;
#[cfg(test)]
mod flow_lookup_tests;
#[cfg(test)]
mod flow_lr_tests;
mod flow_match;
mod flow_match_evidence;
mod flow_match_filter;
#[cfg(test)]
mod flow_match_tests;
mod flow_match_tree;
mod flow_ms2;
mod flow_msnfigura;
mod flow_msngraph;
mod flow_msnlib;
#[cfg(test)]
mod flow_msnlib_tests;
#[cfg(test)]
mod flow_msnraw_tests;
mod flow_nature;
mod flow_network;
mod flow_out;
mod flow_pace;
#[cfg(test)]
mod flow_pace_tests;
mod flow_pca;
#[cfg(test)]
mod flow_pca_tests;
mod flow_position;
#[cfg(test)]
mod flow_position_tests;
mod flow_quant;
mod flow_reactivity;
#[cfg(test)]
mod flow_reactivity_tests;
mod flow_read;
mod flow_refilter;
#[cfg(test)]
mod flow_refilter_tests;
mod flow_render;
#[cfg(test)]
mod flow_render_tests;
pub mod flow_report;
mod flow_run;
mod flow_set;
mod flow_stats;
mod flow_table;
#[cfg(test)]
mod flow_table_tests;
mod flow_tablematch;
#[cfg(test)]
mod flow_tablematch_tests;
pub mod llm;
mod llm_ctx;
pub mod msn_figura;
pub mod msn_graph;
mod msn_graph_compare;
pub mod msn_html;
mod msn_lib;
mod msn_png;
mod msn_rules;
pub mod msn_uscite;
mod par;
mod steps_cmd;
mod tutor;
#[cfg(not(target_family = "wasm"))]
mod validate;

const HELP: &str = "mzlab-cli features: the feature table of one or more LC-MS files as CSV, from the engine of the mzLab app.

usage: mzlab-cli features FILE... [--out PATH] [--preset auto|pos|neg | --params FILE.json]
       mzlab-cli flow run PLAN.json [--in FILE|FOLDER...] --out FOLDER [--set INSTANCE.NAME=VALUE...]   a whole Flow plan (flow run --help)
       mzlab-cli flow check PLAN.json [--set INSTANCE.NAME=VALUE...] [--json]   is the plan valid, without the input files (flow check --help)
       mzlab-cli steps [--json] [--id ID[@V]]   the Flow steps of this program, native or page only (steps --help)
       mzlab-cli flow report RUNDIR [--out FILE]   the corsa/1 report of a run folder (prov.json and the files beside it)
       mzlab-cli llm tools [--out FILE]   the catalog strumenti/1 of the assistant tools (llm tools --help)
       mzlab-cli llm replay REG.jsonl --conv CONV.json [--turn N] [--strict]   the events a recording gives for a conversation, one per line
       mzlab-cli tutor eval CASES --blocchi F (--replay DIR | --solo-recupero)   score the Teoria tutor on the eval cases (tutor --help)
       mzlab-cli llm context RUNDIR [--budget N] [--hide-names]   the contesto/1 of a run folder; llm eval CASES_DIR --replay DIR scores the cases
       mzlab-cli msn-graph FILE.mzML [--formula ION] [...]   the fragmentation graph of an MSn infusion (msn-graph --help)
       mzlab-cli canali FILE [--out PATH]   the channels of a file as JSON (canali --help)
       mzlab-cli explain-missing TABLE.mzfeat EXPECTED.csv   where a table loses each expected product (explain-missing --help)
       mzlab-cli msn-lib add|build|export|search ...                the library of MSn trees (msn-lib --help)
       mzlab-cli msn-rules RECORD... | somma LINES...   the fragmentation rules judged on library spectra (msn-rules --help)
       mzlab-cli convert INPUT... --out FOLDER [...]   vendor files and folders to indexed mzML (convert --help)
       mzlab-cli validate FILE.mzML... [--json]   is the mzML valid: structure, PSI-MS vocabulary, index and SHA-1 (validate --help)

  FILE...            mzML, or any single file the app reads (.raw, mzXML, .cdf, ...); several files form one series, in the order given
  --out PATH         a .csv file, or a folder (then features.csv is written in it); default: standard output
  --preset NAME      auto (default: both polarities as the panel does), pos or neg
  --params FILE.json the numbers of the panel Pipeline: {\"polarity\", \"mode\", \"floor\", \"min_height\", \"min_points\", \"ppm\", \"rt_tol\", \"fill\"}
  --help             this text";

fn io(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

/// The `.mzidx` of `path` written to `idx` with `budget` bytes of working memory: an mzML through its scan table, any other file or dataset (a `.wiff` with
/// its `.wiff.scan`, a `.D`/`.d`/`.raw` folder) through the reader of the registry. `label` is the plan input (`NAME@ID` names the sample `ID`).
pub(crate) use flow_pace::build_index_of;

pub(crate) fn build_index(path: &Path, idx: &Path, budget: usize) -> Result<()> {
    build_index_of(path, "", idx, budget)
}

pub(crate) fn open(idx: &Path) -> Result<Index<File>> {
    let f = File::open(idx).map_err(io)?;
    let size = f.metadata().map_err(io)?.len();
    Index::open(f, size)
}

/// The CSV of the series `files`, indexes kept in `tmp` (the caller owns it: WASI has no temporary folder).
pub fn features(files: &[PathBuf], params: &str, tmp: &Path) -> Result<String> {
    let idxs: Vec<PathBuf> = (0..files.len())
        .map(|k| tmp.join(format!("{k}.mzidx")))
        .collect();
    let (recipe, detected, weak) = crate::par::index_and_detect(files, &idxs, params)?;
    let mut series = crate::par::Series(detected, weak);
    let origin = run::origin_of(&mut series)?;
    eprintln!("align {} file(s)", files.len());
    let mut table = Vec::new();
    let budget = MemBudget::mb(mzlab_core::budget_auto::native_auto_mb());
    run::align_to_par(
        &mut series,
        &budget,
        &recipe,
        &|k| open(&idxs[k]),
        &mut table,
    )?;
    let mut list = Out::default();
    list.raw("{\"files\":[");
    for (k, (name, sha)) in origin.inputs.iter().enumerate() {
        list.raw(if k > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(name)
            .raw(",\"sha256\":")
            .str(sha)
            .raw("}");
    }
    list.raw("]}");
    let samples = samples_json(&list.0)?;
    let polarity = recipe.detect.polarity.map_or(1, i32::from);
    table_file(
        &mut mzlab_feat::store::Slice(&table),
        table.len() as u64,
        &samples,
        "",
        &sha256_hex(&table),
        polarity,
        "full",
    )
}

struct Args {
    files: Vec<PathBuf>,
    out: Option<PathBuf>,
    params: String,
}

fn parse(mut a: impl Iterator<Item = String>) -> std::result::Result<Option<Args>, String> {
    let mut r = Args {
        files: Vec::new(),
        out: None,
        params: String::new(),
    };
    let mut set = false;
    while let Some(x) = a.next() {
        let mut value = |what: &str| a.next().ok_or(format!("{x} needs {what}"));
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--out" => r.out = Some(value("a path")?.into()),
            "--preset" | "--params" if set => {
                return Err("use --preset or --params, not both".into())
            }
            "--preset" => {
                r.params = match value("auto, pos or neg")?.as_str() {
                    "auto" => String::new(),
                    "pos" => "{\"polarity\":1}".into(),
                    "neg" => "{\"polarity\":-1}".into(),
                    p => return Err(format!("unknown preset {p}")),
                };
                set = true;
            }
            "--params" => {
                let p = value("a JSON file")?;
                r.params = std::fs::read_to_string(&p).map_err(|e| format!("{p}: {e}"))?;
                set = true;
            }
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            _ => r.files.push(x.into()),
        }
    }
    if r.files.is_empty() {
        return Err("no input file".into());
    }
    Ok(Some(r))
}

/// What a run of the command leaves: exit code, standard output, standard error.
pub struct Outcome {
    pub code: u8,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Outcome {
    fn ok(stdout: Vec<u8>) -> Self {
        Outcome {
            code: 0,
            stdout,
            stderr: String::new(),
        }
    }

    fn fail(msg: &str) -> Self {
        Outcome {
            code: 2,
            stdout: Vec::new(),
            stderr: format!("error: {msg}\n(mzlab-cli features --help)\n"),
        }
    }
}

/// The text to the standard output, or to the file of `--out`.
fn emit(t: String, out: &Option<PathBuf>) -> Outcome {
    match out {
        None => Outcome::ok(t.into_bytes()),
        Some(p) => std::fs::write(p, t).map_or_else(
            |e| Outcome::fail(&e.to_string()),
            |_| Outcome::ok(Vec::new()),
        ),
    }
}

/// `mzlab-cli msn-lib ...`: the library of MSn trees; the data on stdout, the notices on stderr.
fn msn_lib_cmd(args: Vec<String>) -> Outcome {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-lib{k}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(io)
        .and_then(|_| msn_lib::run(args, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    match r {
        Ok((data, note)) => Outcome {
            code: 0,
            stdout: data,
            stderr: note,
        },
        Err(e) => Outcome::fail(&e.to_string()),
    }
}

/// `mzlab-cli flow command PLAN.json`: the script that reruns the plan, on stdout.
fn flow_command(args: Vec<String>) -> Outcome {
    let [path] = args.as_slice() else {
        return Outcome::fail("usage: mzlab-cli flow command PLAN.json");
    };
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"));
    match text.and_then(|t| mzlab_core::flow::command_route(&t).map_err(|e| e.to_string())) {
        Ok(s) => Outcome::ok(s.into_bytes()),
        Err(e) => Outcome::fail(&e),
    }
}

/// `mzlab-cli features ...`: the feature table in CSV on the standard output or at `--out`.
fn features_cmd(args: impl Iterator<Item = String>) -> Outcome {
    let a = match parse(args) {
        Ok(Some(a)) => a,
        Ok(None) => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}", std::process::id()));
    let r = std::fs::create_dir_all(&tmp)
        .map_err(io)
        .and_then(|_| features(&a.files, &a.params, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    let csv = match r {
        Ok(c) => c,
        Err(e) => return Outcome::fail(&e.to_string()),
    };
    match &a.out {
        None => Outcome::ok(csv.into_bytes()),
        Some(p) => {
            let to = if p.is_dir() {
                p.join("features.csv")
            } else {
                p.clone()
            };
            std::fs::write(to, csv).map_or_else(
                |e| Outcome::fail(&e.to_string()),
                |_| Outcome::ok(Vec::new()),
            )
        }
    }
}

/// `mzlab-cli flow <run|check|report|command> ...`.
fn flow_cmd(mut args: impl Iterator<Item = String>) -> Outcome {
    match args.next().as_deref() {
        Some("report") => flow_report::run(args).map_or_else(|e| Outcome::fail(&e), Outcome::ok),
        Some("run") => flow_run::run(args.collect(), None),
        Some("command") => flow_command(args.collect()),
        Some("check") => flow_json::check(args.collect()),
        _ => Outcome::fail("usage: mzlab-cli flow run PLAN.json ... | mzlab-cli flow check PLAN.json | mzlab-cli flow report RUNDIR [--out FILE] | mzlab-cli flow command PLAN.json"),
    }
}

/// The subcommands that take their arguments as a plain list.
const PLAIN: [&str; 6] = [
    "llm",
    "steps",
    "tutor",
    "explain-missing",
    "msn-rules",
    "canali",
];

fn run_plain(command: &str, args: Vec<String>) -> Outcome {
    match command {
        "llm" => llm::run(args),
        "steps" => steps_cmd::run(args),
        "explain-missing" => flow_explain::run(args),
        "msn-rules" => msn_rules::run(args),
        "canali" => canali_cmd::run(args),
        _ => tutor::run(args),
    }
}

/// `mzlab-cli <args>`: `args` without the program name.
pub fn execute(args: &[String]) -> Outcome {
    let mut args = args.iter().cloned();
    match args.next().as_deref() {
        Some("features") => {}
        Some("flow") => return flow_cmd(args),
        Some(c) if PLAIN.contains(&c) => return run_plain(c, args.collect()),
        Some("msn-graph") => return msn_graph::cmd(args),
        Some("msn-lib") => return msn_lib_cmd(args.collect()),
        Some("convert") => return convert::run(args.collect()),
        #[cfg(not(target_family = "wasm"))]
        Some("validate") => return validate::run(args),
        Some("--help" | "-h") | None => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        Some(c) => return Outcome::fail(&format!("unknown command {c}")),
    }
    features_cmd(args)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod convert_fixtures;
#[cfg(test)]
mod convert_tests;
#[cfg(test)]
mod flow_budget_tests;
#[cfg(test)]
mod flow_cache_tests;
#[cfg(test)]
mod flow_command_tests;
#[cfg(test)]
mod flow_compare_tests;
#[cfg(test)]
mod flow_estimate_tests;
#[cfg(test)]
mod flow_generate_tests;
#[cfg(test)]
mod flow_ident_tests;
#[cfg(test)]
mod flow_json_tests;
#[cfg(test)]
mod flow_kmd_tests;
#[cfg(test)]
mod flow_level_tests;
#[cfg(test)]
mod flow_ms2_tests;
#[cfg(test)]
mod flow_msngraph_tests;
#[cfg(test)]
mod flow_network_tests;
#[cfg(test)]
mod flow_picchi_tests;
#[cfg(test)]
mod flow_report_tests;
#[cfg(test)]
mod flow_rule_tests;
#[cfg(test)]
mod flow_run_tests;
#[cfg(test)]
mod flow_stats_tests;
#[cfg(test)]
mod llm_agent_tests;
#[cfg(test)]
mod llm_eval_tests;
#[cfg(test)]
mod llm_replay_tests;
#[cfg(test)]
mod llm_tools_tests;
#[cfg(test)]
mod msn_graph_compare_tests;
#[cfg(test)]
mod msn_graph_strutture_tests;
#[cfg(test)]
mod msn_graph_tests;
#[cfg(test)]
mod msn_lib_tests;
#[cfg(test)]
mod tutor_tests;
#[cfg(all(test, not(target_family = "wasm")))]
mod validate_tests;
