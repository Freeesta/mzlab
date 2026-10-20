//! The position of the modification of a row (`site` of `interp/1`): the candidate sites come from `chem::posizioni` grouped by symmetry, the votes from
//! the MSn tree of the parent (`localize_sites`) when there is one, otherwise from the cleavages of `posizioni::match_spectrum`. It declares how sure it
//! is (`site`, `region`, `uncertain`, `cannot_say`) with the reasons, and never gives an identity: the sublevel is the one of `posiz/1`.

use std::collections::BTreeSet;

use mzlab_core::elements::{self, Formula};

use crate::chem::grafo::Mol;
use crate::chem::posizioni::{self, Class, Peak, Pol, Report, Site, Tol};
use crate::legacy::fragtree::Graph;
use crate::legacy::msn::localize::default_groups_en;

/// Tolerance of the matching of the fragments with the cleavages, ppm (HR spectra).
pub const MATCH_PPM: f64 = 10.0;
/// A fragment is informative when it is at least this share of the base peak, %, and its formula is within `INFO_PPM`.
pub const MIN_REL: f64 = 5.0;
pub const INFO_PPM: f64 = 5.0;
/// Fewer informative fragments than this cannot tell one class from the others.
pub const MIN_INFORMATIVE: usize = 3;
/// Log-likelihood units a class must lead the second by to be called alone: two fragments' worth of evidence, so one lucky match never decides.
pub const MARGIN: f64 = 2.0;
/// Without the tree the likelihood of a class is built from its cleavages: a supported fragment adds `LL_SUPPORT`; a peak it cannot explain costs
/// `LL_AGAINST`, twice as much, because a wrong site is worse than an undecided one.
pub const LL_SUPPORT: f64 = 1.0;
pub const LL_AGAINST: f64 = 2.0;

/// What the pass knows about the MS2 of the row.
pub struct Facts {
    pub scans: usize,
    pub clean: usize,
    /// `single`, `separated` or `not_separated`.
    pub isomers: &'static str,
    pub has_reference: bool,
}

/// The candidate sites of a parent and a net change.
pub struct Candidates {
    pub mol: Mol,
    pub smiles: String,
    pub transform: &'static str,
    pub sites: Vec<Site>,
}

/// The votes of the tree: the log-likelihood of every site in the order of `Candidates::sites`, and for a loss the part that is kept.
pub struct TreeVotes {
    pub ll: Vec<f64>,
    pub retained: Option<(Vec<usize>, String)>,
}

/// One class of positions with its status, evidence and weight.
#[derive(Debug, Clone, PartialEq)]
pub struct Vote {
    pub id: usize,
    pub atoms: Vec<usize>,
    pub kind: String,
    pub status: &'static str,
    pub n_support: usize,
    pub n_against: usize,
    pub ll: f64,
    /// Softmax of `ll` among the classes that are not excluded; 0 for an excluded one.
    pub support: f64,
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SiteReport {
    pub transform: String,
    pub confidence: &'static str,
    pub reasons: Vec<&'static str>,
    /// `tree` or `cuts`.
    pub source: &'static str,
    pub votes: Vec<Vote>,
    pub sites: Vec<Site>,
    pub region: Option<(String, Vec<usize>)>,
    pub retained: Option<(Vec<usize>, String)>,
    pub informative: usize,
    pub margin: Option<f64>,
    pub sublevel: &'static str,
    pub why: &'static str,
}

/// The transformation that gives the net change `tp - parent`, or the reason `posizioni` cannot take it.
fn transform_of(parent: &Formula, tp: &Formula) -> Result<&'static str, &'static str> {
    let count = |f: &Formula, el: &str| f.iter().find(|x| x.0 == el).map_or(0i64, |x| x.1 as i64);
    let els: BTreeSet<&str> = parent.iter().chain(tp).map(|x| x.0.as_str()).collect();
    let d: Vec<(&str, i64)> = els
        .into_iter()
        .map(|e| (e, count(tp, e) - count(parent, e)))
        .filter(|x| x.1 != 0)
        .collect();
    Ok(match d.as_slice() {
        [("O", 1)] => "+O",
        [("C", -1), ("H", -2)] => "-CH2",
        [("H", -2)] => "-2H",
        [("H", -2), ("O", 1)] => "+O-2H",
        [("Cl", 1), ("H", -1)] => "+Cl-H",
        [("Br", 1), ("H", -1)] => "+Br-H",
        _ if d.iter().any(|x| x.0 == "O" && x.1 >= 2) => return Err("multiple_changes"),
        _ => return Err("unsupported_change"),
    })
}

/// The sites of the change from `parent` to `tp` (neutral formulas) on the structure `smiles`; `Err` = the reason of `cannot_say`.
pub fn candidates(
    smiles: &str,
    parent: &Formula,
    tp: &Formula,
) -> Result<Candidates, &'static str> {
    let mol = Mol::parse(smiles).map_err(|_| "unsupported_change")?;
    candidates_of(mol, smiles, parent, tp)
}

fn candidates_of(
    mol: Mol,
    smiles: &str,
    parent: &Formula,
    tp: &Formula,
) -> Result<Candidates, &'static str> {
    let transform = transform_of(parent, tp)?;
    let sites = posizioni::sites(&mol, transform).map_err(|_| "unsupported_change")?;
    Ok(Candidates {
        mol,
        smiles: smiles.to_string(),
        transform,
        sites,
    })
}

/// `candidates` for a parent given as a `mol/1` record (feature `record`); `smiles` is still the text the groups of the supposed tree are read from.
#[cfg(feature = "record")]
pub fn candidates_record(
    rec: &mzlab_mol::Mol,
    smiles: &str,
    parent: &Formula,
    tp: &Formula,
) -> Result<Candidates, &'static str> {
    let mol = Mol::from_record(rec, true).map_err(|_| "unsupported_change")?;
    candidates_of(mol, smiles, parent, tp)
}

/// The atoms of each site as a mask (the bond of a `-2H` takes both carbons), for `localize_sites`; None above 64 atoms.
pub fn masks(c: &Candidates) -> Option<Vec<u64>> {
    if c.mol.atoms.len() > 64 {
        return None;
    }
    Some(
        c.sites
            .iter()
            .map(|s| (1u64 << s.atom) | s.partner.map_or(0, |p| 1u64 << p))
            .collect(),
    )
}

/// A report that says nothing about the position, with the reasons.
pub fn undetermined(transform: &str, reasons: Vec<&'static str>) -> SiteReport {
    SiteReport {
        transform: transform.to_string(),
        confidence: "cannot_say",
        reasons,
        source: "cuts",
        votes: vec![],
        sites: vec![],
        region: None,
        retained: None,
        informative: 0,
        margin: None,
        sublevel: "3b",
        why: "no_exclusion",
    }
}

fn early_reasons(f: &Facts) -> Vec<&'static str> {
    let mut r = vec![];
    if f.scans == 0 {
        r.push("no_ms2");
    } else if f.clean == 0 {
        r.push("chimeric");
    }
    if f.isomers == "not_separated" {
        r.push("isomers_not_separated");
    }
    if !f.has_reference {
        r.push("no_reference");
    }
    r
}

/// Distinct fragments of the evidences that are informative: strong enough and with a formula that fits.
fn informative(rep: &Report, peaks: &[Peak]) -> usize {
    let top = peaks.iter().map(|p| p.inten).fold(0.0, f64::max).max(1e-12);
    let mut seen: Vec<f64> = vec![];
    for c in &rep.classes {
        for e in c.support.iter().chain(&c.against) {
            let rel = peaks
                .iter()
                .find(|p| p.mz == e.mz)
                .map_or(0.0, |p| 100.0 * p.inten / top);
            let theo = elements::parse_formula(&e.formula)
                .ok()
                .and_then(|f| elements::ion_mz(elements::mass(&f), rep.pol.adduct()));
            let fits = theo.is_some_and(|t| (e.mz - t).abs() <= t * INFO_PPM * 1e-6);
            if rel >= MIN_REL && fits && !seen.contains(&e.mz) {
                seen.push(e.mz);
            }
        }
    }
    seen.len()
}

fn group_of(atoms: &[usize], groups: &[(String, BTreeSet<usize>)]) -> Option<String> {
    groups
        .iter()
        .find(|g| atoms.iter().all(|a| g.1.contains(a)))
        .map(|g| g.0.clone())
}

fn class_ll(c: &Class, sites: &[Site], tree: Option<&TreeVotes>) -> f64 {
    match tree {
        Some(t) if t.ll.len() == sites.len() => sites
            .iter()
            .zip(&t.ll)
            .filter(|(s, _)| c.atoms.contains(&s.atom))
            .map(|(_, l)| *l)
            .fold(f64::NEG_INFINITY, f64::max),
        _ => LL_SUPPORT * c.support.len() as f64 - LL_AGAINST * c.against.len() as f64,
    }
}

fn votes_of(
    rep: &Report,
    tree: Option<&TreeVotes>,
    groups: &[(String, BTreeSet<usize>)],
) -> Vec<Vote> {
    let mut votes: Vec<Vote> = rep
        .classes
        .iter()
        .map(|c| Vote {
            id: c.id,
            atoms: c.atoms.clone(),
            kind: c.kind.clone(),
            status: c.status,
            n_support: c.support.len(),
            n_against: c.against.len(),
            ll: class_ll(c, &rep.sites, tree),
            support: 0.0,
            group: group_of(&c.atoms, groups),
        })
        .collect();
    let alive = |v: &Vote| v.status != "excluded" && v.ll.is_finite();
    let top = votes
        .iter()
        .filter(|v| alive(v))
        .map(|v| v.ll)
        .fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = votes
        .iter()
        .filter(|v| alive(v))
        .map(|v| (v.ll - top).exp())
        .sum();
    for v in votes.iter_mut().filter(|v| alive(v)) {
        v.support = (v.ll - top).exp() / sum;
    }
    votes
}

/// Fuses the sites and the votes into the declared confidence. `peaks` = consensus MS2 of the row (intensity = % of the base peak).
pub fn analyse(
    c: &Candidates,
    peaks: &[Peak],
    pol: Pol,
    facts: &Facts,
    tree: Option<TreeVotes>,
) -> SiteReport {
    let early = early_reasons(facts);
    if !early.is_empty() {
        return undetermined(c.transform, early);
    }
    let Ok(rep) = posizioni::match_spectrum(&c.mol, &c.sites, peaks, Tol::Ppm(MATCH_PPM), pol)
    else {
        return undetermined(c.transform, vec!["unsupported_change"]);
    };
    let groups = Graph::parse(&c.smiles).map_or(vec![], |g| default_groups_en(&g));
    let votes = votes_of(&rep, tree.as_ref(), &groups);
    let informative = informative(&rep, peaks);
    let mut out = decide(votes, informative, facts, &groups, &c.sites);
    out.transform = c.transform.to_string();
    out.source = if tree.as_ref().is_some_and(|t| t.ll.len() == c.sites.len()) {
        "tree"
    } else {
        "cuts"
    };
    out.sites = c.sites.clone();
    out.retained = tree.and_then(|t| t.retained);
    out.sublevel = if out.confidence == "cannot_say" {
        "3b"
    } else {
        rep.sublevel
    };
    out.why = rep.why;
    out
}

/// The confidence from the votes (the constants above and the table of the study).
fn decide(
    votes: Vec<Vote>,
    informative: usize,
    facts: &Facts,
    groups: &[(String, BTreeSet<usize>)],
    sites: &[Site],
) -> SiteReport {
    let mut out = undetermined("", vec![]);
    out.informative = informative;
    let mut alive: Vec<&Vote> = votes.iter().filter(|v| v.status != "excluded").collect();
    alive.sort_by(|a, b| b.ll.total_cmp(&a.ll));
    let reasons = &mut out.reasons;
    if informative < MIN_INFORMATIVE {
        reasons.push("few_fragments");
    }
    if alive.iter().all(|v| v.n_support == 0) {
        reasons.push("change_lost");
    }
    if !reasons.is_empty() {
        out.votes = votes;
        return out;
    }
    out.margin = alive.get(1).map(|v| alive[0].ll - v.ll);
    let within: Vec<&&Vote> = alive
        .iter()
        .filter(|v| v.ll >= alive[0].ll - MARGIN)
        .collect();
    let atoms: Vec<usize> = within.iter().flat_map(|v| v.atoms.clone()).collect();
    // One class is not enough: its atoms must also be the same position by symmetry (ring positions the fragments cannot tell apart are not).
    let same = |v: &Vote| {
        let mut cl = sites
            .iter()
            .filter(|s| v.atoms.contains(&s.atom))
            .map(|s| s.class);
        cl.next().is_some_and(|f| cl.all(|c| c == f))
    };
    let alone = within.len() == 1
        && alive[0].status == "supported"
        && alive[0].n_against == 0
        && same(alive[0]);
    out.confidence = if alone && facts.isomers == "single" {
        "site"
    } else if let Some(name) = group_of(&atoms, groups) {
        let set = groups.iter().find(|g| g.0 == name).map(|g| g.1.clone());
        out.region = set.map(|s| (name, s.into_iter().collect()));
        "region"
    } else {
        "uncertain"
    };
    out.votes = votes;
    out
}
