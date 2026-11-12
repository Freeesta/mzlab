//! `sdf/1`: the drawing as a plain molfile or SDF, written by `mzlab-mol` from its `mol/1` record. A `.mol` keeps the whole canvas in one
//! record; an `.sdf` has one record per connected structure with the data items `MZLAB_FORMULA`, `MZLAB_EXACT_MASS` (the etich/1 numbers),
//! `MZLAB_ION` and `MZLAB_MZ` (the chosen adduct of a neutral structure, or the drawn ion) and `MZLAB_HASH` (`mol1:` id of the canonical
//! record). Same bytes in the molkit `.wasm`, the app (`feat/grafo`, `kind: "sdf"`) and `mzlab-cli mol-export`.

use mzlab_core::elements::{self, ELECTRON, NO_ADDUCT};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};
use mzlab_mol::{fixed, Mol};

/// Above this many atoms the export refuses (a drawing, not a library).
const MAX_ATOMS: usize = 2000;

/// The text of the file for a request `{"molfile", "format": "mol"|"sdf", "ion", "name"}`.
pub fn sdf_json(request: &str) -> Result<String> {
    let bad = || Error::new("error.sdf.request");
    let v = json::parse(request.as_bytes()).map_err(|_| bad())?;
    let text = |k: &str| v.get(k).and_then(Value::str);
    let mf = text("molfile").ok_or_else(bad)?;
    export(
        mf,
        text("format").unwrap_or("sdf"),
        text("ion").unwrap_or(NO_ADDUCT),
        text("name").unwrap_or(""),
    )
}

/// The `.mol` or `.sdf` text of a molfile; `ion` is an adduct of `elements::adducts` or `none`, `name` the title line of every record.
pub fn export(mf: &str, format: &str, ion: &str, name: &str) -> Result<String> {
    if format != "mol" && format != "sdf" {
        return Err(Error::new("error.sdf.request"));
    }
    if ion != NO_ADDUCT && adduct_shift(ion).is_none() {
        return Err(Error::new("error.sdf.ion").with("ion", ion));
    }
    let m = Mol::from_molfile(mf)
        .map_err(|e| Error::new("error.sdf.molfile").with("message", e.to_string()))?;
    if m.atoms.len() > MAX_ATOMS {
        return Err(Error::new("error.sdf.tooBig").with("max", MAX_ATOMS));
    }
    if format == "mol" {
        return Ok(m.to_molfile(name));
    }
    let mut out = String::new();
    for p in m.pieces() {
        out.push_str(&p.to_molfile(name));
        items(&mut out, &p, ion);
        out.push_str("$$$$\n");
    }
    Ok(out)
}

/// The shift of an adduct of the table `elements::adducts`, from the nuclide table (the one this `.wasm` already carries; within 1e-6 Da of
/// `elements::adducts`, whose Na differs by 4e-7, below the five decimals written; see tests/sdf.rs).
pub fn adduct_shift(ion: &str) -> Option<f64> {
    let m = |e: &str| nuclide_mass(e, None);
    let (h, e) = (m("H"), ELECTRON);
    Some(match ion {
        "[M+H]+" => h - e,
        "[M+NH4]+" => m("N") + 4.0 * h - e,
        "[M+Na]+" => m("Na") - e,
        "[M+K]+" => m("K") - e,
        "[M-H]-" => -h + e,
        "[M+Cl]-" => m("Cl") + e,
        "[M+HCOO]-" => m("C") + h + 2.0 * m("O") + e,
        _ => return None,
    })
}

/// Formula (Hill order of etich/1: C, H, then alphabetical, isotopes after their element as `[13C]`, `D`, `T`; H3N written NH3), charge and
/// monoisotopic mass of a structure.
pub fn formula(m: &Mol) -> (String, i64, f64) {
    type Key = (u8, &'static str, u16);
    fn add(n: &mut Vec<(Key, u32)>, k: Key, c: u32) {
        match n.iter_mut().find(|(x, _)| *x == k) {
            Some((_, v)) => *v += c,
            None => n.push((k, c)),
        }
    }
    let mut n: Vec<(Key, u32)> = Vec::new();
    let (mut q, mut mass) = (0i32, 0.0);
    for a in &m.atoms {
        let el = static_el(&a.el);
        let rank = match el {
            "C" => 0,
            "H" => 1,
            _ => 2,
        };
        add(&mut n, (rank, el, a.iso), 1);
        if a.h > 0 {
            add(&mut n, (1, "H", 0), u32::from(a.h));
        }
        q += i32::from(a.chg);
        let iso = (a.iso > 0).then_some(a.iso);
        mass += nuclide_mass(&a.el, iso) + f64::from(a.h) * nuclide_mass("H", None);
    }
    for i in 1..n.len() {
        let mut j = i;
        while j > 0 && n[j - 1].0 > n[j].0 {
            n.swap(j - 1, j);
            j -= 1;
        }
    }
    let mut f = String::new();
    for ((_, e, a), c) in &n {
        match (*e, *a) {
            (e, 0) => f.push_str(e),
            ("H", 2) => f.push('D'),
            ("H", 3) => f.push('T'),
            (e, a) => f.push_str(&format!("[{}{e}]", u32::from(a))),
        }
        if *c > 1 {
            f.push_str(&c.to_string());
        }
    }
    if f == "H3N" {
        f = "NH3".into();
    }
    let sign = if q > 0 { "+" } else { "-" };
    let text = match q.abs() {
        0 => f,
        1 => f + sign,
        a => format!("[{f}]{a}{sign}"),
    };
    (text, i64::from(q), mass)
}

fn nuclide_mass(el: &str, iso: Option<u16>) -> f64 {
    elements::nuclide(el, iso).map_or(0.0, |(_, x)| x)
}

/// The `&'static str` of an element symbol, from the nuclide table.
fn static_el(el: &str) -> &'static str {
    mzlab_core::nuclides::NUCLIDES
        .iter()
        .find(|x| x.0 == el)
        .map_or("", |x| x.0)
}

/// The data items of one structure.
fn items(out: &mut String, m: &Mol, ion: &str) {
    let (f, q, mass) = formula(m);
    let mut item = |k: &str, v: &str| {
        out.push_str("> <");
        out.push_str(k);
        out.push_str(">\n");
        out.push_str(v);
        out.push_str("\n\n");
    };
    item("MZLAB_FORMULA", &f);
    item("MZLAB_EXACT_MASS", &fixed(mass, 5));
    let mz = if q != 0 {
        Some((
            "drawn",
            (mass - q as f64 * ELECTRON) / q.unsigned_abs() as f64,
        ))
    } else {
        adduct_shift(ion).map(|d| (ion, mass + d))
    };
    if let Some((name, z)) = mz {
        item("MZLAB_ION", name);
        item("MZLAB_MZ", &fixed(z, 5));
    }
    item("MZLAB_HASH", &m.can());
}
