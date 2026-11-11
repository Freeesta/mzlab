//! A substructure query: a `mol/1` record plus one test per atom and per bond (`docs/agenti/mol.md`, «Substructure»). It comes from a
//! SMILES (every atom and bond tests its own element and order) or from a V2000/V3000 molfile with the query features a drawing editor
//! writes: generic atoms `A AH Q QH X XH *`, atom lists (`L` + `M  ALS`, V3000 `[C,N]` and `NOT[C,N]`) and query bonds of type 4 to 8.
//! Query features the matcher cannot honour (ring counts, substitution counts, bond topology, R-groups) are refused, never ignored.

use crate::codec::MolError;
use crate::elements::is_element;
use crate::model::Mol;
use std::collections::HashMap;

/// What a query atom accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtomTest {
    /// The element of the record atom; a non-zero charge, isotope or radical count must be equal too.
    Element,
    /// One of these elements (`M  ALS ... F`, `[C,N]`).
    List(Vec<String>),
    /// Any element but these (`M  ALS ... T`, `NOT[C,N]`).
    NotList(Vec<String>),
    /// `A` (any atom but hydrogen) or, with `h`, `AH` and `*` (any atom).
    Any { h: bool },
    /// `Q` (not carbon, not hydrogen) or, with `h`, `QH` (not carbon).
    Hetero { h: bool },
    /// `X` (F, Cl, Br, I, At) or, with `h`, `XH` (also hydrogen).
    Halogen { h: bool },
}

/// What a query bond accepts. Aromaticity is perceived on both sides (`Mol::aromatic_bonds`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BondTest {
    /// The same class as the record bond: single, double, triple (outside aromatic rings) or aromatic.
    Order,
    /// Type 4: aromatic.
    Aromatic,
    /// Type 5: single or double, outside aromatic rings.
    SingleOrDouble,
    /// Type 6: single or aromatic.
    SingleOrAromatic,
    /// Type 7: double or aromatic.
    DoubleOrAromatic,
    /// Type 8: any bond.
    Any,
}

/// A query: `atoms[i]` tests atom `i` of `mol`, `bonds[k]` tests `mol.bonds[k]`.
#[derive(Clone, Debug, PartialEq)]
pub struct SubQuery {
    pub mol: Mol,
    pub atoms: Vec<AtomTest>,
    pub bonds: Vec<BondTest>,
}

impl SubQuery {
    /// A query in which every atom and bond tests itself.
    pub fn exact(mol: Mol) -> SubQuery {
        SubQuery {
            atoms: vec![AtomTest::Element; mol.atoms.len()],
            bonds: vec![BondTest::Order; mol.bonds.len()],
            mol,
        }
    }

    /// A molfile with query features (a text with `M  END`) or a SMILES.
    pub fn from_text(text: &str) -> Result<SubQuery, MolError> {
        if text.contains("M  END") {
            SubQuery::from_molfile(text)
        } else {
            Ok(SubQuery::exact(Mol::from_smiles(text.trim())?))
        }
    }

    /// A V2000 or V3000 molfile; query atoms are read with a stand-in element (the first of a list, else C) and query bonds as single bonds.
    pub fn from_molfile(text: &str) -> Result<SubQuery, MolError> {
        let text = text.replace("\r\n", "\n");
        let record = text.split("\n$$$$").next().unwrap_or("");
        let mut lines: Vec<String> = record.lines().map(str::to_string).collect();
        let v3000 = lines.get(3).is_some_and(|l| l.contains("V3000"));
        let (atoms, pairs) = if v3000 {
            rewrite_v3000(&mut lines)?
        } else {
            rewrite_v2000(&mut lines)?
        };
        let mol = Mol::from_molfile(&lines.join("\n"))?;
        let bonds = mol
            .bonds
            .iter()
            .map(|b| {
                pairs
                    .get(&(b.a as usize, b.b as usize))
                    .copied()
                    .unwrap_or(BondTest::Order)
            })
            .collect();
        Ok(SubQuery { mol, atoms, bonds })
    }
}

/// Reads a target: a molfile (a text with `M  END`) or a SMILES. A `mol/1` JSON record is not read here, so the `.wasm` `sub` does not
/// carry the JSON reader of the record.
pub fn read_target(text: &str) -> Result<Mol, MolError> {
    if text.contains("M  END") {
        Mol::from_molfile(text)
    } else {
        Mol::from_smiles(text.trim())
    }
}

fn refuse(what: &str) -> MolError {
    MolError::Query(format!("the query feature {what} is not supported"))
}

/// The test of a generic symbol, with the element that stands in for it; None for an element.
fn generic(sym: &str) -> Result<Option<AtomTest>, MolError> {
    Ok(Some(match sym {
        "A" => AtomTest::Any { h: false },
        "AH" | "*" => AtomTest::Any { h: true },
        "Q" => AtomTest::Hetero { h: false },
        "QH" => AtomTest::Hetero { h: true },
        "X" => AtomTest::Halogen { h: false },
        "XH" => AtomTest::Halogen { h: true },
        "L" => AtomTest::List(Vec::new()),
        "D" | "T" => return Ok(None),
        s if is_element(s) => return Ok(None),
        s => return Err(refuse(&format!("atom {s:?}"))),
    }))
}

fn bond_test(t: i32) -> Result<Option<BondTest>, MolError> {
    Ok(match t {
        1..=3 => None,
        4 => Some(BondTest::Aromatic),
        5 => Some(BondTest::SingleOrDouble),
        6 => Some(BondTest::SingleOrAromatic),
        7 => Some(BondTest::DoubleOrAromatic),
        8 => Some(BondTest::Any),
        t => return Err(MolError::Molfile(format!("bond type {t}"))),
    })
}

/// The elements of a list, checked.
fn elements<'a>(syms: impl Iterator<Item = &'a str>) -> Result<Vec<String>, MolError> {
    let els: Vec<String> = syms.map(str::to_string).collect();
    match els.iter().find(|e| !is_element(e)) {
        Some(e) => Err(refuse(&format!("list element {e:?}"))),
        None if els.is_empty() => Err(refuse("empty atom list")),
        None => Ok(els),
    }
}

/// The stand-in element of a query atom: the first of its list, else carbon.
fn stand_in(t: &AtomTest) -> &str {
    match t {
        AtomTest::List(e) | AtomTest::NotList(e) if !e.is_empty() && e[0] != "H" => &e[0],
        _ => "C",
    }
}

type Rewritten = (Vec<AtomTest>, HashMap<(usize, usize), BondTest>);

fn int(line: &str, a: usize, b: usize) -> i32 {
    line.get(a..b.min(line.len()))
        .unwrap_or("")
        .trim()
        .parse()
        .unwrap_or(0)
}

fn rewrite_v2000(lines: &mut [String]) -> Result<Rewritten, MolError> {
    let counts = lines.get(3).cloned().unwrap_or_default();
    let (na, nb) = (int(&counts, 0, 3) as usize, int(&counts, 3, 6) as usize);
    if lines.len() < 4 + na + nb {
        return Err(MolError::Molfile(
            "the blocks are shorter than the counts line says".into(),
        ));
    }
    let mut atoms = Vec::with_capacity(na);
    for line in &lines[4..4 + na] {
        let sym = line.get(31..34.min(line.len())).unwrap_or("").trim();
        atoms.push(generic(sym)?.unwrap_or(AtomTest::Element));
    }
    let mut pairs = HashMap::new();
    for line in lines[4 + na..4 + na + nb].iter_mut() {
        if int(line, 15, 18) != 0 {
            return Err(refuse("bond topology (ring or chain)"));
        }
        if let Some(t) = bond_test(int(line, 6, 9))? {
            let (a, b) = (int(line, 0, 3) as usize, int(line, 3, 6) as usize);
            pairs.insert((a.min(b).saturating_sub(1), a.max(b).saturating_sub(1)), t);
            line.replace_range(6..9, "  1");
        }
    }
    for line in lines[4 + na + nb..].iter() {
        match line.get(0..6) {
            Some("M  ALS") => {
                let f: Vec<&str> = line[6..].split_whitespace().collect();
                let a = f.first().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
                if a == 0 || a > na || f.len() < 4 {
                    return Err(MolError::Molfile(format!("atom list line {line:?}")));
                }
                let els = elements(f[3..].iter().copied())?;
                atoms[a - 1] = if f[2] == "T" {
                    AtomTest::NotList(els)
                } else {
                    AtomTest::List(els)
                };
            }
            Some("M  RBC") => return Err(refuse("ring bond count")),
            Some("M  SUB") => return Err(refuse("substitution count")),
            Some("M  UNS") => return Err(refuse("unsaturated atom")),
            Some("M  RGP") => return Err(refuse("R-group")),
            _ => {}
        }
    }
    for (line, t) in lines[4..4 + na].iter_mut().zip(&atoms) {
        if *t != AtomTest::Element {
            if *t == AtomTest::List(Vec::new()) {
                return Err(refuse("atom list without its M  ALS line"));
            }
            if line.len() < 34 {
                return Err(MolError::Molfile(format!(
                    "atom line {line:?} is too short"
                )));
            }
            line.replace_range(31..34, &format!("{:<3}", stand_in(t)));
        }
    }
    Ok((atoms, pairs))
}

/// The test of a V3000 symbol: `[C,N]`, `NOT[C,N]` or a generic symbol.
fn v3000_atom(sym: &str) -> Result<Option<AtomTest>, MolError> {
    let (not, rest) = match sym.strip_prefix("NOT") {
        Some(r) => (true, r),
        None => (false, sym),
    };
    if let Some(list) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let els = elements(list.split(','))?;
        return Ok(Some(if not {
            AtomTest::NotList(els)
        } else {
            AtomTest::List(els)
        }));
    }
    match generic(sym)? {
        Some(AtomTest::List(_)) => Err(refuse("atom list without its elements")),
        t => Ok(t),
    }
}

fn rewrite_v3000(lines: &mut [String]) -> Result<Rewritten, MolError> {
    let (mut atoms, mut pairs, mut ids) = (Vec::new(), HashMap::new(), HashMap::new());
    let mut block = "";
    for line in lines.iter_mut() {
        let Some(body) = line.strip_prefix("M  V30 ") else {
            continue;
        };
        let mut t: Vec<String> = body.split_whitespace().map(str::to_string).collect();
        match t.as_slice() {
            [b, w, ..] if b == "BEGIN" => {
                block = if w == "ATOM" {
                    "atom"
                } else if w == "BOND" {
                    "bond"
                } else {
                    ""
                };
                continue;
            }
            [e, ..] if e == "END" => {
                block = "";
                continue;
            }
            _ => {}
        }
        if t.iter().any(|p| {
            ["RBCNT=", "SUBST=", "UNSAT=", "TOPO=", "RGROUPS="]
                .iter()
                .any(|k| p.starts_with(k))
        }) {
            return Err(refuse(
                t.iter()
                    .find(|p| p.contains('='))
                    .map_or("", |s| s.as_str()),
            ));
        }
        if block == "atom" && t.len() > 1 {
            ids.insert(t[0].clone(), atoms.len());
            let test = v3000_atom(&t[1])?.unwrap_or(AtomTest::Element);
            if test != AtomTest::Element {
                t[1] = stand_in(&test).to_string();
            }
            atoms.push(test);
        } else if block == "bond" && t.len() > 3 {
            if let Some(test) = bond_test(t[1].parse().unwrap_or(0))? {
                let (Some(&a), Some(&b)) = (ids.get(&t[2]), ids.get(&t[3])) else {
                    return Err(MolError::Molfile(format!(
                        "bond line {body:?} names an unknown atom"
                    )));
                };
                pairs.insert((a.min(b), a.max(b)), test);
                t[1] = "1".into();
            }
        } else {
            continue;
        }
        *line = format!("M  V30 {}", t.join(" "));
    }
    Ok((atoms, pairs))
}
