//! Reader of the V2000 connection table: fixed-column atom and bond lines and the `M  CHG`, `M  ISO`, `M  RAD` property lines; of the
//! S-groups only the `MRV_IMPLICIT_H` data group (`M  SAL`, `M  SDT`, `M  SED`).

use crate::codec::MolError;
use crate::elements::is_element;
use crate::molfile::{fail, implicit_h, Draft, DraftAtom, DraftBond, Wedge};
use crate::sortm::SortM;
use std::collections::BTreeMap;

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

/// Element symbol, isotope and query flag of an atom-block symbol (`D` and `T` are hydrogen isotopes; any other non-element is a query atom).
pub(crate) fn symbol(s: &str) -> (String, u16, bool) {
    match s {
        "D" => ("H".into(), 2, false),
        "T" => ("H".into(), 3, false),
        _ => (s.to_string(), 0, !is_element(s)),
    }
}

/// One atom line: the atom, the charge code of the legacy column and the unpaired electrons it implies.
fn atom_line(line: &str) -> Result<(DraftAtom, i8, u8), MolError> {
    let (el, iso, query) = symbol(field(line, 31, 34));
    let atom = DraftAtom {
        el,
        iso,
        query,
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

/// A charge as the record keeps it; one outside -15 to 15 (what a molfile can state) is a damaged file, not a value to cut.
pub(crate) fn charge(v: i32, atom: usize) -> Result<i8, MolError> {
    if (-15..=15).contains(&v) {
        Ok(v as i8)
    } else {
        Err(fail(format!(
            "the charge {v} of atom {} is not -15 to 15",
            atom + 1
        )))
    }
}

pub(crate) fn read_v2000(lines: &[&str]) -> Result<Draft, MolError> {
    let counts = lines.get(3).copied().unwrap_or("");
    // a negative count would wrap into a huge one
    let count = |a, b| {
        usize::try_from(int(counts, a, b)?)
            .map_err(|_| fail(format!("the counts line {counts:?} has a negative count")))
    };
    let (na, nb) = (count(0, 3)?, count(3, 6)?);
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
        // a property line starts with `M  `: any other line is not one
        let Some(kind) = line.strip_prefix("M  ").and_then(|l| l.get(..3)) else {
            continue;
        };
        let pairs = || -> Result<Vec<(usize, i32)>, MolError> { property_pairs(line, na) };
        match kind {
            "CHG" => {
                if !seen_chg {
                    legacy_charge.fill(0);
                    seen_chg = true;
                }
                for (a, v) in pairs()? {
                    legacy_charge[a] = charge(v, a)?;
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
        stated_h: stated_hydrogens(&lines[4 + na + nb..], na)?,
    })
}

/// The (0-based atom, hydrogens) of the `MRV_IMPLICIT_H` data groups among the property `lines` of a V2000 file of `na` atoms
/// (`M  SAL g n atoms`, `M  SDT g field`, `M  SED g data`, up to `M  END`), sorted: an atom named twice has its larger count last.
pub fn stated_hydrogens(lines: &[&str], na: usize) -> Result<Vec<(usize, u32)>, MolError> {
    // data groups by number: (atoms, field name, data)
    // ordered, so that of two damaged groups the same one is named in every run
    let mut data: BTreeMap<&str, (Vec<usize>, &str, &str)> = BTreeMap::new();
    for line in lines.iter().take_while(|l| !l.starts_with("M  END")) {
        let kind = line.starts_with("M  ").then(|| line.get(3..6)).flatten();
        let Some(kind @ ("SAL" | "SDT" | "SED")) = kind else {
            continue;
        };
        let mut t = line[6..].split_whitespace();
        let Some(group) = t.next() else { continue };
        let g = data.entry(group).or_default();
        match kind {
            "SAL" => g.0.extend(
                t.skip(1)
                    .filter_map(|a| a.parse::<usize>().ok())
                    .filter(|a| (1..=na).contains(a))
                    .map(|a| a - 1),
            ),
            "SDT" => g.1 = t.next().unwrap_or(""),
            _ => g.2 = t.next().unwrap_or(""),
        }
    }
    let mut out = Vec::new();
    for (list, field, value) in data.values() {
        if let Some(h) = implicit_h(field, value)? {
            out.extend(list.iter().map(|&a| (a, h)));
        }
    }
    out.sortm();
    Ok(out)
}

fn bond(line: &str, na: usize) -> Result<DraftBond, MolError> {
    let (a, b) = (int(line, 0, 3)? as usize, int(line, 3, 6)? as usize);
    if a == 0 || b == 0 || a > na || b > na {
        return Err(fail(format!(
            "bond line {line:?} names atoms outside 1..{na}"
        )));
    }
    if a == b {
        return Err(fail(format!("bond line {line:?} joins atom {a} to itself")));
    }
    let (order, query) = match int(line, 6, 9)? {
        t @ 1..=3 => (t as u8, 0),
        4 => (0, 0),
        t => (1, t.max(1)),
    };
    let mark = int(line, 9, 12)?;
    let wedge = match (order, mark) {
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
        query,
        multi_wedge: order != 1 && matches!(mark, 1 | 6),
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
    let n = usize::try_from(nums.first().copied().unwrap_or(0))
        .map_err(|_| fail(format!("property line {line:?} has a negative count")))?;
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
