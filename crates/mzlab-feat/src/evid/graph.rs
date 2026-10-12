//! The graph of the proofs: one identity per row of the aligned table (a family with its members), its proofs graded by the table of points, the arcs
//! between members, from the parent, to the library and to the external results, and the order of the candidates. Pure function of its inputs: the
//! same tables give the same graph (stable sorts, no `HashMap`, sums in a fixed order).

use super::arcs::{proof_evidence, RowArc};
use super::*;
use crate::aligned::Aligned;
use crate::family::evidence::EdgeKind as FamilyEdge;
use crate::hypo::{Dir, Registry, SRC_RULE};
use crate::kinetic::{kinetic_report, Controls, Coupling, Setup};
use crate::table::*;
use mzlab_core::roles::PROTON;

pub const SOURCE_FEATURES: &str = "mzlab-feat/evid";
/// Window (ppm) to match a row to a hypothesis, RT window (min) when the hypothesis has one, margin (min) of the direction of the RT.
pub const HYPOTHESIS_PPM: f64 = 5.0;
pub const HYPOTHESIS_RT: f64 = 0.3;
pub const RT_MARGIN: f64 = 0.05;
/// The highest level reachable with and without an MS2 proof (no library match without an MS2).
pub const MAX_LEVEL_WITH_MS2: &str = "2a";
pub const MAX_LEVEL_WITHOUT_MS2: &str = "4";

/// Measurements that need the data of the files and are made by the caller; absent = not measured.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    /// Score of the fine isotope pattern of the family (`isoscore`), 0..1.
    pub isotope: Option<f64>,
    /// Share of the peaks of the MS2 explained by sub-formulas of the candidate.
    pub fragments: Option<f64>,
    /// Purity of the best MS2 of the row.
    pub purity: Option<f64>,
    /// Share of the fragments of the MS2 equal to those of the parent, shifted by the mass of the reaction or explained by losses.
    pub network: Option<f64>,
}

/// A hit of the library search on a row.
#[derive(Clone, Debug)]
pub struct LibHit {
    pub row: usize,
    pub name: String,
    pub library: String,
    pub entropy: f64,
    pub shared: u32,
    /// Error on the precursor, ppm.
    pub ppm: f64,
}

/// A result of another program on a row (SIRIUS/COSMIC, MetFrag, GNPS2, mzmine): `score` is the confidence it gives.
#[derive(Clone, Debug)]
pub struct ExternalHit {
    pub row: usize,
    pub name: String,
    pub score: f64,
    pub source: &'static str,
}

/// Kinetics of the series: the setup (times, banks, decay of the parent) and the files of the treated samples (in time order) and of the controls.
pub struct Kinetics<'a> {
    pub setup: &'a Setup<'a>,
    pub treated: &'a [usize],
    pub controls: &'a [usize],
}

pub struct Inputs<'a> {
    pub table: &'a PointsTable,
    pub aligned: &'a Aligned,
    pub facts: &'a [Facts],
    pub arcs: &'a [RowArc],
    pub registry: &'a Registry,
    pub libs: &'a [LibHit],
    pub external: &'a [ExternalHit],
    pub kinetics: Option<Kinetics<'a>>,
    /// m/z and RT of the parent in the table.
    pub parent: Option<(f64, f64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Adduct,
    Isf,
    Isotope,
    Undetermined,
    Other,
    ParentProduct,
    Library,
    External,
}

impl EdgeKind {
    pub fn name(self) -> &'static str {
        match self {
            EdgeKind::Adduct => "addotto",
            EdgeKind::Isf => "isf",
            EdgeKind::Isotope => "isotopo",
            EdgeKind::Undetermined => "non distinguibile",
            EdgeKind::Other => "altro",
            EdgeKind::ParentProduct => "progenitore-prodotto",
            EdgeKind::Library => "libreria",
            EdgeKind::External => "esterno",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Identity(u32),
    /// Identity and place among its members.
    Member(u32, u32),
    Library(String),
    External(String),
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub kind: EdgeKind,
    pub from: Node,
    pub to: Node,
    pub label: String,
    pub proofs: Vec<Evidence>,
}

#[derive(Clone, Copy, Debug)]
pub struct Member {
    pub mz: f64,
    pub rt: f64,
    pub role: u8,
}

#[derive(Clone, Debug)]
pub struct Identity {
    /// Row of the aligned table (the id is stable: it does not depend on the order of the candidates).
    pub id: u32,
    pub hypothesis: Option<u32>,
    pub neutral: f64,
    pub formula: Option<String>,
    pub members: Vec<Member>,
    pub breakdown: Breakdown,
    pub edges: Vec<Edge>,
    /// Largest area of the row over the files: the tie-break of equal totals (then the order of the rows).
    pub area_max: f64,
    /// The highest level the cascade can reach with these proofs: without an MS2 there is no library match, so never above 4.
    pub max_level: &'static str,
}

/// Row -> index of the hypothesis it confirms: the closest in m/z within the window (and in RT when the hypothesis has one).
fn match_hypotheses(inp: &Inputs) -> Vec<Option<usize>> {
    let hyps = &inp.registry.hyps;
    let mut by_mz: Vec<usize> = (0..hyps.len()).collect();
    by_mz.sort_by(|&a, &b| hyps[a].mz.total_cmp(&hyps[b].mz));
    (0..inp.aligned.len())
        .map(|r| {
            let (mz, rt) = (inp.aligned.mz[r], inp.aligned.rt[r]);
            let tol = mz * HYPOTHESIS_PPM * 1e-6;
            let from = by_mz.partition_point(|&h| hyps[h].mz < mz - tol);
            by_mz[from..]
                .iter()
                .take_while(|&&h| hyps[h].mz <= mz + tol)
                .copied()
                .filter(|&h| {
                    hyps[h]
                        .rt_ref
                        .is_none_or(|x| (f64::from(x) - rt).abs() <= HYPOTHESIS_RT)
                })
                .min_by(|&a, &b| (hyps[a].mz - mz).abs().total_cmp(&(hyps[b].mz - mz).abs()))
        })
        .collect()
}

/// Best library hit of every row (highest entropy, the first on a tie).
fn best_hits(inp: &Inputs) -> Vec<Option<usize>> {
    let mut best: Vec<Option<usize>> = vec![None; inp.aligned.len()];
    for (i, h) in inp.libs.iter().enumerate() {
        if h.row < best.len() && best[h.row].is_none_or(|b| inp.libs[b].entropy < h.entropy) {
            best[h.row] = Some(i);
        }
    }
    best
}

/// N of every candidate: itself and the others of the same neutral mass (within the tolerance of the table) that the MS2 does not separate. The same
/// rule as `indistinguishable`, with the masses sorted once.
fn counts_of_n(table: &PointsTable, c: &[Candidate]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..c.len()).collect();
    order.sort_by(|&a, &b| c[a].mass.total_cmp(&c[b].mass));
    let mut n = vec![1; c.len()];
    for (k, &i) in order.iter().enumerate() {
        let reach = c[i].mass * table.mass_ppm * 1e-6;
        let below = order[..k]
            .iter()
            .rev()
            .take_while(|&&j| c[i].mass - c[j].mass <= reach);
        let above = order[k + 1..]
            .iter()
            .take_while(|&&j| c[j].mass - c[i].mass <= reach);
        n[i] += below
            .chain(above)
            .filter(|&&j| !separated(table, c[i], c[j]))
            .count();
    }
    n
}

fn separated(table: &PointsTable, a: Candidate, b: Candidate) -> bool {
    matches!((a.ms2, b.ms2), (Some(x), Some(y)) if (x - y).abs() >= table.ms2_margin)
}

fn graded(
    inp: &Inputs,
    kind: Kind,
    value: f64,
    support: Option<f64>,
    opposed: bool,
    params: Vec<(&'static str, f64)>,
) -> Evidence {
    Evidence::graded(
        inp.table,
        kind,
        value,
        support,
        opposed,
        SOURCE_FEATURES,
        EVID_VERSION,
        params,
    )
}

fn frequency(inp: &Inputs, row: usize) -> (Evidence, usize, usize) {
    let nf = inp.aligned.n_files.max(1);
    let cells = &inp.aligned.state[row * nf..(row + 1) * nf];
    let det = cells.iter().filter(|&&s| s == CELL_DETECTED).count();
    let filled = cells.iter().filter(|&&s| s == CELL_FILLED).count();
    let e = graded(
        inp,
        Kind::Frequency,
        det as f64 / nf as f64,
        None,
        false,
        vec![("detected", det as f64), ("files", nf as f64)],
    );
    (e, det, filled)
}

fn kinetic_proofs(inp: &Inputs, row: usize) -> Vec<Evidence> {
    let Some(k) = &inp.kinetics else {
        return vec![];
    };
    let nf = inp.aligned.n_files;
    let area = |f: &usize| {
        let a = inp.aligned.area[row * nf + f];
        if a.is_finite() {
            a
        } else {
            0.0
        }
    };
    let (y, ctrl): (Vec<f64>, Vec<f64>) = (
        k.treated.iter().map(area).collect(),
        k.controls.iter().map(area).collect(),
    );
    if y.len() != k.setup.times.len() || y.iter().all(|&v| v <= 0.0) {
        return vec![];
    }
    let rep = kinetic_report(k.setup, &y, &ctrl);
    let ke = &rep.evidence;
    let params: Vec<_> = ke.params.clone();
    let kin = graded(
        inp,
        Kind::Kinetics,
        ke.value,
        None,
        rep.coupling == Coupling::TooFast,
        params,
    );
    let fc = if rep.controls == Controls::NoSignal {
        f64::NAN
    } else {
        rep.log2fc
    };
    let ctl = graded(
        inp,
        Kind::Controls,
        fc,
        None,
        rep.controls == Controls::Present,
        vec![],
    );
    vec![kin, ctl]
}

fn rt_proof(inp: &Inputs, row: usize, hyp: Option<usize>) -> Option<Evidence> {
    let (_, prt) = inp.parent?;
    let dir = inp.registry.hyps[hyp?].dir;
    let d = inp.aligned.rt[row] - prt;
    let opposed = match dir {
        Dir::MorePolar => d > RT_MARGIN,
        Dir::LessPolar => d < -RT_MARGIN,
        Dir::Free => return None,
    };
    Some(graded(
        inp,
        Kind::Rt,
        d,
        None,
        opposed,
        vec![("margin", RT_MARGIN)],
    ))
}

fn area_max(t: &Aligned, row: usize) -> f64 {
    t.area[row * t.n_files..(row + 1) * t.n_files]
        .iter()
        .copied()
        .filter(|a| a.is_finite())
        .fold(0.0, f64::max)
}

fn members_of(t: &Aligned, row: usize) -> Vec<Member> {
    let m = t.members(row);
    if m.is_empty() {
        return vec![Member {
            mz: t.mz[row],
            rt: t.rt[row],
            role: ROLE_M,
        }];
    }
    m.map(|k| Member {
        mz: t.member_mz[k],
        rt: f64::from(t.member_rt[k]),
        role: t.member_role[k],
    })
    .collect()
}

fn edge_of_arc(id: u32, a: &RowArc) -> Edge {
    let kind = match a.kind {
        FamilyEdge::Adduct => EdgeKind::Adduct,
        FamilyEdge::Isf => EdgeKind::Isf,
        FamilyEdge::Undetermined => EdgeKind::Undetermined,
        FamilyEdge::Other => EdgeKind::Other,
    };
    Edge {
        kind,
        from: Node::Member(id, a.lo as u32),
        to: Node::Member(id, a.up as u32),
        label: a.readings.join("/"),
        proofs: a.proofs.iter().map(proof_evidence).collect(),
    }
}

fn internal_edges(inp: &Inputs, row: usize, members: &[Member]) -> Vec<Edge> {
    let id = row as u32;
    let mut edges: Vec<Edge> = inp
        .arcs
        .iter()
        .filter(|a| a.row == row)
        .map(|a| edge_of_arc(id, a))
        .collect();
    for (pos, _) in members
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, m)| m.role == ROLE_ISOTOPE)
    {
        edges.push(Edge {
            kind: EdgeKind::Isotope,
            from: Node::Member(id, 0),
            to: Node::Member(id, pos as u32),
            label: String::new(),
            proofs: vec![],
        });
    }
    edges
}

fn parent_row(inp: &Inputs) -> Option<usize> {
    let (mz, rt) = inp.parent?;
    let tol = mz * HYPOTHESIS_PPM * 1e-6;
    (0..inp.aligned.len()).find(|&r| {
        (inp.aligned.mz[r] - mz).abs() <= tol && (inp.aligned.rt[r] - rt).abs() <= HYPOTHESIS_RT
    })
}

/// The graph: identities ordered by the explanation of their points (none against first, then the total, higher first, then the larger area; equal
/// ones keep the order of the rows).
pub fn build_graph(inp: &Inputs) -> Vec<Identity> {
    let n = inp.aligned.len();
    let hyp = match_hypotheses(inp);
    let best = best_hits(inp);
    let sign = f64::from(inp.registry.polarity.signum());
    let neutral = |r: usize| {
        hyp[r].map_or(inp.aligned.mz[r] - sign * PROTON, |h| {
            inp.registry.hyps[h].neutral
        })
    };
    let cands: Vec<Candidate> = (0..n)
        .map(|r| Candidate {
            mass: neutral(r),
            ms2: best[r].map(|b| inp.libs[b].entropy),
        })
        .collect();
    let ns = counts_of_n(inp.table, &cands);
    let parent = parent_row(inp);
    let mut ids: Vec<Identity> = (0..n)
        .map(|r| identity_of(inp, r, hyp[r], best[r], ns[r], parent, neutral(r)))
        .collect();
    ids.sort_by(|a, b| {
        a.breakdown
            .rank(&b.breakdown)
            .then_with(|| b.area_max.total_cmp(&a.area_max))
    });
    ids
}

fn identity_of(
    inp: &Inputs,
    row: usize,
    hyp: Option<usize>,
    lib: Option<usize>,
    n: usize,
    parent: Option<usize>,
    neutral: f64,
) -> Identity {
    let facts = inp.facts.get(row).cloned().unwrap_or_default();
    let h = hyp.map(|i| &inp.registry.hyps[i]);
    let mut ev: Vec<Evidence> = Vec::new();
    let ppm = h
        .map(|h| (inp.aligned.mz[row] - h.mz) / h.mz * 1e6)
        .or(lib.map(|l| inp.libs[l].ppm));
    if let Some(p) = ppm {
        ev.push(graded(inp, Kind::Mass, p, None, false, vec![]));
    }
    let opt = |k, v: Option<f64>| v.map(|v| graded(inp, k, v, None, false, vec![]));
    ev.extend(opt(Kind::Isotopes, facts.isotope));
    let ms2 = lib.map(|l| {
        let hit = &inp.libs[l];
        let params = facts
            .purity
            .map(|p| vec![("purity", p)])
            .unwrap_or_default();
        graded(
            inp,
            Kind::Ms2,
            hit.entropy,
            Some(f64::from(hit.shared)),
            false,
            params,
        )
    });
    ev.extend(ms2.clone());
    ev.extend(opt(Kind::Fragments, facts.fragments));
    ev.extend(opt(Kind::Network, facts.network));
    let is_parent = parent == Some(row);
    if !is_parent {
        ev.extend(kinetic_proofs(inp, row));
        ev.extend(rt_proof(inp, row, hyp));
    }
    let (freq, det, filled) = frequency(inp, row);
    ev.push(freq);
    let ext: Vec<&ExternalHit> = inp.external.iter().filter(|e| e.row == row).collect();
    ev.extend(
        ext.iter()
            .map(|e| graded(inp, Kind::External, e.score, None, false, vec![])),
    );
    let members = members_of(inp.aligned, row);
    let mut edges = internal_edges(inp, row, &members);
    let id = row as u32;
    if let (Some(p), Some(h), false) = (parent, h, is_parent) {
        if h.sources & SRC_RULE != 0 {
            let proofs = ev
                .iter()
                .filter(|e| matches!(e.kind, Kind::Network | Kind::Kinetics | Kind::Rt))
                .cloned()
                .collect();
            edges.push(Edge {
                kind: EdgeKind::ParentProduct,
                from: Node::Identity(p as u32),
                to: Node::Identity(id),
                label: h.label.clone(),
                proofs,
            });
        }
    }
    if let (Some(l), Some(e)) = (lib, ms2) {
        let hit = &inp.libs[l];
        edges.push(Edge {
            kind: EdgeKind::Library,
            from: Node::Identity(id),
            to: Node::Library(hit.name.clone()),
            label: hit.library.clone(),
            proofs: vec![e],
        });
    }
    for (x, e) in ext
        .iter()
        .zip(ev.iter().filter(|e| e.kind == Kind::External))
    {
        edges.push(Edge {
            kind: EdgeKind::External,
            from: Node::Identity(id),
            to: Node::External(x.name.clone()),
            label: x.source.to_string(),
            proofs: vec![e.clone()],
        });
    }
    let has_ms2 = ev.iter().any(|e| e.kind == Kind::Ms2 && !e.value.is_nan());
    let mut breakdown = breakdown(inp.table, ev, n);
    let undetermined = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Undetermined)
        .count();
    if undetermined > 0 {
        breakdown.reasons.push(Reason {
            code: R_UNDETERMINED_EDGE,
            numbers: vec![("arcs", undetermined as f64)],
        });
    }
    if det == 0 && filled > 0 {
        breakdown.reasons.push(Reason {
            code: R_ONLY_FILLED,
            numbers: vec![("filled", filled as f64)],
        });
    }
    Identity {
        id,
        hypothesis: h.map(|h| h.id),
        neutral,
        formula: h.map(|h| h.formula.clone()),
        members,
        breakdown,
        edges,
        area_max: area_max(inp.aligned, row),
        max_level: if has_ms2 {
            MAX_LEVEL_WITH_MS2
        } else {
            MAX_LEVEL_WITHOUT_MS2
        },
    }
}
