//! The sources of hypotheses: (a) the rules, (a') cleavage formulas, (b) the suspect index, (c) families already found, (d) imported lists.

use std::collections::BTreeMap;

use mzlab_core::elements::{self, ion_mz, mass_of};
use mzlab_core::error::{Error, Result};

use super::rules::{Counts, Dir, Rule, Rules};
use super::{
    counts_of, ion, to_formula, HypoParams, Hypothesis, SRC_CLEAVAGE, SRC_FAMILY, SRC_IMPORTED,
    SRC_RULE, SRC_SUSPECT,
};
use crate::suspects::{Suspects, SRC_TRANSPUBCHEM};

/// A family base found in a file: its ion m/z, RT (min) and charge sign (+1 for [M+H]+, -1 for [M-H]-).
#[derive(Debug, Clone, Copy)]
pub struct FamilyBase {
    pub mz: f64,
    pub rt: f32,
    pub polarity: i8,
}

/// A compound of a list of the user or imported: a label and a molecular formula (a SMILES is turned into a formula by the caller), or only a
/// monoisotopic neutral mass when the formula is empty.
#[derive(Debug, Clone)]
pub struct Imported {
    pub name: String,
    pub formula: String,
    pub mass: Option<f64>,
}

const D13C: f64 = 1.003355;
const DPLUS2: f64 = 2.00671;

struct Path {
    steps: u8,
    label: String,
    level: u8,
    dir: Dir,
    alts: Vec<String>,
}

fn join_dir(a: Dir, b: Dir) -> Dir {
    match (a, b) {
        (x, y) if x == y => x,
        (Dir::Free, y) => y,
        (x, Dir::Free) => x,
        _ => Dir::Free,
    }
}

/// Every multiset of `k` indices below `n` (combinations with replacement), in lexicographic order.
fn combos(n: usize, k: usize, start: usize, cur: &mut Vec<usize>, f: &mut dyn FnMut(&[usize])) {
    if cur.len() == k {
        return f(cur);
    }
    for i in start..n {
        cur.push(i);
        combos(n, k, i, cur, f);
        cur.pop();
    }
}

fn path_of(rules: &[&Rule], combo: &[usize]) -> Option<(Counts, Path)> {
    let mut net = Counts::new();
    let mut uses = vec![0u8; rules.len()];
    for &i in combo {
        uses[i] += 1;
        if uses[i] > rules[i].max_steps {
            return None;
        }
        for (e, n) in &rules[i].delta {
            *net.entry(e.clone()).or_insert(0) += n;
        }
    }
    net.retain(|_, n| *n != 0);
    let label = combo
        .iter()
        .map(|&i| rules[i].id.as_str())
        .collect::<Vec<_>>()
        .join("+");
    let dir = combo
        .iter()
        .skip(1)
        .fold(rules[combo[0]].dir, |d, &i| join_dir(d, rules[i].dir));
    let level = combo.iter().map(|&i| rules[i].level).max().unwrap_or(0);
    Some((
        net,
        Path {
            steps: combo.len() as u8,
            label,
            level,
            dir,
            alts: Vec::new(),
        },
    ))
}

/// (a) the parent times the rules, up to `max_steps` reactions, one hypothesis per net change of formula (fewest steps, then first label).
pub fn from_rules(
    rules: &Rules,
    parent: &Counts,
    groups: Option<&[String]>,
    p: &HypoParams,
) -> Vec<Hypothesis> {
    let ok: Vec<&Rule> = rules
        .rules
        .iter()
        .filter(|r| rules.applies(r, parent, groups))
        .collect();
    let mut best: BTreeMap<Counts, Path> = BTreeMap::new();
    for k in 1..=(p.max_steps as usize).min(4) {
        combos(ok.len(), k, 0, &mut Vec::new(), &mut |c| {
            let Some((net, path)) = path_of(&ok, c) else {
                return;
            };
            if net.is_empty() {
                return;
            }
            match best.get_mut(&net) {
                None => {
                    best.insert(net, path);
                }
                Some(b)
                    if path.steps == b.steps
                        && path.label != b.label
                        && !b.alts.contains(&path.label) =>
                {
                    b.alts.push(path.label)
                }
                _ => {}
            }
        });
    }
    let mut out = Vec::new();
    for (net, path) in best {
        let mut f = parent.clone();
        for (e, n) in &net {
            *f.entry(e.clone()).or_insert(0) += n;
        }
        let Some(formula) = to_formula(&f) else {
            continue;
        };
        out.push(hypothesis(
            &formula, p, path.label, SRC_RULE, path.steps, path.level, path.dir, path.alts,
        ));
    }
    out.retain(|h| h.mz >= p.min_mz);
    out
}

#[allow(clippy::too_many_arguments)]
fn hypothesis(
    f: &elements::Formula,
    p: &HypoParams,
    label: String,
    sources: u8,
    steps: u8,
    level: u8,
    dir: Dir,
    alternatives: Vec<String>,
) -> Hypothesis {
    let neutral = elements::mass(f);
    let (adduct, mz) = ion(neutral, p.polarity);
    Hypothesis {
        id: 0,
        sources,
        label,
        formula: elements::fmt(f),
        neutral,
        mz,
        adduct,
        polarity: p.polarity,
        steps,
        level,
        dir,
        rt_ref: None,
        alternatives,
    }
}

/// (a') products of the cleavage of the parent, given as formulas: each must have no more atoms of an element than the parent (hydrogens within 3).
pub fn from_fragments(
    frags: &[String],
    parent: &Counts,
    p: &HypoParams,
) -> Result<Vec<Hypothesis>> {
    let mut out = Vec::new();
    for t in frags {
        let f = elements::parse_formula(t)?;
        let c = counts_of(&f);
        let inside = c
            .iter()
            .all(|(e, n)| *n <= parent.get(e).copied().unwrap_or(0) + if e == "H" { 3 } else { 0 });
        if !inside || c == *parent {
            return Err(Error::new("error.hypo.fragment").with("formula", t));
        }
        out.push(hypothesis(
            &f,
            p,
            "rottura".to_string(),
            SRC_CLEAVAGE,
            1,
            4,
            Dir::Free,
            Vec::new(),
        ));
    }
    out.retain(|h| h.mz >= p.min_mz);
    Ok(out)
}

/// (a'') one reaction of the rules on a cleavage product: the second step of a path that starts with the cleavage (the cleavage is the first). Only
/// the rules that need no groups apply (the groups known are those of the parent, not of the fragment).
pub fn from_fragment_products(
    rules: &Rules,
    frags: &[String],
    parent: &Counts,
    p: &HypoParams,
) -> Result<Vec<Hypothesis>> {
    if p.max_steps < 2 {
        return Ok(Vec::new());
    }
    let one = HypoParams {
        max_steps: 1,
        ..p.clone()
    };
    let mut out = Vec::new();
    for h in from_fragments(frags, parent, p)? {
        let counts = counts_of(&elements::parse_formula(&h.formula)?);
        for mut r in from_rules(rules, &counts, None, &one) {
            r.label = format!("{}+{}", h.label, r.label);
            r.sources = SRC_CLEAVAGE;
            r.steps = 2;
            r.level = r.level.max(h.level);
            out.push(r);
        }
    }
    Ok(out)
}

/// A suspect can derive from the parent: no element the parent lacks, none in larger number (oxygens up to +3, hydrogens up to +4).
fn derivable(f: &Counts, parent: &Counts) -> bool {
    f.iter().all(|(e, n)| {
        let room = match e.as_str() {
            "O" => 3,
            "H" => 4,
            _ => 0,
        };
        *n <= parent.get(e).copied().unwrap_or(0) + room
    })
}

/// (b) hits of the suspect index that derive from the parent and have a reaction in Transformations in PubChem; the most cited first, at most
/// `max_suspects` (the others are counted). Returns (hypotheses, number cut).
pub fn from_suspects(s: &Suspects, parent: &Counts, p: &HypoParams) -> (Vec<Hypothesis>, u32) {
    let pm = elements::mass(&to_formula(parent).unwrap_or_default());
    let (lo, hi) = (
        p.min_mz,
        pm + 3.0 * mass_of("O").unwrap_or(16.0) + 4.0 * mass_of("H").unwrap_or(1.0),
    );
    let mid = (lo + hi) / 2.0;
    let mut hits: Vec<_> = s
        .search(mid, (hi - lo) / 2.0 / mid * 1e6)
        .iter()
        .filter(|h| h.sources & SRC_TRANSPUBCHEM != 0)
        .filter_map(|h| {
            let f = elements::parse_formula(s.formula(h)).ok()?;
            derivable(&counts_of(&f), parent).then_some((h, f))
        })
        .collect();
    crate::sortc::any_by(&mut hits, |a, b| {
        b.0.pubmed
            .cmp(&a.0.pubmed)
            .then(a.0.mass.total_cmp(&b.0.mass))
            .then(a.0.key.cmp(&b.0.key))
    });
    let cut = hits.len().saturating_sub(p.max_suspects) as u32;
    hits.truncate(p.max_suspects);
    let out = hits
        .into_iter()
        .filter(|(h, _)| h.mass + 1.0 >= p.min_mz)
        .map(|(h, f)| {
            let label = String::from_utf8_lossy(&h.key).into_owned();
            hypothesis(&f, p, label, SRC_SUSPECT, 0, 0, Dir::Free, Vec::new())
        })
        .collect();
    (out, cut)
}

/// (c) members expected of a family base that the file may not show: M+1 and M+2, the sodium, ammonium and potassium adducts, and with
/// `other_polarity` the molecule in the other polarity. They are looked for at the RT of the base.
pub fn from_families(bases: &[FamilyBase], p: &HypoParams) -> Vec<Hypothesis> {
    let mut out = Vec::new();
    for b in bases.iter().filter(|b| b.polarity == p.polarity) {
        let own = if b.polarity >= 0 { "[M+H]+" } else { "[M-H]-" };
        let neutral = b.mz - ion_mz(0.0, own).unwrap_or(0.0);
        let mut add = |label: &str, adduct: &'static str, mz: f64, pol: i8| {
            out.push(Hypothesis {
                id: 0,
                sources: SRC_FAMILY,
                label: label.to_string(),
                formula: String::new(),
                neutral,
                mz,
                adduct,
                polarity: pol,
                steps: 0,
                level: 0,
                dir: Dir::Free,
                rt_ref: Some(b.rt),
                alternatives: Vec::new(),
            });
        };
        add("m+1", own, b.mz + D13C, b.polarity);
        add("m+2", own, b.mz + DPLUS2, b.polarity);
        let others: &[&'static str] = if b.polarity >= 0 {
            &["[M+NH4]+", "[M+Na]+", "[M+K]+"]
        } else {
            &["[M+Cl]-", "[M+HCOO]-"]
        };
        for a in others {
            add(a, a, neutral + ion_mz(0.0, a).unwrap_or(0.0), b.polarity);
        }
    }
    if p.other_polarity {
        for b in bases.iter().filter(|b| b.polarity == -p.polarity) {
            let own = if b.polarity >= 0 { "[M+H]+" } else { "[M-H]-" };
            let neutral = b.mz - ion_mz(0.0, own).unwrap_or(0.0);
            let (adduct, mz) = ion(neutral, p.polarity);
            out.push(Hypothesis {
                id: 0,
                sources: SRC_FAMILY,
                label: adduct.to_string(),
                formula: String::new(),
                neutral,
                mz,
                adduct,
                polarity: p.polarity,
                steps: 0,
                level: 0,
                dir: Dir::Free,
                rt_ref: Some(b.rt),
                alternatives: Vec::new(),
            });
        }
    }
    out.retain(|h| h.mz >= p.min_mz);
    out
}

/// (d) lists imported or typed by the user.
pub fn from_imported(list: &[Imported], p: &HypoParams) -> Result<Vec<Hypothesis>> {
    let mut out = Vec::new();
    for it in list {
        if it.formula.is_empty() {
            let neutral = it
                .mass
                .ok_or_else(|| Error::new("error.hypo.imported").with("name", &it.name))?;
            let (adduct, mz) = ion(neutral, p.polarity);
            out.push(Hypothesis {
                id: 0,
                sources: SRC_IMPORTED,
                label: it.name.clone(),
                formula: String::new(),
                neutral,
                mz,
                adduct,
                polarity: p.polarity,
                steps: 0,
                level: 0,
                dir: Dir::Free,
                rt_ref: None,
                alternatives: Vec::new(),
            });
            continue;
        }
        let f = elements::parse_formula(&it.formula)?;
        let label = it.name.clone();
        out.push(hypothesis(
            &f,
            p,
            label,
            SRC_IMPORTED,
            0,
            0,
            Dir::Free,
            Vec::new(),
        ));
    }
    out.retain(|h| h.mz >= p.min_mz);
    Ok(out)
}
