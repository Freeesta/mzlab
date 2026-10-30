//! The text of `annot.level@1` (`annot/1`): the inputs it reads, one row per query and how a row is written. Takes bytes and returns the text; `flow run` and the browser call it.

use super::{
    assess_with, extra, formula_counts, from_hits, scans_of, Assessment, Evidence, GateResult,
    Params,
};
use mzlab_core::error::Result;
use mzlab_core::json::{self, round, Out, Value};

/// What `annot.level` reads: the `candidates` port (candidates/1 and screen/1 files), the formulas, and the optional standards CSV and MGF (the RT of each query).
#[derive(Default)]
pub struct Inputs<'a> {
    pub hits: &'a [&'a [u8]],
    pub formulas: &'a [&'a [u8]],
    pub standards: Option<&'a [u8]>,
    pub spectra: Option<&'a [u8]>,
}

/// `annot.level` with the default parameters and only the library hits and formulas.
pub fn level_text(hits: &[&[u8]], formulas: &[&[u8]]) -> Result<String> {
    let inputs = Inputs {
        hits,
        formulas,
        ..Inputs::default()
    };
    level_text_with(&Params::default(), &inputs)
}

/// `level_text_with` from the page's texts: `params_json` (the resolved parameters), the `candidates/1` and `screen/1` files, and the formulas, standards and MGF texts (an empty slice is a port not wired).
pub fn level_text_json(
    params_json: &str,
    (hits, screen): (&[u8], &[u8]),
    (formulas, standards, spectra): (&[u8], &[u8], &[u8]),
) -> Result<String> {
    fn given(b: &[u8]) -> Option<&[u8]> {
        (!b.is_empty()).then_some(b)
    }
    let params = crate::flow_ms2::json_params(params_json)?;
    let mut files = vec![hits];
    files.extend(given(screen));
    let inputs = Inputs {
        hits: &files,
        formulas: &[formulas],
        standards: given(standards),
        spectra: given(spectra),
    };
    level_text_with(&Params::from_params(&params), &inputs)
}

/// `annot.level`: one row per query of the hits and per feature or spectrum of the formulas or the suspects, with the level, why and the gates of the best hit.
pub fn level_text_with(p: &Params, inp: &Inputs) -> Result<String> {
    let (mut libs, mut screens) = (Vec::new(), Vec::new());
    for f in inp.hits {
        let doc = json::parse(f)?;
        if doc.get("format").and_then(Value::str) == Some("screen/1") {
            screens.push(doc);
        } else {
            libs.push(doc);
        }
    }
    let mut rows = from_hits(&libs);
    let counts = formula_counts(inp.formulas)?;
    let suspects = extra::suspects_of(&screens);
    let standards = inp.standards.map(extra::read_standards).transpose()?;
    let rts = inp.spectra.map(extra::read_rts).unwrap_or_default();
    for id in counts
        .iter()
        .map(|(id, _)| id)
        .chain(suspects.iter().map(|(id, _)| id))
    {
        if !rows.iter().any(|(q, _)| q == id) {
            rows.push((id.clone(), Evidence::default()));
        }
    }
    let snapshot = rows.clone();
    let over = p.overrides();
    let mut out = Out::default();
    out.raw("{\"format\":\"annot/1\",\"step\":\"annot.level@1\",\"algorithm\":\"cascade\",\"profile\":\"mzlab\",\"rows\":[");
    for (k, (id, e)) in rows.iter_mut().enumerate() {
        e.n_scans = scans_of(&snapshot, e);
        e.formula_unique = counts.iter().any(|(q, n)| q == id && *n == 1);
        e.suspect = suspects
            .iter()
            .find(|(q, _)| q == id)
            .map(|(_, s)| s.clone());
        if let Some(st) = &standards {
            let rt = rts.iter().find(|(q, _)| q == id).map(|(_, r)| *r);
            e.std = extra::std_of(&e.cands, st, rt);
        }
        write_row(&mut out, k, id, &assess_with(e, "mzlab", &over)?);
    }
    out.raw("]}");
    Ok(out.0)
}

/// The row of one query: the level, why, the evidence named and the gates.
fn write_row(out: &mut Out, k: usize, id: &str, a: &Assessment) {
    out.raw(if k > 0 { "," } else { "" })
        .raw("{\"id\":")
        .str(id)
        .raw(",\"level\":")
        .str(&a.level)
        .raw(",\"why\":")
        .str(a.why);
    let named = a.std.as_ref().filter(|_| a.why == "standard");
    if let Some(s) = named {
        out.raw(",\"name\":")
            .str(&s.name)
            .raw(",\"entry\":")
            .str(&s.ik);
    } else if let Some(b) = &a.best {
        out.raw(",\"name\":")
            .str(&b.name)
            .raw(",\"entry\":")
            .str(&b.ik);
    }
    if let Some(n) = &a.analog {
        out.raw(",\"analog\":")
            .str(&n.name)
            .raw(",\"analog_delta\":")
            .num(round(n.delta, 4));
    }
    if let Some(s) = &a.suspect {
        out.raw(",\"suspect\":")
            .str(&s.name)
            .raw(",\"suspect_fragments\":[");
        for (i, m) in s.fragments.iter().enumerate() {
            out.raw(if i > 0 { "," } else { "" }).num(round(*m, 5));
        }
        out.raw("]");
    }
    if let Some(s) = &a.std {
        out.raw(",\"standard\":{\"name\":")
            .str(&s.name)
            .raw(",\"rt\":")
            .num(round(s.rt, 4));
        if let Some(mz) = s.mz {
            out.raw(",\"mz\":").num(round(mz, 5));
        }
        if !s.adduct.is_empty() {
            out.raw(",\"adduct\":").str(&s.adduct);
        }
        out.raw(",\"ran\":")
            .raw(if s.drt.is_some() { "true" } else { "false" })
            .raw(",\"gates\":[");
        write_gates(out, &a.std_gates);
        out.raw("]}");
    }
    out.raw(",\"gates\":[");
    write_gates(out, &a.gates);
    out.raw("]}");
}

fn write_gates(out: &mut Out, gates: &[GateResult]) {
    for (i, g) in gates.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .str(&g.id)
            .raw(",\"ok\":")
            .raw(if g.ok { "true" } else { "false" })
            .raw(",\"value\":")
            .opt(g.value.filter(|v| v.is_finite()).map(|v| round(v, 4)))
            .raw(",\"threshold\":")
            .num(round(g.threshold, 4))
            .raw("}");
    }
}
