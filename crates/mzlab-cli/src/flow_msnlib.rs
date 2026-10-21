//! The native executors of `msn.library_export@1` and `msn.table_export@1`: the fragmentation graph `msngrafo/1` of an infusion (`spectra.msn_graph`) written
//! as a spectral library (MSP or MassBank, every MS level, with the formula and the neutral loss of each explained peak) or as a CSV table of its nodes,
//! its modifications (the neutral losses between ions) or its peaks. Both read the graph text, so the plan and `mzlab-cli msn-lib export` write the same bytes.

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow::{io, read_one, Exec, FileRunner};
use crate::msn_lib_export::{export_text, Formato};

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

/// The library text of the graph `g` (written as `json`): the entry is wrapped as a one-entry `msnlib/1` and given to the writer of `msn-lib export`.
pub(crate) fn library_text(g: &Value, json: &str, job: &Job) -> Result<String> {
    let ion = g
        .get("ione")
        .and_then(|i| i.get("formula")?.str())
        .unwrap_or("");
    let given = param(job, "id").and_then(Value::str).unwrap_or("");
    let id = match (given, ion) {
        ("", "") => "MSN",
        ("", f) => f,
        (n, _) => n,
    };
    let mut o = Out::default();
    o.raw("{\"voci\":[{\"id\":").str(id);
    o.raw(",\"ione\":{\"formula\":").str(ion);
    o.raw("},\"grafo\":").raw(json.trim()).raw("}]}");
    let formato = Formato::parse(param(job, "format").and_then(Value::str).unwrap_or("msp"))?;
    let all = param(job, "levels").and_then(Value::str) != Some("ms2");
    export_text(
        &o.0,
        formato,
        (flag(job, "clean"), all, flag(job, "annotate")),
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

fn cell(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn num(v: Option<&Value>, places: usize) -> String {
    v.and_then(Value::f64)
        .map_or(String::new(), |x| format!("{x:.places$}"))
}

fn text(v: Option<&Value>) -> String {
    cell(v.and_then(Value::str).unwrap_or(""))
}

fn find<'a>(g: &'a Value, list: &str, id: Option<&str>) -> Option<&'a Value> {
    g.get(list)?
        .arr()
        .iter()
        .find(|n| n.get("id").and_then(Value::str) == id)
}

/// The chosen sub-formula of a node and its error in ppm.
fn formula_of(n: Option<&Value>) -> (String, String) {
    let f = n.and_then(|n| n.get("formula"));
    let name = f.and_then(|f| f.get("scelta")).and_then(Value::str);
    let ppm = f.and_then(|f| {
        f.get("candidati")?
            .arr()
            .iter()
            .find(|c| c.get("formula").and_then(Value::str) == name)?
            .get("ppm")
    });
    (cell(name.unwrap_or("")), num(ppm, 2))
}

fn nodes_csv(g: &Value) -> String {
    let mut t = String::from("node,level,mz,dev_ppm,observations,rel_max,formula,formula_ppm,formula_source,candidates,isolated,tree\n");
    for n in g.get("nodi").map_or(&[][..], |x| x.arr()) {
        let (formula, ppm) = formula_of(Some(n));
        let f = n.get("formula");
        let candidates = f
            .and_then(|f| f.get("candidati"))
            .map_or(0, |c| c.arr().len());
        let tree: Vec<&str> = n
            .get("albero")
            .map_or(&[][..], |a| a.arr())
            .iter()
            .filter_map(Value::str)
            .collect();
        t += &format!(
            "{},{},{},{},{},{},{formula},{ppm},{},{candidates},{},{}\n",
            text(n.get("id")),
            num(n.get("livello"), 0),
            num(n.get("mz"), 5),
            num(n.get("dev_ppm"), 2),
            num(n.get("osservazioni"), 0),
            num(n.get("rel_max"), 2),
            text(f.and_then(|f| f.get("fonte"))),
            n.get("isolato").and_then(Value::bool).unwrap_or(false),
            cell(&tree.join("|")),
        );
    }
    t
}

fn modifications_csv(g: &Value) -> String {
    let mut t = String::from("from,from_mz,from_formula,to,to_mz,to_formula,loss_mass,loss_formula,loss_ppm,isolation_step,tree,level,activation,ce,scans,rel\n");
    for a in g.get("archi").map_or(&[][..], |x| x.arr()) {
        let (from, to) = (
            find(g, "nodi", a.get("da").and_then(Value::str)),
            find(g, "nodi", a.get("a").and_then(Value::str)),
        );
        let loss = a.get("perdita");
        let head = format!(
            "{},{},{},{},{},{},{},{},{},{}",
            text(a.get("da")),
            num(from.and_then(|n| n.get("mz")), 5),
            formula_of(from).0,
            text(a.get("a")),
            num(to.and_then(|n| n.get("mz")), 5),
            formula_of(to).0,
            num(loss.and_then(|l| l.get("da_massa")), 5),
            text(loss.and_then(|l| l.get("formula"))),
            num(loss.and_then(|l| l.get("ppm")), 2),
            a.get("isolamento").and_then(Value::bool).unwrap_or(false),
        );
        let measures = a.get("misure").map_or(&[][..], |m| m.arr());
        if measures.is_empty() {
            t += &format!("{head},,,,,,\n");
        }
        for m in measures {
            t += &format!(
                "{head},{},{},{},{},{},{}\n",
                text(m.get("albero")),
                num(m.get("livello"), 0),
                text(m.get("att")),
                num(m.get("ce"), 1),
                num(m.get("n_scansioni"), 0),
                num(m.get("rel"), 2),
            );
        }
    }
    t
}

fn peaks_csv(g: &Value, clean: bool) -> String {
    let mut t =
        String::from("tree,level,activation,ce,precursor_mz,mz,rel,node,formula,formula_ppm\n");
    for s in g.get("spettri").map_or(&[][..], |x| x.arr()) {
        let pre = find(g, "nodi", s.get("precursore").and_then(Value::str));
        for p in s.get("picchi").map_or(&[][..], |x| x.arr()) {
            let node = p.get("nodo").and_then(Value::str);
            if clean && node.is_none() {
                continue;
            }
            let (formula, ppm) = formula_of(find(g, "nodi", node));
            t += &format!(
                "{},{},{},{},{},{},{},{},{formula},{ppm}\n",
                text(s.get("albero")),
                num(s.get("livello"), 0),
                text(s.get("att")),
                num(s.get("ce"), 1),
                num(pre.and_then(|n| n.get("mz")), 5),
                num(p.get("mz"), 5),
                num(p.get("rel"), 2),
                cell(node.unwrap_or("")),
            );
        }
    }
    t
}

/// The CSV text of the table `view` of the graph `g`.
pub(crate) fn table_text(g: &Value, view: &str, clean: bool) -> String {
    match view {
        "nodes" => nodes_csv(g),
        "peaks" => peaks_csv(g, clean),
        _ => modifications_csv(g),
    }
}

pub(crate) fn table_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (g, _) = graph_of(r, job)?;
    let view = param(job, "view")
        .and_then(Value::str)
        .unwrap_or("modifications");
    let text = table_text(&g, view, flag(job, "clean"));
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Report)
}
