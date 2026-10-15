//! `posiz/1`: the report as JSON, and the public entry point from a SMILES.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};

use super::confronto::{match_spectrum, Evidence, Peak, Pol, Report, Tol};
use super::siti::sites;
use crate::chem::grafo::Mol;

/// Peaks as `[[mz, intensity], ..]` or `[{"mz", "i"}, ..]` (`"intensity"` is accepted for `"i"`).
fn parse_peaks(text: &str) -> Result<Vec<Peak>> {
    let bad = || Error::new("error.posiz.peaks");
    let v = json::parse(text.as_bytes()).map_err(|_| bad())?;
    let one = |p: &Value| -> Option<Peak> {
        let (mz, inten) = match p {
            Value::Arr(_) => (p.at(0)?.f64()?, p.at(1)?.f64()?),
            _ => (
                p.get("mz")?.f64()?,
                p.get("i").or_else(|| p.get("intensity"))?.f64()?,
            ),
        };
        (mz.is_finite() && inten.is_finite() && mz > 0.0 && inten >= 0.0)
            .then_some(Peak { mz, inten })
    };
    match &v {
        Value::Arr(a) => a.iter().map(|p| one(p).ok_or_else(bad)).collect(),
        _ => Err(bad()),
    }
}

fn evidences(o: &mut Out, v: &[Evidence]) {
    o.raw("[");
    for (i, e) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(round(e.mz, 4))
            .raw(",\"frag_formula\":")
            .str(&e.formula)
            .raw(",\"side_atoms\":");
        ints(o, &e.atoms);
        if let Some(m) = e.iso_mz {
            o.raw(",\"iso_mz\":").num(round(m, 4));
        }
        o.raw("}");
    }
    o.raw("]");
}

fn ints(o: &mut Out, v: &[usize]) {
    o.raw("[");
    for (i, a) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(*a as i64);
    }
    o.raw("]");
}

fn write(r: &Report, smiles: &str, transform: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":\"posiz\",\"version\":1,\"parent\":")
        .str(smiles)
        .raw(",\"transform\":")
        .str(transform)
        .raw(",\"tp_formula\":")
        .str(&r.tp_formula)
        .raw(",\"tp_mz\":")
        .num(round(r.tp_mz, 4))
        .raw(",\"pol\":")
        .str(r.pol.sign())
        .raw(",\"tol\":")
        .num(r.tol.value())
        .raw(",\"tol_unit\":")
        .str(r.tol.unit())
        .raw(",\"sites\":[");
    for (i, s) in r.sites.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"atom\":")
            .int(s.atom as i64)
            .raw(",\"symmetry\":")
            .int(s.class as i64)
            .raw(",\"kind\":")
            .str(s.kind)
            .raw("}");
    }
    o.raw("],\"classes\":[");
    for (i, c) in r.classes.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .int(c.id as i64)
            .raw(",\"atoms\":");
        ints(&mut o, &c.atoms);
        o.raw(",\"kind\":")
            .str(&c.kind)
            .raw(",\"status\":")
            .str(c.status)
            .raw(",\"support\":");
        evidences(&mut o, &c.support);
        o.raw(",\"against\":");
        evidences(&mut o, &c.against);
        o.raw("}");
    }
    o.raw("],\"ox_type\":");
    match r.ox_type {
        Some((w, m)) => o
            .raw("{\"h2o_loss\":")
            .raw(if w { "true" } else { "false" })
            .raw(",\"minus_o\":")
            .raw(if m { "true" } else { "false" })
            .raw("}"),
        None => o.raw("null"),
    };
    o.raw(",\"unexplained\":");
    o.rounded(&r.unexplained, 4);
    o.raw(",\"suggest\":[");
    for (i, s) in r.suggest.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(s);
    }
    o.raw("],\"proposal\":{\"sublevel\":")
        .str(r.sublevel)
        .raw(",\"why\":")
        .str(r.why)
        .raw("}}");
    o.0
}

/// `posiz/1` for a parent SMILES, a transformation (`+O`, `-CH2`, `+O-2H`, `+Cl-H`, `+Br-H`) and the peaks of the product's MS2 (`[[mz, intensity], ..]`
/// or `[{"mz","i"}, ..]`). `tol` is in Da when `ppm` is false, in ppm otherwise; `positive` selects [M+H]+ or [M-H]-. Errors are `error.posiz.*`
/// (`transform`, `noSites`, `formula`, `peaks`) or those of `Mol::parse`.
pub fn positions_json(
    smiles: &str,
    transform: &str,
    peaks_json: &str,
    tol: f64,
    ppm: bool,
    positive: bool,
) -> Result<String> {
    if !(tol.is_finite() && tol > 0.0) {
        return Err(Error::new("error.posiz.tol"));
    }
    let mol = Mol::parse(smiles)?;
    let peaks = parse_peaks(peaks_json)?;
    let all = sites(&mol, transform)?;
    let tol = if ppm { Tol::Ppm(tol) } else { Tol::Da(tol) };
    let pol = if positive { Pol::Pos } else { Pol::Neg };
    let r = match_spectrum(&mol, &all, &peaks, tol, pol)?;
    Ok(write(&r, smiles, transform))
}
