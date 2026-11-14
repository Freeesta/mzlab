//! `spectra.lookup@1`: the entries of a spectral library the user declares that carry a name, formula, InChIKey, accession or precursor m/z, written as an MGF.
//! One function for `flow run` and the `.wasm` of the site, so the text is the same byte for byte. Nothing is built in and nothing is looked up online.

use std::fmt::Write as _;

use super::{error_json, library_spectra, params_of_json, Named};
use crate::spec::Spectrum;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

fn param<'a>(params: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    params.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

/// Does the entry carry the value in the field `by`?
fn keeps(params: &[(String, Value)], by: &str, value: &str, s: &Spectrum) -> Result<bool> {
    let low = value.to_lowercase();
    let exact = param(params, "exact").and_then(|v| v.bool()) == Some(true);
    Ok(match by {
        "name" if exact => s.meta.name.to_lowercase() == low,
        "name" => s.meta.name.to_lowercase().contains(&low),
        "formula" => s.meta.formula.replace(' ', "") == value.replace(' ', ""),
        // a 14-letter value is the first block, the skeleton without the stereochemistry
        "inchikey" if low.len() == 14 => s.meta.inchikey.to_lowercase().starts_with(&low),
        "inchikey" => s.meta.inchikey.to_lowercase() == low,
        "accession" => s.meta.accession == value,
        "precursor_mz" => {
            let mz: f64 = value
                .parse()
                .map_err(|_| Error::new("err.flow.lookup.value"))?;
            let ppm = param(params, "ppm").and_then(|v| v.f64()).unwrap_or(10.0);
            (s.prec - mz).abs() <= mz * ppm * 1e-6
        }
        _ => return Err(Error::new("err.flow.lookup.value")),
    })
}

fn block(out: &mut String, k: usize, s: &Spectrum) {
    let m = &s.meta;
    let title = if m.accession.is_empty() {
        format!("lib.{k}")
    } else {
        m.accession.clone()
    };
    let _ = writeln!(out, "BEGIN IONS\nTITLE={title}\nPEPMASS={:.5}", s.prec);
    let mode = match s.pol {
        1 => "Positive",
        -1 => "Negative",
        _ => "",
    };
    let fields = [
        ("NAME", &m.name),
        ("ADDUCT", &m.adduct),
        ("FORMULA", &m.formula),
        ("INCHIKEY", &m.inchikey),
        ("SMILES", &m.smiles),
        ("ACCESSION", &m.accession),
        ("IONMODE", &mode.to_string()),
    ];
    for (key, v) in fields.iter().filter(|(_, v)| !v.is_empty()) {
        let _ = writeln!(out, "{key}={v}");
    }
    for (mz, it) in s.mz.iter().zip(&s.it) {
        let _ = writeln!(out, "{mz:.5} {it}");
    }
    out.push_str("END IONS\n");
}

/// The MGF of the library entries that match `value` in the field `by` (parameters of the step merged with the defaults of its descriptor).
pub fn lookup_text(params: &[(String, Value)], library: Named) -> Result<String> {
    let value = param(params, "value")
        .and_then(|v| v.str())
        .unwrap_or("")
        .trim();
    if value.is_empty() {
        return Err(Error::new("err.flow.lookup.value"));
    }
    let by = match param(params, "by").and_then(|v| v.str()) {
        None | Some("") => "name",
        Some(b) => b,
    };
    let top = param(params, "top_n")
        .and_then(|v| v.f64())
        .unwrap_or(50.0)
        .max(1.0) as usize;
    let mut out = String::new();
    let mut kept = 0;
    for (k, s) in library_spectra(library)?.iter().enumerate() {
        if kept < top && keeps(params, by, value, s)? {
            block(&mut out, k, s);
            kept += 1;
        }
    }
    Ok(out)
}

/// `lookup_text` for the browser: the parameters as JSON, the error as the JSON of `error_json`.
pub fn lookup_json(params_json: &str, library: Named) -> std::result::Result<String, String> {
    let params = params_of_json(params_json).map_err(|e| error_json(&e))?;
    lookup_text(&params, library).map_err(|e| error_json(&e))
}
