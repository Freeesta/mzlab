//! The `tree` algorithm of `spectra.match@1`: the graph `msn-graph/1` in `query` against the library `msn-lib/1`, by `msngraph::cerca`. The output is its
//! `msn-search/1` text, or with `tree_output=candidates` the `candidates/1` of the other algorithms (the three scores and the proofs stay in a `tree` block).

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::json::{parse, round, Out, Value};
use mzlab_core::msngraph::{cerca, OpzioniCerca as Options};

use crate::flow_match::{num, Named};

fn param<'a>(job: &'a Job, name: &str) -> &'a str {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.str()).unwrap_or("")
}

fn text<'a>(f: Named<'a>) -> Result<&'a str> {
    std::str::from_utf8(f.1).map_err(|_| Error::new("err.flow.match.format").with("file", f.0))
}

/// The m/z of the root of the graph (its precursor), else the theoretical m/z of its ion.
fn root_mz(g: &Value) -> Option<f64> {
    let root = g.get("roots")?.at(0)?.str()?;
    let node = g
        .get("nodes")?
        .arr()
        .iter()
        .find(|n| n.get("id").and_then(Value::str) == Some(root));
    node.and_then(|n| n.get("mz")?.f64())
        .or_else(|| g.get("ion")?.get("theoretical_mz")?.f64())
}

/// Runs the search of the job on the texts in `query` (graph) and `library`.
pub(crate) fn tree_text(job: &Job, query: Named, library: Named) -> Result<String> {
    let graph = parse(text(query)?.as_bytes()).map_err(|_| Error::new("err.msnlib.bad"))?;
    let da = num(job, "precursor_da");
    // a window in Da is the same window in ppm of the precursor of the graph
    let ppm_prec = match root_mz(&graph).filter(|mz| da > 0.0 && *mz > 0.0) {
        Some(mz) => da / mz * 1e6,
        None => num(job, "precursor_ppm"),
    };
    let o = Options {
        ppm_prec,
        tol_mz: num(job, "tol_da"),
        min_evidence: num(job, "min_evidence").max(0.0) as usize,
        top: num(job, "top_n").max(1.0) as usize,
        ..Options::default()
    };
    let found = cerca(text(library)?, text(query)?, &o)?;
    let name = param(job, "name_contains").to_lowercase();
    if param(job, "tree_output") != "candidates" && name.is_empty() {
        return Ok(found);
    }
    let lib = parse(text(library)?.as_bytes()).map_err(|_| Error::new("err.msnlib.bad"))?;
    let hits = parse(found.as_bytes()).map_err(|_| Error::new("err.msnlib.bad"))?;
    let qid = graph
        .get("roots")
        .and_then(|r| r.at(0))
        .and_then(Value::str)
        .unwrap_or("q0");
    candidates(job, &lib, &hits, qid, &name)
}

/// The entry of the library with this id and ion formula (an entry is one ion in one condition).
fn entry<'a>(lib: &'a Value, id: &str, ion: &str) -> Option<&'a Value> {
    lib.get("entries")
        .map_or(&[][..], Value::arr)
        .iter()
        .find(|v| {
            v.get("id").and_then(Value::str) == Some(id)
                && v.get("ion").and_then(|i| i.get("formula")?.str()) == Some(ion)
        })
}

/// What a library entry is called: the private name when there is one, else its id.
fn label(v: Option<&Value>, id: &str) -> String {
    let private = v.and_then(|v| v.get("privato")?.get("name")?.str());
    private.unwrap_or(id).to_string()
}

fn candidates(job: &Job, lib: &Value, hits: &Value, qid: &str, name: &str) -> Result<String> {
    let mut o = Out::default();
    o.raw("{\"format\":\"candidates/1\",\"step\":")
        .str(&format!("{}@1", job.step.id));
    o.raw(",\"algorithm\":\"tree\",\"hits\":[");
    let mut first = true;
    for h in hits.get("candidates").map_or(&[][..], Value::arr) {
        let id = h.get("id").and_then(Value::str).unwrap_or("");
        let ion = h.get("ion").and_then(Value::str).unwrap_or("");
        let e = entry(lib, id, ion);
        let label = label(e, id);
        if !name.is_empty()
            && !label.to_lowercase().contains(name)
            && !id.to_lowercase().contains(name)
        {
            continue;
        }
        if !first {
            o.raw(",");
        }
        first = false;
        write_hit(&mut o, qid, &label, e, h);
    }
    o.raw("]}");
    Ok(o.0)
}

fn write_hit(o: &mut Out, qid: &str, label: &str, e: Option<&Value>, h: &Value) {
    let std = |k: &str| {
        e.and_then(|v| v.get("standard")?.get(k)?.str())
            .unwrap_or("")
            .to_string()
    };
    let score = |k: &str| h.get("scores").and_then(|p| p.get(k)).and_then(Value::f64);
    // the score is the MS2 entropy similarity (the scale of spectral_entropy); the tree and the losses are in the block
    let main = score("ms2").or_else(|| score("tree")).unwrap_or(0.0);
    let evidence = h.get("proofs");
    let proof = |k: &str| {
        evidence
            .and_then(|p| p.get(k))
            .and_then(Value::f64)
            .unwrap_or(0.0)
    };
    o.raw("{\"query\":").str(qid);
    o.raw(",\"id\":")
        .str(h.get("id").and_then(Value::str).unwrap_or(""));
    o.raw(",\"name\":").str(label);
    o.raw(",\"inchikey\":").str(&std("inchikey"));
    o.raw(",\"smiles\":").str(&std("smiles"));
    o.raw(",\"formula\":").str(&std("neutral_formula"));
    o.raw(",\"score\":").num(round(main, 6));
    o.raw(",\"shared\":").int(proof("common_peaks") as i64);
    o.raw(",\"mass_ppm\":")
        .num(h.get("delta_ppm").and_then(Value::f64).unwrap_or(0.0));
    o.raw(",\"source\":\"library\",\"algorithm\":\"tree\",\"tree\":{\"ion\":")
        .str(h.get("ion").and_then(Value::str).unwrap_or(""));
    o.raw(",\"ms2\":").opt(score("ms2"));
    o.raw(",\"tree\":").opt(score("tree"));
    o.raw(",\"losses\":").opt(score("losses"));
    o.raw(",\"aligned_nodes\":")
        .int(proof("aligned_nodes") as i64);
    o.raw(",\"unknown_nodes\":")
        .int(proof("unknown_nodes") as i64);
    o.raw(",\"common_losses\":")
        .int(proof("common_losses") as i64);
    o.raw(",\"unknown_losses\":")
        .int(proof("unknown_losses") as i64);
    o.raw("}}");
}
