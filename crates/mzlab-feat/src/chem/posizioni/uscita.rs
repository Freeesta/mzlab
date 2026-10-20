//! `posiz/1`: the report as JSON, and the public entry point from a SMILES.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};

use super::confronto::{match_spectrum, Evidence, Peak, Pol, Report, Tol};
use super::prior::{prior_of, write_prior, Prior};
use super::siti::{sites, Site};
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

/// The `sites` array items; the atoms of a multi-atom site (`partner`, `third`) are listed after its kind.
fn sites_json(o: &mut Out, sites: &[Site]) {
    for (i, s) in sites.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"atom\":")
            .int(s.atom as i64)
            .raw(",\"symmetry\":")
            .int(s.class as i64)
            .raw(",\"kind\":")
            .str(s.kind);
        if let (true, Some(p)) = (super::siti::is_multi(s.kind), s.partner) {
            o.raw(",\"partner\":").int(p as i64);
            if let Some(t) = s.third {
                o.raw(",\"third\":").int(t as i64);
            }
        }
        o.raw("}");
    }
}

fn write(r: &Report, smiles: &str, transform: &str, prior: Option<&[Prior]>) -> String {
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
    sites_json(&mut o, &r.sites);
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
        .raw("}");
    if let Some(p) = prior {
        write_prior(&mut o, p);
    }
    o.raw("}");
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
    positions_prior_json(smiles, transform, peaks_json, tol, ppm, positive, "")
}

/// `positions_json` with the `reatt/1` of the same parent (`reatt_json`, empty = none): the report also carries `prior` (the f(−) of every site, «Prev.»),
/// identical otherwise. A change without a prior (`-CH2`, `+O-2H`) gets `"prior":[]`. Errors also `error.posiz.prior` (`reason`).
pub fn positions_prior_json(
    smiles: &str,
    transform: &str,
    peaks_json: &str,
    tol: f64,
    ppm: bool,
    positive: bool,
    reatt_json: &str,
) -> Result<String> {
    let mol = Mol::parse(smiles)?;
    report(
        &mol, smiles, transform, peaks_json, tol, ppm, positive, reatt_json,
    )
}

/// The report for an already built graph; `parent` is only the label written in the `parent` field.
#[allow(clippy::too_many_arguments)]
fn report(
    mol: &Mol,
    parent: &str,
    transform: &str,
    peaks_json: &str,
    tol: f64,
    ppm: bool,
    positive: bool,
    reatt_json: &str,
) -> Result<String> {
    if !(tol.is_finite() && tol > 0.0) {
        return Err(Error::new("error.posiz.tol"));
    }
    let peaks = parse_peaks(peaks_json)?;
    let all = sites(mol, transform)?;
    let prior = match reatt_json.trim() {
        "" => None,
        t => Some(prior_of(&all, mol.atoms.len(), t)?),
    };
    let tol = if ppm { Tol::Ppm(tol) } else { Tol::Da(tol) };
    let pol = if positive { Pol::Pos } else { Pol::Neg };
    let r = match_spectrum(mol, &all, &peaks, tol, pol)?;
    Ok(write(&r, parent, transform, prior.as_deref()))
}

/// `positions_prior_json` for a `mol/1` record instead of a SMILES (feature `record`): the graph is `Mol::from_record(rec, true)`, aromaticity perceived
/// from the Kekulé form, so a Kekulé parent finds its aromatic sites too. `parent` is the label of the `parent` field (the caller's SMILES).
#[cfg(feature = "record")]
#[allow(clippy::too_many_arguments)]
pub fn positions_record_json(
    rec: &mzlab_mol::Mol,
    parent: &str,
    transform: &str,
    peaks_json: &str,
    tol: f64,
    ppm: bool,
    positive: bool,
    reatt_json: &str,
) -> Result<String> {
    let mol = Mol::from_record(rec, true)?;
    report(
        &mol, parent, transform, peaks_json, tol, ppm, positive, reatt_json,
    )
}
