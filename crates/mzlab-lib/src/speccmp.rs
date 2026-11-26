//! `spectrum-compare/1`: two MS2 spectra binned at nominal mass (the `rebin=` answers of `/api/spectrum`) compared with the engine's cosine
//! (`scores::cosine_from_pairs`, square-root intensities). Site (`.wasm`), app and CLI call `compare_json`; the callers only select the spectra.

use crate::scores::{cosine_from_pairs, match_peaks};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, round, Out, Value};

/// Default tolerance between two bin centres, Da: 0 = the same bin.
pub const TOL_DA: f64 = 0.0;
/// Default collision-energy difference, eV, above which the two spectra are said to differ.
pub const CE_TOL: f64 = 0.5;
/// Two precursors further apart than this, Da, are not the same ion (the route's own window is 0.6).
const PRECURSOR_DA: f64 = 0.6;
const CE_UNKNOWN: &str = "warn.spectrum.ceUnknown";
const CE_MIXED: &str = "warn.spectrum.ceMixed";
const CE_DIFFER: &str = "warn.spectrum.ceDiffer";
const PRECURSOR_DIFFER: &str = "warn.spectrum.precursorDiffer";

/// One side: its label, the precursor and collision energies the caller found in the scans, and the bins.
struct Side {
    name: String,
    precursor: Option<f64>,
    ces: Vec<f64>,
    bin: Vec<f64>,
    mz: Vec<f64>,
    y: Vec<f64>,
    /// Bins with no intensity above zero, left out.
    dropped: usize,
    /// The route query that selected the scans, as the caller sent it, and the number of scans the route averaged.
    query: Option<String>,
    scans: Option<f64>,
}

impl Side {
    /// The one collision energy of the side, or `None` when the scans carry none or several.
    fn ce(&self) -> Option<f64> {
        match self.ces.as_slice() {
            [c, rest @ ..] if rest.iter().all(|x| (x - c).abs() < 1e-9) => Some(*c),
            _ => None,
        }
    }
    fn total(&self) -> f64 {
        self.y.iter().sum()
    }
}

fn bad(side: &str, what: &str) -> Error {
    Error::new("err.spectrum.compare.input")
        .with("side", side)
        .with("what", what)
}

fn nums(v: &Value, key: &str, side: &str) -> Result<Vec<f64>> {
    v.get(key)
        .map(Value::arr)
        .ok_or_else(|| bad(side, key))?
        .iter()
        .map(|x| x.f64().ok_or_else(|| bad(side, key)))
        .collect()
}

fn side_of(v: &Value, label: &str) -> Result<Side> {
    let sp = v.get("spectrum").ok_or_else(|| bad(label, "spectrum"))?;
    if sp.get("mode").and_then(Value::str) != Some("bins") {
        return Err(bad(label, "spectrum (not binned: ask the route with rebin=nominal)"));
    }
    let (bin, mz, y) = (nums(sp, "bin", label)?, nums(sp, "mz", label)?, nums(sp, "y", label)?);
    if bin.len() != mz.len() || bin.len() != y.len() {
        return Err(bad(label, "bin, mz and y of different lengths"));
    }
    let keep: Vec<usize> = (0..y.len()).filter(|&i| y[i] > 0.0).collect();
    let pick = |a: &[f64]| keep.iter().map(|&i| a[i]).collect::<Vec<f64>>();
    let ces = match v.get("ce_values") {
        Some(_) => nums(v, "ce_values", label)?,
        None => Vec::new(),
    };
    Ok(Side {
        name: v.get("name").and_then(Value::str).unwrap_or(label).to_string(),
        precursor: v.get("precursor").and_then(Value::f64),
        query: v.get("query").and_then(Value::str).map(String::from),
        scans: sp.get("scans").and_then(Value::f64),
        ces,
        dropped: y.len() - keep.len(),
        bin: pick(&bin),
        mz: pick(&mz),
        y: pick(&y),
    })
}

/// The parameters: `tol` (Da between bin centres) and `ce_tol` (eV); the names of the ones that took their default are logged.
fn params(text: &str) -> Result<(f64, f64, Vec<&'static str>)> {
    let v = if text.trim().is_empty() { Value::Obj(Vec::new()) } else { parse(text.as_bytes())? };
    let mut out = [TOL_DA, CE_TOL];
    let names = ["tol", "ce_tol"];
    for (k, x) in v.entries() {
        let i = names
            .iter()
            .position(|n| n == k)
            .ok_or_else(|| Error::new("err.spectrum.compare.param").with("name", k.as_str()))?;
        out[i] = x
            .f64()
            .filter(|t| t.is_finite() && *t >= 0.0)
            .ok_or_else(|| Error::new("err.spectrum.compare.value").with("name", k.as_str()))?;
    }
    let default = (0..2).filter(|&i| v.get(names[i]).is_none()).map(|i| names[i]).collect();
    Ok((out[0], out[1], default))
}

fn side_json(o: &mut Out, s: &Side, matched: f64) {
    let total = s.total();
    o.raw("{\"name\":").str(&s.name).raw(",\"query\":");
    match &s.query {
        Some(q) => o.str(q),
        None => o.raw("null"),
    };
    o.raw(",\"scans\":").opt(s.scans).raw(",\"precursor\":").opt(s.precursor);
    o.raw(",\"ce\":").opt(s.ce()).raw(",\"ce_values\":").floats(&s.ces);
    o.raw(",\"n_bins\":").int(s.bin.len() as i64).raw(",\"dropped_bins\":").int(s.dropped as i64);
    o.raw(",\"total\":").num(round(total, 3)).raw(",\"matched\":").num(round(matched, 3));
    o.raw(",\"explained\":").num(round(if total > 0.0 { matched / total } else { 0.0 }, 6)).raw("}");
}

fn list(o: &mut Out, key: &str, rows: &[String]) {
    o.raw(",\"").raw(key).raw("\":[").raw(&rows.join(",")).raw("]");
}

/// The warnings: collision energies that differ, mixed or unknown, and precursors that differ.
fn warnings(a: &Side, b: &Side, ce_tol: f64) -> Vec<String> {
    let mut w = Vec::new();
    for (s, label) in [(a, "a"), (b, "b")] {
        if s.ces.is_empty() {
            let mut o = Out::default();
            o.raw("{\"key\":").str(CE_UNKNOWN).raw(",\"side\":").str(label).raw("}");
            w.push(o.0);
        } else if s.ce().is_none() {
            let mut o = Out::default();
            o.raw("{\"key\":").str(CE_MIXED).raw(",\"side\":").str(label).raw(",\"ce_values\":").floats(&s.ces).raw("}");
            w.push(o.0);
        }
    }
    if let (Some(x), Some(y)) = (a.ce(), b.ce()) {
        if (x - y).abs() > ce_tol {
            let mut o = Out::default();
            o.raw("{\"key\":").str(CE_DIFFER).raw(",\"a\":").num(x).raw(",\"b\":").num(y).raw("}");
            w.push(o.0);
        }
    }
    if let (Some(x), Some(y)) = (a.precursor, b.precursor) {
        if (x - y).abs() > PRECURSOR_DA {
            let mut o = Out::default();
            o.raw("{\"key\":").str(PRECURSOR_DIFFER).raw(",\"a\":").num(x).raw(",\"b\":").num(y).raw("}");
            w.push(o.0);
        }
    }
    w
}

/// The `spectrum-compare/1` text of the two sides, `{"a":{..},"b":{..}}` with `name`, `precursor`, `ce_values` and `spectrum` (the `rebin=` answer of
/// `/api/spectrum`) each, and the parameters `{"tol":DA,"ce_tol":EV}` (any may be missing).
pub fn compare_text(params_json: &str, sides_json: &str) -> Result<String> {
    let (tol, ce_tol, default) = params(params_json)?;
    let v = parse(sides_json.as_bytes())?;
    let get = |k: &str| v.get(k).ok_or_else(|| bad(k, "side"));
    let (a, b) = (side_of(get("a")?, "a")?, side_of(get("b")?, "b")?);
    for (s, l) in [(&a, "a"), (&b, "b")] {
        if s.bin.is_empty() {
            return Err(Error::new("err.spectrum.compare.empty").with("side", l));
        }
    }
    let pairs = match_peaks(&a.bin, &a.y, &b.bin, &b.y, tol);
    let cosine = cosine_from_pairs(&a.y, &b.y, &pairs);
    // the pairs in the order of the bins of A (the spectra come ascending)
    let mut partner: Vec<Option<usize>> = vec![None; a.bin.len()];
    for &(i, j) in &pairs {
        partner[i] = Some(j);
    }
    let (mut ma, mut mb) = (0.0, 0.0);
    let mut matched = Vec::new();
    for (i, j) in partner.iter().enumerate().filter_map(|(i, j)| j.map(|j| (i, j))) {
        (ma, mb) = (ma + a.y[i], mb + b.y[j]);
        let mut o = Out::default();
        o.raw("{\"bin\":").num(a.bin[i]).raw(",\"bin_b\":").num(b.bin[j]);
        o.raw(",\"mz_a\":").num(a.mz[i]).raw(",\"mz_b\":").num(b.mz[j]);
        o.raw(",\"a\":").num(a.y[i]).raw(",\"b\":").num(b.y[j]).raw("}");
        matched.push(o.0);
    }
    let only = |s: &Side, taken: &[bool]| {
        (0..s.bin.len())
            .filter(|&i| !taken[i])
            .map(|i| {
                let mut o = Out::default();
                o.raw("{\"bin\":").num(s.bin[i]).raw(",\"mz\":").num(s.mz[i]).raw(",\"y\":").num(s.y[i]).raw("}");
                o.0
            })
            .collect::<Vec<String>>()
    };
    let mut used_b = vec![false; b.bin.len()];
    pairs.iter().for_each(|&(_, j)| used_b[j] = true);
    let used_a: Vec<bool> = partner.iter().map(Option::is_some).collect();
    let (only_a, only_b) = (only(&a, &used_a), only(&b, &used_b));
    let mut o = Out::default();
    o.raw("{\"schema\":\"spectrum-compare/1\",\"bin\":\"nominal\",\"algorithm\":\"cosine of square-root intensities\"");
    o.raw(",\"params\":{\"tol\":").num(tol).raw(",\"ce_tol\":").num(ce_tol);
    o.raw(",\"default\":[").raw(&default.iter().map(|d| format!("\"{d}\"")).collect::<Vec<_>>().join(",")).raw("]}");
    o.raw(",\"cosine\":").num(round(cosine, 6)).raw(",\"n_matched\":").int(pairs.len() as i64);
    o.raw(",\"a\":");
    side_json(&mut o, &a, ma);
    o.raw(",\"b\":");
    side_json(&mut o, &b, mb);
    list(&mut o, "matched", &matched);
    list(&mut o, "only_a", &only_a);
    list(&mut o, "only_b", &only_b);
    list(&mut o, "warnings", &warnings(&a, &b, ce_tol));
    o.raw("}");
    Ok(o.0)
}

/// `compare_text` for the browser: the error as the `{error_key, params}` text of every `.wasm`.
pub fn compare_json(params_json: &str, sides_json: &str) -> std::result::Result<String, String> {
    compare_text(params_json, sides_json).map_err(|e| crate::flowmatch::error_json(&e))
}

#[cfg(test)]
#[path = "speccmp_tests.rs"]
mod tests;
