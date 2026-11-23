//! `descriptors/1`: the one function behind the drawing values of mzDraw, the molkit `.wasm`, the app (`feat/grafo` kind `descriptors`) and
//! `mzlab-cli mol measure`. The molfile (V2000 or V3000) is read once by `mzlab_mol::Mol::from_molfile`; `Mol::descriptors` gives logP, MR,
//! TPSA, donors, acceptors, rotatable bonds and the conventional average weight, and the same atoms go to the `analysis/1` writer. Theory for
//! the drawn structure: nothing is fitted to data.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};
use mzlab_mol::Mol;

use super::analysis::analysis_out;
use super::labels::AtomIn;

/// Atoms above this are a library, not a drawing (the limit of `mol draw` and the export): the ring test of the rotors grows with atoms x bonds.
const MAX_ATOMS: usize = 2000;

/// The `M  RAD` code of a count of unpaired electrons, as `label/1` reads it (1 doublet, 2 triplet).
fn rad_code(unpaired: u8) -> f64 {
    match unpaired {
        0 => 0.0,
        1 => 2.0,
        _ => 3.0,
    }
}

/// `descriptors/1` of a request `{"molfile"}`: `logp`, `mr` (Wildman-Crippen), `tpsa` (Ertl, N and O), `hbd`, `hba`, `rotors`, `mw` (conventional
/// atomic weights, null when an element has none) and `analysis` (the `analysis/1` of the whole drawing). Decimals: 4. Over 2000 atoms: `error.descr.tooBig`.
pub fn descriptors_json(request: &str) -> Result<String> {
    let v = json::parse(request.as_bytes()).map_err(|_| Error::new("error.descr.request"))?;
    let mf = v
        .get("molfile")
        .and_then(Value::str)
        .ok_or_else(|| Error::new("error.descr.request"))?;
    let m =
        Mol::from_molfile(mf).map_err(|e| Error::new("error.descr.molfile").with("detail", e))?;
    if m.atoms.len() > MAX_ATOMS {
        return Err(Error::new("error.descr.tooBig").with("max", MAX_ATOMS));
    }
    let d = m.descriptors();
    let atoms: Vec<AtomIn> = m
        .atoms
        .iter()
        .map(|a| AtomIn {
            el: &a.el,
            iso: f64::from(a.iso),
            h: f64::from(a.h),
            q: f64::from(a.chg),
            r: rad_code(a.rad),
        })
        .collect();
    let mut o = Out::default();
    o.raw("{\"schema\":\"descriptors/1\",\"logp\":")
        .num(round(d.logp, 4))
        .raw(",\"mr\":")
        .num(round(d.mr, 4))
        .raw(",\"tpsa\":")
        .num(round(d.tpsa, 4));
    o.raw(",\"hbd\":")
        .int(i64::from(d.hbd))
        .raw(",\"hba\":")
        .int(i64::from(d.hba))
        .raw(",\"rotors\":")
        .int(i64::from(d.rotors));
    o.raw(",\"mw\":")
        .opt(d.mw.map(|w| round(w, 4)))
        .raw(",\"analysis\":");
    analysis_out(&mut o, &atoms)?;
    o.raw("}");
    Ok(o.0)
}
