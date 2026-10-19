//! Reader of the V2000 connection table: fixed-column atom and bond lines and the `M  CHG`, `M  ISO`, `M  RAD` property lines.

use crate::codec::MolError;
use crate::elements::is_element;
use crate::molfile::{fail, Draft, DraftAtom, DraftBond, Wedge};

/// The text between columns `a` and `b` (0-based, `b` exclusive), trimmed; empty when the line is shorter.
fn field(line: &str, a: usize, b: usize) -> &str {
    line.get(a.min(line.len())..b.min(line.len()))
        .unwrap_or("")
        .trim()
}

fn int(line: &str, a: usize, b: usize) -> Result<i32, MolError> {
    let f = field(line, a, b);
    if f.is_empty() {
        return Ok(0);
    }
    f.parse()
        .map_err(|_| fail(format!("{f:?} is not an integer (line {line:?})")))
}

fn float(line: &str, a: usize, b: usize) -> Result<f64, MolError> {
    let f = field(line, a, b);
    if f.is_empty() {
        return Ok(0.0);
    }
    f.parse()
        .map_err(|_| fail(format!("{f:?} is not a number (line {line:?})")))
}

/// Element symbol and isotope of an atom-block symbol (`D` and `T` are hydrogen isotopes).
pub(crate) fn symbol(s: &str) -> Result<(String, u16), MolError> {
    match s {
        "D" => Ok(("H".into(), 2)),
        "T" => Ok(("H".into(), 3)),
        _ if is_element(s) => Ok((s.to_string(), 0)),
        _ => Err(fail(format!("atom symbol {s:?} is not an element"))),
    }
}

/// One atom line: the atom, the charge code of the legacy column and the unpaired electrons it implies.
fn atom_line(line: &str) -> Result<(DraftAtom, i8, u8), MolError> {
    let (el, iso) = symbol(field(line, 31, 34))?;
    let atom = DraftAtom {
        el,
        iso,
        chg: 0,
        rad: 0,
        valence: match int(line, 48, 51)? {
            0 => None,
            15 => Some(0),
            v => u8::try_from(v).ok(),
        },
        xyz: [
            float(line, 0, 10)?,
            float(line, 10, 20)?,
            float(line, 20, 30)?,
        ],
    };
    let (charge, rad) = match int(line, 36, 39)? {
        1 => (3, 0),
        2 => (2, 0),
        3 => (1, 0),
        4 => (0, 1),
        5 => (-1, 0),
        6 => (-2, 0),
        7 => (-3, 0),
        _ => (0, 0),
    };
    Ok((atom, charge, rad))
}

pub(crate) fn read_v2000(lines: &[&str]) -> Result<Draft, MolError> {
    let counts = lines.get(3).copied().unwrap_or("");
    let (na, nb) = (int(counts, 0, 3)? as usize, int(counts, 3, 6)? as usize);
    if lines.len() < 4 + na + nb {
        return Err(fail(
            "the atom and bond blocks are shorter than the counts line says",
        ));
    }
    let mut atoms = Vec::with_capacity(na);
    let mut legacy_charge = vec![0i8; na];
    let mut rad = vec![0u8; na];
    for (i, line) in lines[4..4 + na].iter().enumerate() {
        let (atom, charge, unpaired) = atom_line(line)?;
        atoms.push(atom);
        legacy_charge[i] = charge;
        rad[i] = unpaired;
    }
    let mut bonds = Vec::with_capacity(nb);
    for line in &lines[4 + na..4 + na + nb] {
        bonds.push(bond(line, na)?);
    }
    let mut seen_chg = false;
    for line in &lines[4 + na + nb..] {
        if line.starts_with("M  END") {
            break;
        }
        let Some(kind) = line.get(3..6) else { continue };
        let pairs = || -> Result<Vec<(usize, i32)>, MolError> { property_pairs(line, na) };
        match kind {
            "CHG" => {
                if !seen_chg {
                    legacy_charge.fill(0);
                    seen_chg = true;
                }
                for (a, v) in pairs()? {
                    legacy_charge[a] = v as i8;
                }
            }
            "ISO" => {
                for (a, v) in pairs()? {
                    atoms[a].iso = v as u16;
                }
            }
            "RAD" => {
                for (a, v) in pairs()? {
                    rad[a] = match v {
                        1 | 3 => 2,
                        2 => 1,
                        _ => 0,
                    };
                }
            }
            _ => {}
        }
    }
    for (i, a) in atoms.iter_mut().enumerate() {
        a.chg = legacy_charge[i];
        a.rad = rad[i];
    }
    Ok(Draft {
        atoms,
        bonds,
        collections: Vec::new(),
    })
}

fn bond(line: &str, na: usize) -> Result<DraftBond, MolError> {
    let (a, b) = (int(line, 0, 3)? as usize, int(line, 3, 6)? as usize);
    if a == 0 || b == 0 || a > na || b > na || a == b {
        return Err(fail(format!(
            "bond line {line:?} names atoms outside 1..{na}"
        )));
    }
    let order = match int(line, 6, 9)? {
        t @ 1..=3 => t as u8,
        4 => 0,
        t => return Err(fail(format!("bond type {t} (a query or unsupported bond)"))),
    };
    let wedge = match (order, int(line, 9, 12)?) {
        (1, 1) => Wedge::Up,
        (1, 6) => Wedge::Down,
        (1, 4) | (2, 3) => Wedge::Either,
        _ => Wedge::None,
    };
    Ok(DraftBond {
        a: a - 1,
        b: b - 1,
        order,
        wedge,
        arom: order == 0,
    })
}

/// `M  XXX  n  aaa vvv aaa vvv ...` as (0-based atom, value) pairs.
fn property_pairs(line: &str, na: usize) -> Result<Vec<(usize, i32)>, MolError> {
    let nums: Vec<i32> = line[6..]
        .split_whitespace()
        .map(|t| {
            t.parse()
                .map_err(|_| fail(format!("{t:?} in property line {line:?}")))
        })
        .collect::<Result<_, _>>()?;
    let n = nums.first().copied().unwrap_or(0) as usize;
    let mut out = Vec::new();
    for k in 0..n {
        let (Some(&a), Some(&v)) = (nums.get(1 + 2 * k), nums.get(2 + 2 * k)) else {
            return Err(fail(format!(
                "property line {line:?} is shorter than its count"
            )));
        };
        if a < 1 || a as usize > na {
            return Err(fail(format!("property line {line:?} names atom {a}")));
        }
        out.push((a as usize - 1, v));
    }
    Ok(out)
}
