//! Highlight lists of `mol draw` pictures: the JSON list or the compact text `0,1,0-1@red;5@blue` (atoms, bonds `a-b`, a colour name), checked
//! against the atom count. Colours are pale halos by name.

use super::draw_svg::Rgb;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

/// Pale halo colours by name.
const COLORS: [(&str, Rgb); 9] = [
    ("red", [244, 166, 166]),
    ("orange", [249, 201, 143]),
    ("yellow", [247, 229, 138]),
    ("green", [168, 223, 168]),
    ("cyan", [157, 223, 240]),
    ("blue", [169, 196, 245]),
    ("purple", [207, 176, 238]),
    ("pink", [245, 176, 216]),
    ("gray", [204, 204, 204]),
];

/// One highlight list: atoms, bonds (by atom pairs) and a colour.
pub struct Hl {
    pub atoms: Vec<u32>,
    pub bonds: Vec<(u32, u32)>,
    pub color: Rgb,
}

fn bad(name: &str, value: impl ToString) -> Error {
    Error::new("err.mol.draw.set")
        .with("name", name)
        .with("value", value.to_string())
}

fn color_of(name: &str) -> Result<Rgb> {
    COLORS
        .iter()
        .find(|c| c.0 == name)
        .map(|c| c.1)
        .ok_or_else(|| bad("highlight", name))
}

/// An atom number below `n`.
fn atom_in(x: Option<f64>, n: usize) -> Result<u32> {
    match x {
        Some(f) if f >= 0.0 && f.fract() == 0.0 && (f as usize) < n => Ok(f as u32),
        _ => Err(bad("highlight", "atom")),
    }
}

fn pair(a: u32, b: u32) -> Result<(u32, u32)> {
    if a == b {
        return Err(bad("highlight", "bond"));
    }
    Ok((a, b))
}

/// The compact form `0,1,0-1@red;5@blue` (atoms, bonds `a-b`, a colour name after `@`, lists apart with `;`).
fn compact(t: &str, n: usize) -> Result<Vec<Hl>> {
    let mut out = Vec::new();
    for list in t.split(';').filter(|l| !l.trim().is_empty()) {
        let (body, name) = list.split_once('@').unwrap_or((list, "yellow"));
        let num = |x: &str| atom_in(whole(x), n);
        let mut hl = Hl {
            atoms: Vec::new(),
            bonds: Vec::new(),
            color: color_of(name.trim())?,
        };
        for tok in body.split(',').map(str::trim).filter(|x| !x.is_empty()) {
            match tok.split_once('-') {
                Some((a, b)) => hl.bonds.push(pair(num(a)?, num(b)?)?),
                None => hl.atoms.push(num(tok)?),
            }
        }
        out.push(hl);
    }
    Ok(out)
}

/// A non-negative whole number written in digits.
fn whole(x: &str) -> Option<f64> {
    let t = x.trim();
    (!t.is_empty() && t.len() < 9 && t.bytes().all(|b| b.is_ascii_digit()))
        .then(|| t.bytes().fold(0.0, |v, b| v * 10.0 + f64::from(b - b'0')))
}

/// A decimal number written as `12` or `12.5` (the float parser of `std` costs the `.wasm` several KB).
pub(super) fn decimal(x: &str) -> Option<f64> {
    let (i, f) = x.trim().split_once('.').unwrap_or((x.trim(), ""));
    let frac = if f.is_empty() {
        Some(0.0)
    } else {
        whole(f).map(|v| v / 10f64.powi(f.len() as i32))
    };
    Some(whole(i)? + frac?)
}

/// `[{"atoms": [i], "bonds": [[a, b]], "color": name}]` (or the compact text of `compact`), numbers as in the scene, below `n` atoms.
pub fn highlights(v: &Value, n: usize) -> Result<Vec<Hl>> {
    if let Some(t) = v.str() {
        return compact(t, n);
    }
    let mut out = Vec::new();
    for l in v.arr() {
        let mut hl = Hl {
            atoms: Vec::new(),
            bonds: Vec::new(),
            color: color_of(l.get("color").and_then(Value::str).unwrap_or("yellow"))?,
        };
        for a in l.get("atoms").map_or(&[][..], Value::arr) {
            hl.atoms.push(atom_in(a.f64(), n)?);
        }
        for b in l.get("bonds").map_or(&[][..], Value::arr) {
            match b.arr() {
                [a, c] => hl
                    .bonds
                    .push(pair(atom_in(a.f64(), n)?, atom_in(c.f64(), n)?)?),
                _ => return Err(bad("highlight", "bond")),
            }
        }
        out.push(hl);
    }
    Ok(out)
}
