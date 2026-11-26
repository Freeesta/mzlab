//! The ion-scale method values read from a `method/1` document (`docs/agenti/method.md`): `Facts::from_method` for the engine, `read_json` for the three
//! callers (CLI, app, site) that pass the values to `/api/rawxic` as route parameters. Only a field the document names is read; a value the method does
//! not give stays absent, and a method that gives several different values gives none (the user states it with `--set`).

use crate::error::Result;
use crate::ionscale::{Facts, ScanKind};
use crate::json::{parse, Out, Value};

/// A number as `Out::num` writes it (the shortest text that reads back as the same value; already linked, unlike `fmt_g`).
fn num(x: f64) -> String {
    let mut o = Out::default();
    o.num(x);
    o.0
}

/// One method value looked for: the field, the value used (`None`: the method gives several) and the experiments it comes from.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub field: &'static str,
    pub value: Option<f64>,
    pub from: String,
}

/// Experiments named in `Read::from` before the rest is only counted.
const NAMED: usize = 4;

/// The dwell of each experiment that has transitions: (period, experiment, dwell values), in document order.
fn dwells(doc: &Value) -> Vec<(usize, usize, Vec<f64>)> {
    let periods = doc
        .get("ms")
        .and_then(|m| m.get("periods"))
        .map(Value::arr)
        .unwrap_or_default();
    let mut out = Vec::new();
    for (p, period) in periods.iter().enumerate() {
        let exps = period
            .get("experiments")
            .map(Value::arr)
            .unwrap_or_default();
        for (e, exp) in exps.iter().enumerate() {
            let v: Vec<f64> = exp
                .get("transitions")
                .map(Value::arr)
                .unwrap_or_default()
                .iter()
                .filter_map(|t| t.get("dwell_ms").and_then(Value::f64))
                .filter(|x| x.is_finite() && *x > 0.0)
                .collect();
            if !v.is_empty() {
                out.push((p, e, v));
            }
        }
    }
    out
}

/// `periods[P].experiments[E]` of each of the first `NAMED`, then the count of the others.
fn places(of: &[&(usize, usize, Vec<f64>)]) -> String {
    let mut s: Vec<String> = of
        .iter()
        .take(NAMED)
        .map(|(p, e, _)| format!("ms.periods[{p}].experiments[{e}]"))
        .collect();
    if of.len() > NAMED {
        s.push(format!("{} more", of.len() - NAMED));
    }
    s.join(", ")
}

/// The dwell of a SIM or MRM scan: `transitions[].dwell_ms` of every experiment that has them. Equal everywhere: that value; different: none.
fn dwell(doc: &Value) -> Option<Read> {
    let all = dwells(doc);
    let mut distinct: Vec<f64> = Vec::new();
    for x in all.iter().flat_map(|a| a.2.iter()) {
        if !distinct.contains(x) {
            distinct.push(*x);
        }
    }
    let first = *distinct.first()?;
    if distinct.len() == 1 {
        let of: Vec<_> = all.iter().collect();
        return Some(Read {
            field: "dwell_ms",
            value: Some(first),
            from: format!("transitions[].dwell_ms of {}", places(&of)),
        });
    }
    let mut parts: Vec<String> = Vec::new();
    for d in &distinct {
        let of: Vec<_> = all.iter().filter(|a| a.2.contains(d)).collect();
        parts.push(format!("{} ms in {}", num(*d), places(&of)));
    }
    Some(Read {
        field: "dwell_ms",
        value: None,
        from: format!(
            "transitions[].dwell_ms differs ({}): not used",
            parts.join("; ")
        ),
    })
}

impl Facts {
    /// The method values of `doc` that `kind` needs (`None`: those of every kind) and where each came from. The labels read are those `method/1` names
    /// (`dwell_ms` of a transition); a fill time, a Q1 scan time, step or range has none yet, so those fields stay absent.
    pub fn from_method(doc: &Value, kind: Option<ScanKind>) -> (Facts, Vec<Read>) {
        let mut f = Facts::default();
        let mut read = Vec::new();
        if let Some(r) = dwell(doc).filter(|_| kind.is_none_or(|k| k == ScanKind::Sim)) {
            f.dwell_ms = r.value;
            read.push(r);
        }
        (f, read)
    }

    /// The route parameters of the values set (`dwell_ms=5`), in the order of `ionscale::Facts`.
    pub fn query(&self) -> String {
        let mut q: Vec<String> = Vec::new();
        for (k, v) in [
            ("fill_ms", self.fill_ms),
            ("dwell_ms", self.dwell_ms),
            ("scan_s", self.scan_s),
            ("step_da", self.step_da),
            ("mass_lo", self.mass_lo),
            ("mass_hi", self.mass_hi),
        ] {
            if let Some(x) = v {
                q.push(format!("{k}={}", num(x)));
            }
        }
        q.join("&")
    }
}

/// `{"query":"dwell_ms=5","read":[{"field","value","from"}]}` for the `method/1` text `method` (empty: the file carries none) and the scan type named
/// (`""`: not named, every kind): the bytes the CLI, the app and the site share. An unknown scan type is `err.raw.xic.param`; a text that is not JSON
/// gives the parser's error.
pub fn read_json(method: &str, scan_type: &str) -> Result<String> {
    let kind = match scan_type {
        "" => None,
        s => Some(ScanKind::parse(s).ok_or_else(|| {
            crate::error::Error::new("err.raw.xic.param")
                .with("name", "scan_type".to_string())
                .with("value", s.to_string())
        })?),
    };
    let (facts, read) = if method.is_empty() {
        (Facts::default(), Vec::new())
    } else {
        let doc = parse(method.as_bytes())?;
        Facts::from_method(&doc, kind)
    };
    let mut o = Out::default();
    o.raw("{\"query\":").str(&facts.query()).raw(",\"read\":[");
    for (i, r) in read.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.raw("{\"field\":")
            .str(r.field)
            .raw(",\"value\":")
            .opt(r.value);
        o.raw(",\"from\":").str(&r.from).raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}

#[cfg(test)]
#[path = "ionfacts_tests.rs"]
mod tests;
