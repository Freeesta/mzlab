//! `ions/1`: the ions a drawn structure is expected to give in ESI+ and ESI−, from the adducts of the m/z calculator (`mzlab_core::adduct_table`)
//! applied to the formula of the canvas (`etich/1` atoms: isotopes, charge and radical marks kept). Each row is a candidate m/z to look for with
//! an XIC, never an identification. Doubly charged rows only where a second charge has somewhere to sit (two N for 2+, four O for 2−).

use mzlab_core::adduct_table;
use mzlab_core::elements::{self, ELECTRON};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};
use mzlab_core::nuclides::NUCLIDES;

use super::labels::{self, Counts, Key, Shape};
use crate::sortc::SortC;

/// Below this neutral mass a doubly charged ion of a small molecule falls under any usual scan range.
const Z2_MIN_MASS: f64 = 150.0;
/// The decimals of the m/z text when the request gives none, and the most it accepts.
const DECIMALS: (u64, u64) = (4, 6);
const PPM: f64 = 5.0;

/// Total count of an element over its isotopes.
fn of(n: &Counts, e: &str) -> u64 {
    n.iter().filter(|((x, _), _)| *x == e).map(|(_, c)| c).sum()
}

/// Whether a doubly charged row of polarity `z` is worth showing for this molecule.
fn z2_ok(s: &Shape, z: i8) -> bool {
    s.mass >= Z2_MIN_MASS
        && if z > 0 {
            of(&s.n, "N") >= 2
        } else {
            of(&s.n, "O") >= 4
        }
}

fn key_of(e: &str) -> Option<Key> {
    NUCLIDES.iter().find(|n| n.0 == e).map(|n| (n.0, 0))
}

/// Removes `c` atoms of element `e`: the natural ones first, then the labelled ones by mass number (a fully deuterated acid loses a D as [M−D]−).
fn take(n: &mut Counts, e: &str, mut c: u64) -> bool {
    if of(n, e) < c {
        return false;
    }
    let mut idx: Vec<usize> = (0..n.len()).filter(|&i| n[i].0 .0 == e).collect();
    idx.sortc_by_key(|&i| n[i].0 .1);
    for i in idx {
        let t = n[i].1.min(c);
        n[i].1 -= t;
        c -= t;
    }
    n.retain(|(_, c)| *c > 0);
    true
}

/// The ion of one catalogue row: `k` molecules, plus `add`, minus `sub`; `None` when the molecule has not what the row removes.
fn ion(s: &Shape, k: u8, add: &str, sub: &str) -> Result<Option<Counts>> {
    let mut n: Counts =
        s.n.iter()
            .map(|(key, c)| (*key, c * u64::from(k)))
            .collect();
    if !add.is_empty() {
        for (e, c) in elements::parse_formula(add)? {
            labels::add(
                &mut n,
                key_of(&e).ok_or_else(|| Error::new("error.ions.adduct"))?,
                c,
            );
        }
    }
    if !sub.is_empty() {
        for (e, c) in elements::parse_formula(sub)? {
            if !take(&mut n, &e, c) {
                return Ok(None);
            }
        }
    }
    Ok(Some(n))
}

fn mass_of(n: &Counts) -> f64 {
    mzlab_core::pysum::pysum(n.iter().map(|((e, a), c)| {
        elements::nuclide(e, (*a > 0).then_some(*a)).map_or(0.0, |(_, m)| m) * *c as f64
    }))
}

fn electrons(n: &Counts, z: i64) -> i64 {
    n.iter()
        .map(|((e, _), c)| labels::z_of(e) * *c as i64)
        .sum::<i64>()
        - z
}

struct Row {
    name: String,
    id: &'static str,
    k: u8,
    z: i64,
    common: bool,
    n: Counts,
}

/// The rows of one structure: the catalogue for a neutral drawing (net charge 0, zwitterions too), the ion itself for a charged one.
fn rows(s: &Shape) -> Result<Vec<Row>> {
    if s.q != 0 {
        let sign = if s.q > 0 { "+" } else { "-" };
        let a = s.q.unsigned_abs();
        let name = if a == 1 {
            format!("[M]{sign}")
        } else {
            format!("[M]{a}{sign}")
        };
        let id = if s.q > 0 { "mcat" } else { "man" };
        return Ok(vec![Row {
            name,
            id,
            k: 1,
            z: s.q,
            common: true,
            n: s.n.clone(),
        }]);
    }
    let mut out = Vec::new();
    for (name, id, k, add, sub, z, common) in adduct_table::catalog() {
        // [M]+ and [M]- are the rows of a structure drawn charged
        if add.is_empty() && sub.is_empty() || z.abs() == 2 && !z2_ok(s, z) {
            continue;
        }
        if let Some(n) = ion(s, k, add, sub)? {
            out.push(Row {
                name: name.to_string(),
                id,
                k,
                z: i64::from(z),
                common,
                n,
            });
        }
    }
    Ok(out)
}

fn structure_json(o: &mut Out, s: &Shape, dec: usize, ppm: f64) -> Result<()> {
    let f = labels::hill(&s.n);
    o.raw("{\"formula\":")
        .str(&f)
        .raw(",\"text\":")
        .str(&labels::with_charge(&f, s.q))
        .raw(",\"charge\":")
        .int(s.q)
        .raw(",\"mass\":")
        .num(round(s.mass, 6))
        .raw(",\"kind\":")
        .str(labels::kind(s))
        .raw(",\"rows\":[");
    for (i, r) in rows(s)?.iter().enumerate() {
        let mz = (mass_of(&r.n) - r.z as f64 * ELECTRON) / r.z.unsigned_abs() as f64;
        let w = mz * ppm * 1e-6;
        let rf = labels::hill(&r.n);
        o.raw(if i > 0 { ",{" } else { "{" })
            .raw("\"name\":")
            .str(&r.name)
            .raw(",\"id\":")
            .str(r.id)
            .raw(",\"pol\":")
            .int(r.z.signum())
            .raw(",\"k\":")
            .int(i64::from(r.k))
            .raw(",\"z\":")
            .int(r.z)
            .raw(",\"common\":")
            .raw(if r.common { "true" } else { "false" });
        o.raw(",\"formula\":")
            .str(&rf)
            .raw(",\"text\":")
            .str(&labels::with_charge(&rf, r.z))
            .raw(",\"mz\":")
            .num(round(mz, 6))
            .raw(",\"mzText\":")
            .str(&format!("{mz:.dec$}"))
            .raw(",\"lo\":")
            .num(round(mz - w, 6))
            .raw(",\"hi\":")
            .num(round(mz + w, 6))
            .raw(",\"odd\":")
            .raw(if electrons(&r.n, r.z) % 2 != 0 {
                "true"
            } else {
                "false"
            })
            .raw("}");
    }
    o.raw("]}");
    Ok(())
}

/// `ions/1` of a request `{"structures": [[{el, iso, h, q, r}, ...], ...], "decimals": 4, "ppm": 5}` (atoms as `etich/1` reads them; `decimals`
/// 0-6 for `mzText`, `ppm` the half window `lo`-`hi` of the XIC). A structure with an atom that is no element is `{"error": key, "label"}`.
pub fn ions_json(request: &str) -> Result<String> {
    ions_json_set(request, None, None)
}

/// `ions_json` with `decimals` and `ppm` given apart (the `--set` of `mzlab-cli mol-ions`); a given value wins over the request's.
pub fn ions_json_set(request: &str, decimals: Option<f64>, ppm: Option<f64>) -> Result<String> {
    let v = json::parse(request.as_bytes()).map_err(|_| Error::new("error.ions.request"))?;
    let dec = decimals
        .or_else(|| v.get("decimals").and_then(Value::f64))
        .map_or(DECIMALS.0, |d| d.clamp(0.0, DECIMALS.1 as f64) as u64) as usize;
    let ppm = ppm
        .or_else(|| v.get("ppm").and_then(Value::f64))
        .filter(|p| p.is_finite() && *p > 0.0 && *p <= 1000.0)
        .unwrap_or(PPM);
    let mut o = Out::default();
    o.raw("{\"format\":\"ions/1\",\"decimals\":")
        .int(dec as i64)
        .raw(",\"ppm\":")
        .num(ppm)
        .raw(",\"structures\":[");
    for (i, st) in v
        .get("structures")
        .map_or(&[][..], Value::arr)
        .iter()
        .enumerate()
    {
        o.raw(if i > 0 { "," } else { "" });
        let atoms: Vec<_> = st.arr().iter().map(labels::atom_in).collect();
        let shape = if atoms.is_empty() {
            Err(Error::new("error.ions.empty"))
        } else {
            labels::shape(&atoms)
        };
        match shape {
            Ok(s) => structure_json(&mut o, &s, dec, ppm)?,
            Err(e) => {
                let label = e
                    .params
                    .iter()
                    .find(|(k, _)| *k == "label")
                    .map_or("", |(_, v)| v.as_str());
                o.raw("{\"error\":")
                    .str(e.key)
                    .raw(",\"label\":")
                    .str(label)
                    .raw("}");
            }
        }
    }
    o.raw("]}");
    Ok(o.0)
}

#[cfg(test)]
#[path = "ions_tests.rs"]
mod tests;
