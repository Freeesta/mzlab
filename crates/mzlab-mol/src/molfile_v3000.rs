//! Reader of the V3000 connection table (`M  V30` lines, `-` continuations): atoms with `CHG`/`RAD`/`MASS`, bonds with `CFG`, and the
//! enhanced-stereo collections `MDLV30/STERAC`, `STEREL`, `STEABS`. Other blocks (S-groups, queries) are skipped; query atoms are refused.

use crate::codec::MolError;
use crate::model::GroupKind;
use crate::molfile::{fail, Draft, DraftAtom, DraftBond, Wedge};
use crate::molfile_v2000::symbol;
use std::collections::HashMap;

/// Whitespace-separated tokens, keeping `( ... )` groups whole.
fn tokens(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth) = (Vec::new(), String::new(), 0);
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if c.is_whitespace() && depth == 0 {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The `V30` lines with continuations joined, without the `M  V30 ` prefix.
fn logical_lines(lines: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut open = String::new();
    for l in lines.iter().filter(|l| l.starts_with("M  V30")) {
        let body = l[6..].trim();
        match body.strip_suffix('-') {
            Some(head) => {
                open.push_str(head);
                open.push(' ');
            }
            None => {
                open.push_str(body);
                out.push(std::mem::take(&mut open));
            }
        }
    }
    out
}

fn number<T: std::str::FromStr>(t: &str) -> Result<T, MolError> {
    t.parse()
        .map_err(|_| fail(format!("{t:?} is not a number")))
}

/// `KEY=value` tokens of a line.
fn props(toks: &[String]) -> HashMap<&str, &str> {
    toks.iter().filter_map(|t| t.split_once('=')).collect()
}

pub(crate) fn read_v3000(lines: &[&str]) -> Result<Draft, MolError> {
    let mut ids: HashMap<usize, usize> = HashMap::new();
    let mut atoms = Vec::new();
    let mut raw_bonds: Vec<Vec<String>> = Vec::new();
    let mut collections = Vec::new();
    let mut block = "";
    for l in logical_lines(lines) {
        let t = tokens(&l);
        match (t.first().map(String::as_str), t.get(1).map(String::as_str)) {
            (Some("BEGIN"), Some(b)) => {
                block = match b {
                    "ATOM" => "ATOM",
                    "BOND" => "BOND",
                    "COLLECTION" => "COLLECTION",
                    _ => "",
                }
            }
            (Some("END"), _) => block = "",
            _ => match block {
                "ATOM" => {
                    ids.insert(number(&t[0])?, atoms.len());
                    atoms.push(atom(&t)?);
                }
                "BOND" => raw_bonds.push(t),
                "COLLECTION" => collections.extend(collection(&t, &ids)?),
                _ => {}
            },
        }
    }
    if atoms.is_empty() {
        return Err(fail("the V3000 file has no atoms"));
    }
    let bonds = raw_bonds
        .iter()
        .map(|t| bond(t, &ids))
        .collect::<Result<_, _>>()?;
    Ok(Draft {
        atoms,
        bonds,
        collections,
    })
}

fn atom(t: &[String]) -> Result<DraftAtom, MolError> {
    if t.len() < 5 {
        return Err(fail(format!("atom line {t:?} is too short")));
    }
    let (el, iso) = symbol(&t[1])?;
    let p = props(&t[5..]);
    let rad = match p.get("RAD").copied() {
        Some("1" | "3") => 2,
        Some("2") => 1,
        _ => 0,
    };
    Ok(DraftAtom {
        el,
        iso: match p.get("MASS") {
            Some(m) => number::<f64>(m)?.round() as u16,
            None => iso,
        },
        chg: p.get("CHG").map_or(Ok(0), |v| number(v))?,
        rad,
        valence: match p.get("VAL").map(|v| number::<i32>(v)) {
            Some(Ok(-1)) => Some(0),
            Some(Ok(v)) if (1..=14).contains(&v) => Some(v as u8),
            _ => None,
        },
        xyz: [number(&t[2])?, number(&t[3])?, number(&t[4])?],
    })
}

fn bond(t: &[String], ids: &HashMap<usize, usize>) -> Result<DraftBond, MolError> {
    if t.len() < 4 {
        return Err(fail(format!("bond line {t:?} is too short")));
    }
    let find = |s: &String| -> Result<usize, MolError> {
        ids.get(&number::<usize>(s)?)
            .copied()
            .ok_or_else(|| fail(format!("bond names unknown atom {s}")))
    };
    let (a, b) = (find(&t[2])?, find(&t[3])?);
    let order = match number::<u8>(&t[1])? {
        o @ 1..=3 => o,
        4 => 0,
        o => return Err(fail(format!("bond type {o} (a query or unsupported bond)"))),
    };
    let wedge = match (order, props(&t[4..]).get("CFG").copied()) {
        (1, Some("1")) => Wedge::Up,
        (1, Some("3")) => Wedge::Down,
        (1 | 2, Some("2")) => Wedge::Either,
        _ => Wedge::None,
    };
    Ok(DraftBond {
        a,
        b,
        order,
        wedge,
        arom: order == 0,
    })
}

/// `MDLV30/STERAC1 ATOMS=(2 1 3)` and its relatives; other collections give nothing.
fn collection(
    t: &[String],
    ids: &HashMap<usize, usize>,
) -> Result<Option<(GroupKind, Vec<usize>)>, MolError> {
    let Some(name) = t.first().and_then(|n| n.strip_prefix("MDLV30/")) else {
        return Ok(None);
    };
    let kind = if name.starts_with("STERAC") {
        GroupKind::And
    } else if name.starts_with("STEREL") {
        GroupKind::Or
    } else if name.starts_with("STEABS") {
        GroupKind::Abs
    } else {
        return Ok(None);
    };
    let Some(list) = props(&t[1..])
        .get("ATOMS")
        .map(|v| v.trim_matches(|c| c == '(' || c == ')'))
    else {
        return Ok(None);
    };
    let mut atoms = Vec::new();
    for s in list.split_whitespace().skip(1) {
        let id: usize = number(s)?;
        atoms.push(
            *ids.get(&id)
                .ok_or_else(|| fail(format!("collection names unknown atom {id}")))?,
        );
    }
    Ok(Some((kind, atoms)))
}
