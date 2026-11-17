//! The JSON face of the matcher for the `.wasm` of the site: parameters in as JSON, errors out as `{error_key, params}` texts; the same `match_text` the CLI calls.

use super::{evidence as ev, match_text, Cand, Fdr, Lib, MatchJob, Named};
use crate::decoy::quantize;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{round, Out, Value};

/// The parameters of the step as the object the page sends (the parameters merged with the defaults of the descriptor); anything but an object is no parameter.
pub fn params_of_json(text: &str) -> Result<Vec<(String, Value)>> {
    match mzlab_core::json::parse(text.as_bytes())? {
        Value::Obj(o) => Ok(o),
        _ => Ok(Vec::new()),
    }
}

/// `{"error_key":..,"params":{..}}`, the text every `.wasm` of the site gives an error in.
pub fn error_json(e: &Error) -> String {
    let mut o = Out::default();
    o.raw("{\"error_key\":").str(e.key).raw(",\"params\":{");
    for (i, (k, v)) in e.params.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
    }
    o.raw("}}");
    o.0
}

/// `match_text` for the browser: the parameters as JSON, the error as the JSON of `error_json`. An empty `algorithm` is the default one of the step.
pub fn match_json(
    step: &str,
    algorithm: &str,
    params_json: &str,
    query: Named,
    library: Named,
) -> std::result::Result<String, String> {
    let params = params_of_json(params_json).map_err(|e| error_json(&e))?;
    let job = MatchJob {
        step,
        algorithm: Some(algorithm).filter(|a| !a.is_empty()),
        params: &params,
    };
    match_text(&job, query, library).map_err(|e| error_json(&e))
}

/// Serializes one candidate hit to JSON (`candidates/1` schema).
pub(crate) fn write_cand(
    o: &mut Out,
    lib: &Lib,
    qid: &str,
    alg: &str,
    c: &Cand,
    fdr: Option<&Fdr>,
    rt_delta: Option<f64>,
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
    if let Some(m) = &c.matched {
        ev::write(o, m);
    }
    o.raw(",\"mass_ppm\":").num(round(c.mass_ppm, 3));
    o.raw(",\"precursor_delta\":").num(round(c.delta, 4));
    match &c.delta_formula {
        Some(df) => {
            o.raw(",\"delta_formula\":").str(&df.formula);
            o.raw(",\"delta_formula_err_mda\":")
                .num(round(df.err_mda, 3));
        }
        None => {
            o.raw(",\"delta_formula\":null,\"delta_formula_err_mda\":null");
        }
    }
    if let Some(d) = rt_delta {
        o.raw(",\"rt_delta_min\":").num(round(d, 3));
    }
    o.raw(",\"source\":\"library\",\"algorithm\":").str(alg);
    // only the best target of the query carries a q-value (the score the decoys were counted on)
    if let Some(f) = fdr {
        if f.best.index.is_some() && quantize(c.score) == f.best.target {
            o.raw(",\"decoy_q\":").num(r(f.fdr.q_value as f64));
            o.raw(",\"decoy_targets_ge\":")
                .int(f.fdr.n_target_ge as i64);
            o.raw(",\"decoy_ge\":").int(f.fdr.n_decoy_ge as i64);
            o.raw(",\"best_decoy\":").num(r(f.best.decoy as f64));
        } else {
            o.raw(",\"decoy_q\":null");
        }
    }
    o.raw("}");
    Ok(())
}
