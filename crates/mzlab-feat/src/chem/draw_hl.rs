//! Highlight lists of `mol draw` pictures: the JSON list or the compact text `0,1,0-1@red;5@blue` (atoms, bonds `a-b`, a colour name), checked
//! against the structure (atoms below the atom count, bonds that exist, at most one item per atom and bond in all). Colours are pale halos by
//! name.

use super::draw_svg::Rgb;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;
use mzlab_mol::Mol;

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

/// What a list may name: the atom count, the bonds of the structure, and how many items are still allowed (each atom and each bond once, so a
/// hostile list cannot make the picture cost more than the structure itself).
struct Allowed {
    n: usize,
    bonds: Vec<(u32, u32)>,
    left: usize,
}

impl Allowed {
    fn new(m: &Mol) -> Self {
        let bonds: Vec<(u32, u32)> = m.written_bonds().iter().map(|b| (b.0, b.1)).collect();
        Allowed {
            n: m.atoms.len(),
            left: m.atoms.len() + bonds.len(),
            bonds,
        }
    }

    fn take(&mut self) -> Result<()> {
        if self.left == 0 {
            return Err(bad("highlight", "too many items"));
        }
        self.left -= 1;
        Ok(())
    }

    /// An atom number below the atom count.
    fn atom(&mut self, x: Option<f64>) -> Result<u32> {
        self.take()?;
        match x {
            Some(f) if f >= 0.0 && f.fract() == 0.0 && (f as usize) < self.n => Ok(f as u32),
            _ => Err(bad("highlight", "atom")),
        }
    }

    /// A bond of the structure, in either order.
    fn bond(&mut self, a: Option<f64>, b: Option<f64>) -> Result<(u32, u32)> {
        let ends = (self.atom(a)?, self.atom(b)?);
        if self
            .bonds
            .iter()
            .any(|&p| p == ends || p == (ends.1, ends.0))
        {
            Ok(ends)
        } else {
            Err(bad("highlight", "bond"))
        }
    }
}

/// The compact form `0,1,0-1@red;5@blue` (atoms, bonds `a-b`, a colour name after `@`, lists apart with `;`).
fn compact(t: &str, ok: &mut Allowed) -> Result<Vec<Hl>> {
    let mut out = Vec::new();
    for list in t.split(';').filter(|l| !l.trim().is_empty()) {
        let (body, name) = list.split_once('@').unwrap_or((list, "yellow"));
        let mut hl = Hl {
            atoms: Vec::new(),
            bonds: Vec::new(),
            color: color_of(name.trim())?,
        };
        for tok in body.split(',').map(str::trim).filter(|x| !x.is_empty()) {
            match tok.split_once('-') {
                Some((a, b)) => hl.bonds.push(ok.bond(whole(a), whole(b))?),
                None => hl.atoms.push(ok.atom(whole(tok))?),
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
        // `whole` keeps `f` under 9 digits, so the divisor is exact
        whole(f).map(|v| v / f.bytes().fold(1.0, |p, _| p * 10.0))
    };
    Some(whole(i)? + frac?)
}

/// `[{"atoms": [i], "bonds": [[a, b]], "color": name}]` (or the compact text of `compact`) for the structure `m`: atoms below its count, bonds
/// that exist in it, one item per atom and bond at most.
pub fn highlights(v: &Value, m: &Mol) -> Result<Vec<Hl>> {
    let mut ok = Allowed::new(m);
    if let Some(t) = v.str() {
        return compact(t, &mut ok);
    }
    let mut out = Vec::new();
    for l in v.arr() {
        let mut hl = Hl {
            atoms: Vec::new(),
            bonds: Vec::new(),
            color: color_of(l.get("color").and_then(Value::str).unwrap_or("yellow"))?,
        };
        for a in l.get("atoms").map_or(&[][..], Value::arr) {
            hl.atoms.push(ok.atom(a.f64())?);
        }
        for b in l.get("bonds").map_or(&[][..], Value::arr) {
            match b.arr() {
                [a, c] => hl.bonds.push(ok.bond(a.f64(), c.f64())?),
                _ => return Err(bad("highlight", "bond")),
            }
        }
        out.push(hl);
    }
    Ok(out)
}
