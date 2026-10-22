//! The native executors of `msn.library_export@1` and `msn.table_export@1`: the fragmentation graph `msngrafo/1` of an infusion (`spectra.msn_graph`) written
//! as a spectral library (MSP or MassBank, every MS level, with the formula and the neutral loss of each explained peak) or as a CSV table of its nodes,
//! its modifications (the neutral losses between ions) or its peaks. Both read the graph text, so the plan and `mzlab-cli msn-lib export` write the same bytes.

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::{parse, Value};
use mzlab_core::msngrafo;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow::{io, read_one, Exec, FileRunner};

/// The executors of the two steps, read by the runner after its own list.
pub(crate) const EXECUTORS: &[(&str, Exec)] = &[
    ("msn.library_export", Exec::File("library", library_job)),
    ("msn.table_export", Exec::File("table", table_job)),
];

fn graph_port() -> Port {
    Port::new(
        "graph",
        DataType::Report,
        "Fragmentation graph (msngrafo/1)",
    )
}

/// `msn.library_export@1`: the spectra of the tree as a library file that `spectra.match` and the library builders read.
pub fn library_step() -> Step {
    Step {
        ins: vec![graph_port()],
        outs: vec![Port::new("library", DataType::Library, "Spectra of the tree (MSP or MassBank)")],
        params: vec![
            Param::choice("format", "Format", &["msp", "massbank"])
                .default_str("msp")
                .doc("MSP, or MassBank records (MS_TYPE MS2, MS3, ... and the path of precursors in the comment)"),
            Param::choice("levels", "MS levels", &["all", "ms2"])
                .default_str("all")
                .doc("all: one record per spectrum of the tree, MS2 and deeper; ms2: only the spectra of the first fragmentation"),
            Param::boolean("clean", "Explained peaks only")
                .default_bool(false)
                .doc("Keep only the peaks a node of the graph explains"),
            Param::boolean("annotate", "Annotate peaks")
                .default_bool(true)
                .doc("Write the sub-formula of each explained peak and the neutral loss from its precursor (MSP: quoted note; MassBank: PK$ANNOTATION and a COMMENT)"),
            Param::text("id", "Record name")
                .default_str("")
                .doc("Name of the records; empty = the formula of the ion of the root, or MSN"),
        ],
        runs_in: RunsIn {
            wasm: Some("wasip1 (mzlab-cli)"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.msnlib.bad"],
        ..Step::core(
            "msn.library_export",
            "MSn tree as a library",
            "Writes the spectra of the fragmentation graph as an MSP or MassBank library, one record per spectrum of the tree, with the proposed formula and the neutral loss of each explained peak. The formulas are candidates of the graph, not identifications.",
        )
    }
}

/// `msn.table_export@1`: the graph as a CSV table.
pub fn table_step() -> Step {
    Step {
        ins: vec![graph_port()],
        outs: vec![Port::new("table", DataType::Report, "Table of the graph (CSV)")],
        params: vec![
            Param::choice("view", "Table", &["modifications", "nodes", "peaks"])
                .default_str("modifications")
                .doc("modifications: one row per neutral loss between two ions of the tree (mass, formula, ppm, measures); nodes: one row per ion with its proposed sub-formula; peaks: one row per peak of every spectrum with its node"),
            Param::boolean("clean", "Explained peaks only")
                .default_bool(false)
                .doc("peaks table only: keep the peaks a node of the graph explains"),
        ],
        runs_in: RunsIn {
            wasm: Some("wasip1 (mzlab-cli)"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.msnlib.bad"],
        ..Step::core(
            "msn.table_export",
            "MSn graph as a table",
            "Writes the fragmentation graph as CSV: its ions with the proposed sub-formulas, its modifications (neutral losses between ions) or its peaks. The formulas are candidates of the graph, not identifications.",
        )
    }
}

fn param<'a>(job: &'a Job, name: &str) -> Option<&'a Value> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
}

fn flag(job: &Job, name: &str) -> bool {
    param(job, name).and_then(Value::bool) == Some(true)
}

fn graph_of(r: &FileRunner, job: &Job) -> Result<(Value, String)> {
    let text = String::from_utf8(read_one(r, job, "graph")?.1)
        .map_err(|_| Error::new("err.msnlib.bad"))?;
    let g = parse(text.as_bytes())?;
    if g.get("nodi").is_none() || g.get("spettri").is_none() {
        return Err(Error::new("err.msnlib.bad"));
    }
    Ok((g, text))
}

/// The library text of the graph `g` (written as `json`) with the parameters of the step.
pub(crate) fn library_text(g: &Value, json: &str, job: &Job) -> Result<String> {
    let text_param = |n: &str, d: &'static str| param(job, n).and_then(Value::str).unwrap_or(d);
    msngrafo::library_text(
        g,
        json,
        (
            text_param("id", ""),
            text_param("format", "msp"),
            text_param("levels", "all"),
            flag(job, "clean"),
            flag(job, "annotate"),
        ),
    )
}

pub(crate) fn library_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (g, json) = graph_of(r, job)?;
    let text = library_text(&g, &json, job)?;
    let ext = if param(job, "format").and_then(Value::str) == Some("massbank") {
        "txt"
    } else {
        "msp"
    };
    let out = r.work.join(format!("{}.{ext}", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Library)
}

pub(crate) fn table_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (g, _) = graph_of(r, job)?;
    let view = param(job, "view")
        .and_then(Value::str)
        .unwrap_or("modifications");
    let text = msngrafo::table_text(&g, view, flag(job, "clean"));
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Report)
}
