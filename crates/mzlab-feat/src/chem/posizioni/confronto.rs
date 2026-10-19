//! The comparison of the candidate positions with the observed MS2: which groups of positions the spectrum supports, excludes or cannot tell apart.
//!
//! Rules (all of them written in `docs/agenti/posizioni.md`):
//! * the spectrum is reduced to its 30 most intense peaks above 1 % of the base peak (precursor removed); «intense» means at least 10 % of the base peak;
//! * a candidate explains a peak WITH the change when a fragment carrying it matches, WITHOUT when a fragment that does not carry it matches;
//! * candidates with the same set of peaks explained with the change (and the same exclusion) form one class: the spectrum cannot tell them apart;
//! * a class is SUPPORTED when a peak explained with the change belongs, among the candidates not excluded, to exactly its members; EXCLUDED when an intense peak carries the change in a
//!   piece the members cannot carry it in and no other cleavage of theirs explains that peak; otherwise UNDISTINGUISHED.

use crate::sortc::SortC;
use std::collections::BTreeMap;

use mzlab_core::elements;
use mzlab_core::error::{Error, Result};

use super::frammenti::{expected_fragments, Frag};
use super::siti::Site;
use crate::chem::grafo::Mol;

pub const MAX_PEAKS: usize = 30;
const MIN_REL: f64 = 0.01;
const INTENSE_REL: f64 = 0.10;
const H2O: f64 = 18.010564684;
const OXYGEN: f64 = 15.99491461957;
/// Mass distance between the two natural isotopes of Cl (35/37) and of Br (79/81).
const ISO_CL: f64 = 36.96590260 - 34.968852682;
const ISO_BR: f64 = 80.9162906 - 78.9183376;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peak {
    pub mz: f64,
    pub inten: f64,
}

/// Matching tolerance: absolute in Da (low resolution) or relative in ppm (high resolution).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tol {
    Da(f64),
    Ppm(f64),
}

impl Tol {
    pub fn ok(self, obs: f64, calc: f64) -> bool {
        match self {
            Tol::Da(d) => (obs - calc).abs() <= d,
            Tol::Ppm(p) => (obs - calc).abs() <= calc.abs() * p * 1e-6,
        }
    }
    pub fn value(self) -> f64 {
        match self {
            Tol::Da(v) | Tol::Ppm(v) => v,
        }
    }
    pub fn unit(self) -> &'static str {
        match self {
            Tol::Da(_) => "da",
            Tol::Ppm(_) => "ppm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pol {
    Pos,
    Neg,
}

impl Pol {
    pub fn adduct(self) -> &'static str {
        match self {
            Pol::Pos => "[M+H]+",
            Pol::Neg => "[M-H]-",
        }
    }
    pub fn sign(self) -> &'static str {
        match self {
            Pol::Pos => "+",
            Pol::Neg => "-",
        }
    }
}

/// A fragment seen in the spectrum and its explanation: observed m/z, formula and atoms of the predicted side, m/z of the isotope peak (+2) when the
/// fragment carries Cl or Br and the spectrum has it.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    pub mz: f64,
    pub formula: String,
    pub atoms: Vec<usize>,
    pub iso_mz: Option<f64>,
}

/// A group of positions the spectrum treats alike. `status`: `supported`, `excluded` or `undistinguished`.
#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub id: usize,
    pub atoms: Vec<usize>,
    pub kind: String,
    pub status: &'static str,
    pub support: Vec<Evidence>,
    pub against: Vec<Evidence>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub tp_formula: String,
    pub tp_mz: f64,
    pub pol: Pol,
    pub tol: Tol,
    pub sites: Vec<Site>,
    pub classes: Vec<Class>,
    /// `(water loss seen, [M+H-O] seen)`; only for oxidations (`+O`).
    pub ox_type: Option<(bool, bool)>,
    pub unexplained: Vec<f64>,
    pub suggest: Vec<&'static str>,
    pub sublevel: &'static str,
    pub why: &'static str,
}

/// A candidate: the sites of one symmetry class and kind (one is enough to compute fragments), its fragments and what each peak says about it.
struct Cand {
    sites: Vec<usize>,
    rep: Site,
    frags: Vec<Frag>,
    with: Vec<Option<usize>>,
    without: Vec<bool>,
}

fn candidates(mol: &Mol, sites: &[Site], pol: Pol, used: &[Peak], tol: Tol) -> Vec<Cand> {
    let mut groups: BTreeMap<(u32, &str), Vec<&Site>> = BTreeMap::new();
    for s in sites {
        groups.entry((s.class, s.kind)).or_default().push(s);
    }
    let mut out: Vec<Cand> = groups
        .into_values()
        .map(|g| {
            let frags = expected_fragments(mol, g[0], pol);
            let mut c = Cand {
                sites: g
                    .iter()
                    .flat_map(|s| std::iter::once(s.atom).chain(s.partner))
                    .collect(),
                rep: g[0].clone(),
                frags,
                with: vec![None; used.len()],
                without: vec![false; used.len()],
            };
            for (i, f) in c.frags.iter().enumerate() {
                for (p, pk) in used.iter().enumerate() {
                    if tol.ok(pk.mz, f.mz) {
                        if f.modified {
                            c.with[p].get_or_insert(i);
                        } else {
                            c.without[p] = true;
                        }
                    }
                }
            }
            c
        })
        .collect();
    out.sortc_by_key(|c| c.sites[0]);
    out
}

/// The peaks the comparison works on, and the base peak of the whole spectrum.
fn select_peaks(peaks: &[Peak], prec: f64, tol: Tol) -> (Vec<Peak>, f64) {
    let base = peaks.iter().map(|p| p.inten).fold(0.0, f64::max);
    let mut v: Vec<Peak> = peaks
        .iter()
        .copied()
        .filter(|p| p.inten >= base * MIN_REL && !tol.ok(p.mz, prec))
        .collect();
    v.sortc_by(|a, b| b.inten.total_cmp(&a.inten));
    v.truncate(MAX_PEAKS);
    v.sortc_by(|a, b| a.mz.total_cmp(&b.mz));
    (v, base)
}

fn iso_shift(kind: &str) -> Option<f64> {
    match kind {
        "aromatic_cl" => Some(ISO_CL),
        "aromatic_br" => Some(ISO_BR),
        _ => None,
    }
}

fn evidence(c: &Cand, p: usize, used: &[Peak], tol: Tol) -> Evidence {
    let f = &c.frags[c.with[p].unwrap_or(0)];
    let iso_mz = iso_shift(c.rep.kind)
        .and_then(|d| used.iter().map(|q| q.mz).find(|m| tol.ok(*m, f.mz + d)));
    Evidence {
        mz: used[p].mz,
        formula: f.formula.clone(),
        atoms: f.atoms.clone(),
        iso_mz,
    }
}

/// Peaks that cannot be explained by any candidate, by the isotope peak of a halogenated fragment or by the precursor.
fn unexplained(cands: &[Cand], used: &[Peak], tol: Tol) -> Vec<f64> {
    let iso_hit = |m: f64| {
        cands.iter().any(|c| {
            iso_shift(c.rep.kind)
                .is_some_and(|d| c.frags.iter().any(|f| f.modified && tol.ok(m, f.mz + d)))
        })
    };
    (0..used.len())
        .filter(|p| {
            !cands.iter().any(|c| c.with[*p].is_some() || c.without[*p]) && !iso_hit(used[*p].mz)
        })
        .map(|p| used[p].mz)
        .collect()
}

/// For every candidate, the intense peaks that carry the change where the candidate cannot carry it.
fn excluded_by(cands: &[Cand], used: &[Peak], base: f64, s: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut out = vec![Vec::new(); cands.len()];
    for p in (0..used.len()).filter(|p| used[*p].inten >= base * INTENSE_REL && !s[*p].is_empty()) {
        for (ci, c) in cands.iter().enumerate() {
            if !s[p].contains(&ci) && !c.without[p] {
                out[ci].push(p);
            }
        }
    }
    out
}

fn classes_of(
    cands: &[Cand],
    used: &[Peak],
    tol: Tol,
    s: &[Vec<usize>],
    ex: &[Vec<usize>],
) -> Vec<Class> {
    // A peak supports a class when no candidate that is still possible (not excluded) explains it with the change except the members.
    let alive = |v: &Vec<usize>| -> Vec<usize> {
        v.iter().copied().filter(|ci| ex[*ci].is_empty()).collect()
    };
    let mut groups: BTreeMap<(Vec<usize>, bool), Vec<usize>> = BTreeMap::new();
    for (ci, c) in cands.iter().enumerate() {
        let fp: Vec<usize> = (0..used.len()).filter(|p| c.with[*p].is_some()).collect();
        groups.entry((fp, !ex[ci].is_empty())).or_default().push(ci);
    }
    let mut out: Vec<Class> = groups
        .into_iter()
        .map(|((fp, excluded), members)| {
            let first = &cands[members[0]];
            let status = if excluded {
                "excluded"
            } else if fp.iter().any(|p| alive(&s[*p]) == members) {
                "supported"
            } else {
                "undistinguished"
            };
            let mut atoms: Vec<usize> = members
                .iter()
                .flat_map(|m| cands[*m].sites.clone())
                .collect();
            atoms.sortc();
            let kind = if members.iter().all(|m| cands[*m].rep.kind == first.rep.kind) {
                first.rep.kind.to_string()
            } else {
                "mixed".to_string()
            };
            Class {
                id: 0,
                atoms,
                kind,
                status,
                support: fp.iter().map(|p| evidence(first, *p, used, tol)).collect(),
                against: ex[members[0]]
                    .iter()
                    .map(|p| evidence(&cands[s[*p][0]], *p, used, tol))
                    .collect(),
            }
        })
        .collect();
    out.sortc_by_key(|c| c.atoms[0]);
    for (i, c) in out.iter_mut().enumerate() {
        c.id = i;
    }
    out
}

/// The experiments that would still help: `hdx` when a C-OH and an N- or S-oxide are both left, `ms3` when a fragment with the change is shared by
/// several classes, `standard` always.
fn suggestions(cands: &[Cand], classes: &[Class], s: &[Vec<usize>]) -> Vec<&'static str> {
    let left = |c: &Cand| {
        classes
            .iter()
            .any(|k| k.status != "excluded" && k.atoms.contains(&c.sites[0]))
    };
    let kinds: Vec<&str> = cands
        .iter()
        .filter(|c| left(c))
        .map(|c| c.rep.kind)
        .collect();
    let c_oh = kinds
        .iter()
        .any(|k| matches!(*k, "aromatic_oh" | "aliphatic_oh"));
    let x_ox = kinds.iter().any(|k| matches!(*k, "n_oxide" | "s_oxide"));
    let class_of = |ci: usize| {
        classes
            .iter()
            .position(|k| k.atoms.contains(&cands[ci].sites[0]))
    };
    let shared = s.iter().any(|v| {
        let mut ids: Vec<Option<usize>> = v.iter().map(|ci| class_of(*ci)).collect();
        ids.sortc();
        ids.dedup();
        ids.len() > 1
    });
    let mut out = Vec::new();
    if c_oh && x_ox {
        out.push("hdx");
    }
    if shared {
        out.push("ms3");
    }
    out.push("standard");
    out
}

fn ox_type(prec: f64, used: &[Peak], tol: Tol) -> (bool, bool) {
    let seen = |loss: f64| used.iter().any(|p| tol.ok(p.mz, prec - loss));
    (seen(H2O), seen(OXYGEN))
}

/// Compares the candidate positions with the observed peaks. `sites` come from `sites()`; an empty list gives `error.posiz.noSites`.
pub fn match_spectrum(
    mol: &Mol,
    sites: &[Site],
    peaks: &[Peak],
    tol: Tol,
    pol: Pol,
) -> Result<Report> {
    if sites.is_empty() {
        return Err(Error::new("error.posiz.noSites"));
    }
    super::siti::check_formulas(sites)?;
    let tp = elements::parse_formula(&sites[0].tp_formula)?;
    let prec = elements::ion_mz(elements::mass(&tp), pol.adduct()).unwrap_or(0.0);
    let (used, base) = select_peaks(peaks, prec, tol);
    let cands = candidates(mol, sites, pol, &used, tol);
    let s: Vec<Vec<usize>> = (0..used.len())
        .map(|p| {
            (0..cands.len())
                .filter(|ci| cands[*ci].with[p].is_some())
                .collect()
        })
        .collect();
    let ex = excluded_by(&cands, &used, base, &s);
    let classes = classes_of(&cands, &used, tol, &s, &ex);
    let n_ex = classes.iter().filter(|c| c.status == "excluded").count();
    let (sublevel, why) = match (n_ex, classes.len() - n_ex) {
        (0, _) => ("3b", "none_excluded"),
        (_, left) if left <= 3 => ("3a", "excluded_few_left"),
        _ => ("3b", "many_classes_left"),
    };
    let oxid = matches!(
        sites[0].kind,
        "aromatic_oh" | "aliphatic_oh" | "n_oxide" | "s_oxide"
    );
    Ok(Report {
        tp_formula: sites[0].tp_formula.clone(),
        tp_mz: prec,
        pol,
        tol,
        sites: sites.to_vec(),
        suggest: suggestions(&cands, &classes, &s),
        classes,
        ox_type: oxid.then(|| ox_type(prec, &used, tol)),
        unexplained: unexplained(&cands, &used, tol),
        sublevel,
        why,
    })
}
