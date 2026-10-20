//! The native executor of `spectra.match@1`: the query spectra (MGF/MSP/MassBank/JSON) scored against a library (the same formats, or a `.mzlib`) with
//! the engine of `mzlab-lib` (`search::identity`, `search::analogs`, `scores`), as the Library tab does. The `candidates` file it writes is JSON.

use std::path::Path;

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::json::{round, Out};
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::parse::{detect, parse_all};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::scores::modified_cosine;
use mzlab_lib::search::{analogs, identity, AnalogOpts, IdentityOpts, Query, Tol};
use mzlab_lib::spec::Spectrum;

type Lib = Library<Vec<u8>>;

fn io(e: std::io::Error) -> Error {
    Error::new("err.flow.io").with("detail", e)
}

/// The spectra of a text file in any of the library formats.
fn read_spectra(path: &Path) -> Result<Vec<Spectrum>> {
    let bytes = std::fs::read(path).map_err(io)?;
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let fmt = detect(&name, &bytes[..bytes.len().min(4096)])
        .map_err(|_| Error::new("err.flow.match.format").with("file", &name))?;
    Ok(parse_all(fmt, &bytes).0)
}

fn open_library(path: &Path) -> Result<Lib> {
    let bytes = if path.extension().is_some_and(|e| e == "mzlib") {
        std::fs::read(path).map_err(io)?
    } else {
        let spectra = read_spectra(path)?;
        build_in_memory(&spectra, BuildOpts::default())
            .map_err(|_| Error::new("err.flow.match.library"))?
            .0
    };
    Library::open(bytes, CACHE_BYTES).map_err(|_| Error::new("err.flow.match.library"))
}

fn num(job: &Job, name: &str) -> f64 {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| v.f64())
        .unwrap_or(0.0)
}

/// One candidate of the `candidates` record: the hit of a library spectrum with its score.
struct Cand {
    index: u32,
    score: f64,
    shared: usize,
    mass_ppm: f64,
    delta: f64,
    explained: Vec<f64>,
}

fn by_score(c: &mut Vec<Cand>, top: usize) {
    c.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.index.cmp(&b.index)));
    c.truncate(top);
}

fn exact(lib: &Lib, q: &Query, job: &Job, alg: &str) -> Result<Vec<Cand>> {
    let ppm = if alg == "modified_cosine" {
        num(job, "exact_ppm")
    } else {
        num(job, "precursor_ppm")
    };
    let o = IdentityOpts {
        prec_tol: Tol::Ppm(ppm),
        frag_da: num(job, "tol_da"),
        pol: q.pol,
        top: usize::MAX,
    };
    let hits = identity(lib, q, &o).map_err(|_| Error::new("err.flow.match.library"))?;
    let mut out = Vec::new();
    for h in hits {
        let (score, shared, pairs) = match alg {
            "cosine" => (h.cosine, h.shared, h.pairs.clone()),
            "modified_cosine" => {
                let m = modified_cosine(&q.spec, &h.spec, o.frag_da);
                let p = m.pairs.iter().map(|&(i, j, _)| (i, j)).collect();
                (m.cosine, m.matched, p)
            }
            _ => (h.entropy, h.shared, h.pairs.clone()),
        };
        // the gates of the entropy algorithm; the other two rank everything inside the precursor window
        if alg == "spectral_entropy"
            && (score < num(job, "min_score") || (shared as f64) < num(job, "min_shared"))
        {
            continue;
        }
        out.push(Cand {
            index: h.index,
            score,
            shared,
            mass_ppm: h.dprec,
            delta: 0.0,
            explained: pairs
                .iter()
                .filter_map(|&(i, _)| q.spec.mz.get(i).copied())
                .collect(),
        });
    }
    Ok(out)
}

fn analogues(lib: &Lib, q: &Query, job: &Job) -> Result<Vec<Cand>> {
    let o = AnalogOpts {
        frag_da: num(job, "tol_da"),
        pol: q.pol,
        max_delta: num(job, "analog_window_da"),
        min_delta: num(job, "analog_min_shift_da"),
        candidates: num(job, "analog_prefilter") as usize,
        top: usize::MAX,
        ..AnalogOpts::default()
    };
    let hits = analogs(lib, q, &o).map_err(|_| Error::new("err.flow.match.library"))?;
    Ok(hits
        .into_iter()
        .map(|h| Cand {
            index: h.index,
            score: h.mc.cosine,
            shared: h.mc.matched,
            mass_ppm: (h.prec - q.prec) / q.prec * 1e6,
            delta: h.mc.delta,
            explained: h
                .mc
                .pairs
                .iter()
                .filter_map(|&(i, _, _)| h.mc.a_top.mz.get(i).copied())
                .collect(),
        })
        .collect())
}

fn write_cand(o: &mut Out, lib: &Lib, qid: &str, alg: &str, c: &Cand) -> Result<()> {
    let m = lib
        .meta(c.index)
        .map_err(|_| Error::new("err.flow.match.library"))?;
    let r = |v: f64| round(v, 6);
    o.raw("{\"query\":").str(qid);
    o.raw(",\"id\":").str(&m.accession);
    o.raw(",\"name\":").str(&m.name);
    o.raw(",\"smiles\":").str(&m.smiles);
    o.raw(",\"formula\":").str(&m.formula);
    o.raw(",\"score\":").num(r(c.score));
    o.raw(",\"shared\":").int(c.shared as i64);
    o.raw(",\"explained\":").rounded(&c.explained, 4);
    o.raw(",\"mass_ppm\":").num(round(c.mass_ppm, 3));
    o.raw(",\"precursor_delta\":").num(round(c.delta, 4));
    o.raw(",\"source\":\"library\",\"algorithm\":")
        .str(alg)
        .raw("}");
    Ok(())
}

/// Runs the step of `job` on the query and library files and returns the text of the `candidates` file: one object per candidate, queries in file order,
/// each query's candidates best score first, at most `top_n` of them.
pub fn match_text(job: &Job, query: &Path, library: &Path) -> Result<String> {
    let alg = job
        .resolved
        .algorithm
        .as_ref()
        .map_or("spectral_entropy", |a| a.0.as_str())
        .to_string();
    let lib = open_library(library)?;
    let top = num(job, "top_n").max(1.0) as usize;
    let mut o = Out::default();
    o.raw("{\"format\":\"candidates/1\",\"step\":\"spectra.match@1\",\"algorithm\":")
        .str(&alg)
        .raw(",\"hits\":[");
    let mut first = true;
    for (k, s) in read_spectra(query)?.iter().enumerate() {
        let qid = if s.meta.accession.is_empty() {
            format!("q{k}")
        } else {
            s.meta.accession.clone()
        };
        let Some(q) = Query::new(s.prec, s.pol, &s.mz, &s.it) else {
            continue;
        };
        let mut c = exact(&lib, &q, job, &alg)?;
        if alg == "modified_cosine" {
            c.extend(analogues(&lib, &q, job)?);
        }
        by_score(&mut c, top);
        for h in &c {
            if !first {
                o.raw(",");
            }
            first = false;
            write_cand(&mut o, &lib, &qid, &alg, h)?;
        }
    }
    o.raw("]}");
    Ok(o.0)
}
