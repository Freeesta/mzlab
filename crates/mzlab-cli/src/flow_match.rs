//! The native executor of `spectra.match@1`: the query spectra (MGF/MSP/MassBank/JSON) scored against a library (the same formats, or a `.mzlib`) with
//! the engine of `mzlab-lib` (`search::identity`, `search::analogs`, `scores`), as the Library tab does. The `candidates` file it writes is JSON.

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::json::{round, Out};
use mzlab_core::msngrafo::{cerca, OpzioniCerca as Opzioni};
use mzlab_core::score_terms::ScoreTerms;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::contrib::{
    cosine_contributions, entropy_contributions, modified_cosine_contributions,
};
use mzlab_lib::decoy::{
    best_scores, fdr_note, is_decoy, lib_fdr, quantize, with_decoys, Best, DecoyKind, LibFdr,
    DEFAULT_SEED,
};
use mzlab_lib::parse::{detect, parse_all};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::scores::modified_cosine;
use mzlab_lib::search::{analogs, identity, AnalogOpts, IdentityOpts, Query, Tol};
use mzlab_lib::spec::Spectrum;

type Lib = Library<Vec<u8>>;

/// A file the executors read: its name (the format is told from it and the content) and its bytes.
pub type Named<'a> = (&'a str, &'a [u8]);

/// The spectra of a text file in any of the library formats.
pub(crate) fn read_spectra((name, bytes): Named) -> Result<Vec<Spectrum>> {
    let fmt = detect(name, &bytes[..bytes.len().min(4096)])
        .map_err(|_| Error::new("err.flow.match.format").with("file", name))?;
    Ok(parse_all(fmt, bytes).0)
}

/// The library of the step; with `decoys` every entry gets a decoy beside it (text libraries only: an `.mzlib` is read as it is).
fn bad<E>(_: E) -> Error {
    Error::new("err.flow.match.library")
}

fn open_library(file: Named, decoys: bool) -> Result<Lib> {
    let bytes = if file.0.ends_with(".mzlib") {
        if !decoys {
            file.1.to_vec()
        } else {
            // an `.mzlib` is read back to spectra, so the decoys are made as for a text library
            let lib = Library::open(file.1.to_vec(), CACHE_BYTES).map_err(bad)?;
            let spectra = lib_spectra(&lib).map_err(bad)?;
            let spectra = with_decoys(&spectra, DecoyKind::Fragments, DEFAULT_SEED);
            build_in_memory(&spectra, BuildOpts::default())
                .map_err(bad)?
                .0
        }
    } else {
        let mut spectra = read_spectra(file)?;
        if decoys {
            spectra = with_decoys(&spectra, DecoyKind::Fragments, DEFAULT_SEED);
        }
        build_in_memory(&spectra, BuildOpts::default())
            .map_err(bad)?
            .0
    };
    Library::open(bytes, CACHE_BYTES).map_err(bad)
}

/// The target spectra of a built library (decoys it may already hold are dropped, to be made again).
fn lib_spectra(lib: &Lib) -> mzlab_lib::error::Result<Vec<Spectrum>> {
    let mut out = Vec::new();
    for i in 0..lib.n_spectra() {
        let meta = lib.meta(i)?;
        if is_decoy(&meta.accession) {
            continue;
        }
        let (mz, it) = lib.peaks(i)?;
        let rec = lib.rec(i)?;
        out.push(Spectrum {
            prec: rec.prec,
            pol: rec.pol,
            mz,
            it,
            meta,
        });
    }
    Ok(out)
}

/// The precursor window of the step: `precursor_da` when set (low resolution), else `ppm_name` in ppm.
fn prec_tol(job: &Job, ppm_name: &str) -> Tol {
    match num(job, "precursor_da") {
        da if da > 0.0 => Tol::Da(da),
        _ => Tol::Ppm(num(job, ppm_name)),
    }
}

/// The precursor error in ppm, whichever unit the window had (`Hit.dprec` is in the unit of the window).
fn dprec_ppm(tol: Tol, d: f64, prec: f64) -> f64 {
    match tol {
        Tol::Ppm(_) => d,
        Tol::Da(_) => d / prec * 1e6,
    }
}

fn text_param<'a>(job: &'a Job, name: &str) -> &'a str {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.str()).unwrap_or("")
}

pub(crate) fn num(job: &Job, name: &str) -> f64 {
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
    /// the score as a sum of terms, when the step is asked for it (`score_terms`)
    terms: Option<ScoreTerms>,
}

/// Pairs named one by one in the terms of a score.
const TERMS_SHOWN: usize = 6;

fn mod_terms(m: &mzlab_lib::scores::ModCosine) -> ScoreTerms {
    let c = modified_cosine_contributions(m);
    ScoreTerms::over_pairs(
        "modified_cosine",
        m.cosine,
        c,
        TERMS_SHOWN,
        "score/modified-cosine",
    )
}

fn by_score(c: &mut Vec<Cand>, top: usize) {
    c.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.index.cmp(&b.index)));
    c.truncate(top);
}

fn exact(lib: &Lib, q: &Query, job: &Job, alg: &str, terms: bool) -> Result<Vec<Cand>> {
    let tol = prec_tol(
        job,
        if alg == "modified_cosine" {
            "exact_ppm"
        } else {
            "precursor_ppm"
        },
    );
    let o = IdentityOpts {
        prec_tol: tol,
        frag_da: num(job, "tol_da"),
        pol: q.pol,
        top: usize::MAX,
    };
    let hits = identity(lib, q, &o).map_err(|_| Error::new("err.flow.match.library"))?;
    let mut out = Vec::new();
    for h in hits {
        let (score, shared, pairs, t) = match alg {
            "cosine" => (
                h.cosine,
                h.shared,
                h.pairs.clone(),
                terms.then(|| {
                    let c = cosine_contributions(&q.spec, &h.spec, &h.pairs);
                    ScoreTerms::over_pairs("cosine", h.cosine, c, TERMS_SHOWN, "score/cosine")
                }),
            ),
            "modified_cosine" => {
                let m = modified_cosine(&q.spec, &h.spec, o.frag_da);
                let p = m.pairs.iter().map(|&(i, j, _)| (i, j)).collect();
                (m.cosine, m.matched, p, terms.then(|| mod_terms(&m)))
            }
            _ => (
                h.entropy,
                h.shared,
                h.pairs.clone(),
                terms.then(|| {
                    let c = entropy_contributions(&q.spec, &h.spec, &h.pairs);
                    let n = "entropy_similarity";
                    ScoreTerms::over_pairs(n, h.entropy, c, TERMS_SHOWN, "score/entropy-similarity")
                }),
            ),
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
            mass_ppm: dprec_ppm(tol, h.dprec, q.prec),
            delta: 0.0,
            explained: pairs
                .iter()
                .filter_map(|&(i, _)| q.spec.mz.get(i).copied())
                .collect(),
            terms: t,
        });
    }
    Ok(out)
}

fn analogues(lib: &Lib, q: &Query, job: &Job, terms: bool) -> Result<Vec<Cand>> {
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
            terms: terms.then(|| mod_terms(&h.mc)),
        })
        .collect())
}

/// The target-decoy numbers of one query: its best target and decoy and the q-value of the best target within the batch.
struct Fdr {
    best: Best,
    fdr: LibFdr,
}

/// The identity search of every query of the batch against the library with decoys (the entropy score, as `decoy::search_fdr`), one `Fdr` per query.
fn batch_fdr(lib: &Lib, queries: &[Option<Query>], job: &Job) -> Result<Vec<Fdr>> {
    let mut best = Vec::with_capacity(queries.len());
    for q in queries {
        best.push(match q {
            Some(q) => {
                let o = IdentityOpts {
                    prec_tol: prec_tol(job, "precursor_ppm"),
                    frag_da: num(job, "tol_da"),
                    pol: q.pol,
                    top: usize::MAX,
                };
                best_scores(lib, q, &o).map_err(|_| Error::new("err.flow.match.library"))?
            }
            None => Best::default(),
        });
    }
    let target = |b: &Best| {
        if b.index.is_some() {
            b.target
        } else {
            f32::NAN
        }
    };
    let t: Vec<f32> = best.iter().map(target).collect();
    let d: Vec<f32> = best.iter().map(|b| b.decoy).collect();
    let f = lib_fdr(&t, &d, DecoyKind::Fragments);
    Ok(best
        .into_iter()
        .zip(f)
        .map(|(best, fdr)| Fdr { best, fdr })
        .collect())
}

fn write_cand(
    o: &mut Out,
    lib: &Lib,
    qid: &str,
    alg: &str,
    c: &Cand,
    fdr: Option<&Fdr>,
) -> Result<()> {
    let m = lib
        .meta(c.index)
        .map_err(|_| Error::new("err.flow.match.library"))?;
    let r = |v: f64| round(v, 6);
    let peaks = lib
        .peaks(c.index)
        .map_err(|_| Error::new("err.flow.match.library"))?
        .0
        .len();
    o.raw("{\"query\":").str(qid);
    o.raw(",\"id\":").str(&m.accession);
    o.raw(",\"name\":").str(&m.name);
    o.raw(",\"inchikey\":").str(&m.inchikey);
    o.raw(",\"smiles\":").str(&m.smiles);
    o.raw(",\"formula\":").str(&m.formula);
    o.raw(",\"score\":").num(r(c.score));
    if let Some(t) = &c.terms {
        o.raw(",\"score_terms\":");
        t.write(o);
    }
    o.raw(",\"shared\":").int(c.shared as i64);
    o.raw(",\"library_peaks\":").int(peaks as i64);
    o.raw(",\"explained\":").rounded(&c.explained, 4);
    o.raw(",\"mass_ppm\":").num(round(c.mass_ppm, 3));
    o.raw(",\"precursor_delta\":").num(round(c.delta, 4));
    o.raw(",\"source\":\"library\",\"algorithm\":").str(alg);
    // only the best target of the query carries a q-value (the score the decoys were counted on)
    if let Some(f) = fdr {
        if f.best.index.is_some() && quantize(c.score) == f.best.target {
            o.raw(",\"decoy_q\":").num(r(f.fdr.q_value as f64));
            o.raw(",\"decoy_targets_ge\":")
                .int(f.fdr.n_target_ge as i64);
            o.raw(",\"decoy_ge\":").int(f.fdr.n_decoy_ge as i64);
        } else {
            o.raw(",\"decoy_q\":null");
        }
    }
    o.raw("}");
    Ok(())
}

/// Runs the step of `job` on the query and library files and returns the text of the `candidates` file: one object per candidate, queries in file order,
/// each query's candidates best score first, at most `top_n` of them.
pub fn match_text(job: &Job, query: Named, library: Named) -> Result<String> {
    let alg = job
        .resolved
        .algorithm
        .as_ref()
        .map_or("spectral_entropy", |a| a.0.as_str())
        .to_string();
    let decoys = job
        .resolved
        .params
        .iter()
        .any(|(k, v)| k == "decoys" && v.str() == Some("spectrum"));
    if decoys && alg != "spectral_entropy" {
        return Err(Error::new("err.flow.match.decoysAlgorithm"));
    }
    let terms = job
        .resolved
        .params
        .iter()
        .any(|(k, v)| k == "score_terms" && v.bool() == Some(true));
    if alg == "tree" {
        return tree_text(job, query, library);
    }
    let lib = open_library(library, decoys)?;
    let name = text_param(job, "name_contains").to_lowercase();
    let top = num(job, "top_n").max(1.0) as usize;
    let mut o = Out::default();
    o.raw("{\"format\":\"candidates/1\",\"step\":")
        .str(&format!("{}@1", job.step.id));
    o.raw(",\"algorithm\":").str(&alg);
    if decoys {
        o.raw(",\"decoys\":\"spectrum\",\"fdr_note\":")
            .str(fdr_note());
    }
    o.raw(",\"hits\":[");
    let spectra = read_spectra(query)?;
    let queries: Vec<Option<Query>> = spectra
        .iter()
        .map(|s| Query::new(s.prec, s.pol, &s.mz, &s.it))
        .collect();
    let fdr = if decoys {
        Some(batch_fdr(&lib, &queries, job)?)
    } else {
        None
    };
    let mut first = true;
    for (k, s) in spectra.iter().enumerate() {
        let qid = query_id(k, s);
        let Some(q) = &queries[k] else {
            continue;
        };
        let mut c = exact(&lib, q, job, &alg, terms)?;
        if alg == "modified_cosine" {
            c.extend(analogues(&lib, q, job, terms)?);
        }
        if !name.is_empty() {
            c.retain(|h| {
                lib.meta(h.index)
                    .is_ok_and(|m| m.name.to_lowercase().contains(&name))
            });
        }
        if decoys {
            // the decoys are scored to count the errors, never listed
            c.retain(|h| lib.meta(h.index).is_ok_and(|m| !is_decoy(&m.accession)));
        }
        by_score(&mut c, top);
        for h in &c {
            if !first {
                o.raw(",");
            }
            first = false;
            write_cand(&mut o, &lib, &qid, &alg, h, fdr.as_ref().map(|f| &f[k]))?;
        }
    }
    o.raw("]}");
    Ok(o.0)
}

/// The `tree` algorithm: the graph `msngrafo/1` in `query` against the library `msnlib/1`, written as the `msncerca/1` text of the engine.
fn tree_text(job: &Job, query: Named, library: Named) -> Result<String> {
    let o = Opzioni {
        ppm_prec: num(job, "precursor_ppm"),
        tol_mz: num(job, "tol_da"),
        min_prove: num(job, "min_prove").max(0.0) as usize,
        top: num(job, "top_n").max(1.0) as usize,
        ..Opzioni::default()
    };
    cerca(text(library)?, text(query)?, &o)
}

fn text<'a>(f: Named<'a>) -> Result<&'a str> {
    std::str::from_utf8(f.1).map_err(|_| Error::new("err.flow.match.format").with("file", f.0))
}

/// The identifier of the k-th query spectrum: its accession or title, else `q<k>`.
pub(crate) fn query_id(k: usize, s: &Spectrum) -> String {
    if s.meta.accession.is_empty() {
        format!("q{k}")
    } else {
        s.meta.accession.clone()
    }
}
