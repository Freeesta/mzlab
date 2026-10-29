//! The JSON face of the matcher for the `.wasm` of the site: parameters in as JSON, errors out as `{error_key, params}` texts; the same `match_text` the CLI calls.

use super::{match_text, MatchJob, Named};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};

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
