//! The proofs of a candidate identity as data (`evid/1`, schema in `docs/schemi/evid-1.json`): every proof is a record {type, value, threshold,
//! points, sign, source, version, parameters}, the same as an entry of `evidence[]` of `mzid/1`. The points are stepped (`points::PointsTable`, the
//! section `punti` of `livelli/1`) and explain the order of the candidates: the total never assigns the level (that stays the cascade of
//! `LIVELLI.assess`; 2b and 1 only by the chemist), a proof "against" earns nothing and is never offset by the points of another, and the
//! probability is 1/N (N = candidates the proofs cannot tell apart). Every automatic decision leaves a reason code with the numbers behind it.

mod arcs;
mod graph;
mod out;
mod prepared;
pub mod points;
pub mod producers;
mod proofs;
pub mod rowproofs;

pub use arcs::{arcs_from_families, FileFamilies, RowArc};
pub use graph::{
    build_graph, match_hypotheses, Edge, EdgeKind, ExternalHit, Facts, Identity, Inputs, Kinetics,
    LibHit, Member, Node,
};
pub use out::{graph_json, GraphStream, EVID_GRAPH_FORMAT};

use mzlab_core::json::{round, Out};
pub use points::{Grade, PointsTable};
use std::cmp::Ordering;

/// Version of the rules that give the records and the reason codes.
pub const EVID_VERSION: u32 = 1;
pub const FORMAT: &str = "evid/1";

/// Types of proof, in the order of the column of the contributions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Mass,
    Isotopes,
    Ms2,
    Fragments,
    Network,
    Kinetics,
    Controls,
    Rt,
    Frequency,
    External,
}

impl Kind {
    pub const ALL: [Kind; 10] = [
        Kind::Mass,
        Kind::Isotopes,
        Kind::Ms2,
        Kind::Fragments,
        Kind::Network,
        Kind::Kinetics,
        Kind::Controls,
        Kind::Rt,
        Kind::Frequency,
        Kind::External,
    ];

    /// Name in the files (`tipo` of the schema).
    pub fn name(self) -> &'static str {
        match self {
            Kind::Mass => "massa",
            Kind::Isotopes => "isotopi",
            Kind::Ms2 => "MS2",
            Kind::Fragments => "frammenti",
            Kind::Network => "rete",
            Kind::Kinetics => "cinetica",
            Kind::Controls => "controlli",
            Kind::Rt => "RT",
            Kind::Frequency => "frequenza",
            Kind::External => "esterne",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    For,
    Against,
    Neutral,
}

impl Sign {
    /// Name in the files (`segno` of the schema).
    pub fn name(self) -> &'static str {
        match self {
            Sign::For => "a favore",
            Sign::Against => "contro",
            Sign::Neutral => "neutra",
        }
    }
}

/// One proof. `points` is never negative: an opposing proof carries 0.
#[derive(Clone, Debug, PartialEq)]
pub struct Evidence {
    pub kind: Kind,
    pub value: f64,
    pub threshold: Option<f64>,
    pub points: f64,
    pub sign: Sign,
    pub source: &'static str,
    pub version: u32,
    pub params: Vec<(&'static str, f64)>,
}

impl Evidence {
    /// A proof graded by the table: the sign is "for" when it earns points, "against" when `opposed`, else neutral. An opposing proof earns nothing.
    pub fn graded(
        table: &PointsTable,
        kind: Kind,
        value: f64,
        support: Option<f64>,
        opposed: bool,
        source: &'static str,
        version: u32,
        params: Vec<(&'static str, f64)>,
    ) -> Evidence {
        let g = table.grade(kind, value, support);
        let (points, sign) = match (opposed, g.points > 0.0) {
            (true, _) => (0.0, Sign::Against),
            (false, true) => (g.points, Sign::For),
            (false, false) => (0.0, Sign::Neutral),
        };
        Evidence {
            kind,
            value,
            threshold: g.threshold,
            points,
            sign,
            source,
            version,
            params,
        }
    }

    /// The kinetic proof of `kinetic::evidence` in the table: shape, coupling and controls stay with the kinetics; the points are those of the table
    /// (informative only while the ceiling of the type is 0), a proof "against" keeps its sign.
    pub fn from_kinetic(table: &PointsTable, k: &crate::kinetic::Evidence) -> Evidence {
        use crate::kinetic::Sign as KSign;
        let mut e = Evidence::graded(
            table,
            Kind::Kinetics,
            k.value,
            None,
            k.sign == KSign::Against,
            k.source,
            k.version,
            k.params.clone(),
        );
        if k.sign != KSign::For && e.sign == Sign::For {
            e.points = 0.0;
            e.sign = Sign::Neutral;
        }
        e
    }
}

/// An automatic decision of this layer: a code and the numbers that decided it (as `Q_SHAPE<0.5 in 13/13 files`).
#[derive(Clone, Debug, PartialEq)]
pub struct Reason {
    pub code: &'static str,
    pub numbers: Vec<(&'static str, f64)>,
}

/// A proof against stays open: it is flagged in the column, takes no point away and is never offset by the points of another.
pub const R_AGAINST_OPEN: &str = "EV_AGAINST_OPEN";
/// N > 1 candidates that the proofs do not tell apart: the probability is 1/N.
pub const R_INDISTINGUISHABLE: &str = "EV_INDISTINGUISHABLE";
/// No MS2 proof: no level above 4 can come from this identity.
pub const R_NO_MS2: &str = "EV_NO_MS2";
/// MS2 purity under 0.5: the MS2 proof is weak (it is not discarded).
pub const R_WEAK_MS2: &str = "EV_WEAK_MS2";
/// The kinetic proof is informative only (ceiling 0 in the table).
pub const R_KINETICS_INFO: &str = "EV_KINETICS_INFO";
/// An edge of the graph says the reading is not told from another (an adduct or a fragment, a loss or an ammonium): the identity keeps both.
pub const R_UNDETERMINED_EDGE: &str = "EV_UNDETERMINED_EDGE";
/// The row has peaks only where the gap filling put them: no file with a detected peak.
pub const R_ONLY_FILLED: &str = "EV_ONLY_FILLED";
/// The only proof in favour is the frequency (how many files show the peak): that proves nothing about the identity, so the total is 0 and the
/// identity is discarded.
pub const R_ONLY_FREQUENCY: &str = "EV_ONLY_FREQUENCY";
pub const WEAK_PURITY: f64 = 0.5;

/// What the interface shows for a candidate: the column of the contributions (always), the total, the number of proofs against, N, 1/N and the reasons.
#[derive(Clone, Debug, PartialEq)]
pub struct Breakdown {
    pub rows: Vec<Evidence>,
    pub total: f64,
    pub against: usize,
    pub n: usize,
    pub probability: f64,
    pub reasons: Vec<Reason>,
}

/// Probability of an identification when `n` candidates cannot be told apart (Metz 2025).
pub fn one_over_n(n: usize) -> f64 {
    1.0 / n.max(1) as f64
}

/// A candidate of a group for the count of N: neutral mass and, when there is an MS2 against its library entry, the score.
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub mass: f64,
    pub ms2: Option<f64>,
}

/// N of candidate `i`: the candidates (itself included) within the mass tolerance that the MS2 does not separate (a score missing on either side, or
/// scores closer than the margin of the table).
pub fn indistinguishable(table: &PointsTable, cands: &[Candidate], i: usize) -> usize {
    let me = cands[i];
    cands
        .iter()
        .enumerate()
        .filter(|&(j, c)| {
            j == i
                || (((c.mass - me.mass) / me.mass).abs() * 1e6 <= table.mass_ppm
                    && match (me.ms2, c.ms2) {
                        (Some(a), Some(b)) => (a - b).abs() < table.ms2_margin,
                        _ => true,
                    })
        })
        .count()
}

fn param(e: &Evidence, name: &str) -> Option<f64> {
    e.params.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

fn reasons_of(rows: &[Evidence], against: usize, n: usize, table: &PointsTable) -> Vec<Reason> {
    let mut r = Vec::new();
    if against > 0 {
        r.push(Reason {
            code: R_AGAINST_OPEN,
            numbers: vec![("against", against as f64)],
        });
    }
    if n > 1 {
        r.push(Reason {
            code: R_INDISTINGUISHABLE,
            numbers: vec![("n", n as f64)],
        });
    }
    match rows
        .iter()
        .find(|e| e.kind == Kind::Ms2 && !e.value.is_nan())
    {
        None => r.push(Reason {
            code: R_NO_MS2,
            numbers: vec![],
        }),
        Some(e) => {
            if let Some(p) = param(e, "purity").filter(|&p| (0.0..WEAK_PURITY).contains(&p)) {
                r.push(Reason {
                    code: R_WEAK_MS2,
                    numbers: vec![("purity", round(p, 3))],
                });
            }
        }
    }
    let cap = table
        .rule(Kind::Kinetics)
        .and_then(|k| k.cap)
        .unwrap_or(f64::INFINITY);
    if rows.iter().any(|e| e.kind == Kind::Kinetics) && cap <= 0.0 && !table.weighed {
        r.push(Reason {
            code: R_KINETICS_INFO,
            numbers: vec![("points_if_weighed", 1.0)],
        });
    }
    r
}

/// The contributions of a candidate: rows in the order of the types (stable inside a type), total = points of the proofs "for" (never reduced by the
/// others; 0 when the only one is the frequency), proofs against counted apart.
pub fn breakdown(table: &PointsTable, mut rows: Vec<Evidence>, n: usize) -> Breakdown {
    rows.sort_by_key(|e| e.kind);
    let points = |skip_frequency: bool| {
        rows.iter()
            .filter(|e| e.sign == Sign::For && !(skip_frequency && e.kind == Kind::Frequency))
            .fold(0.0, |s, e| s + e.points.max(0.0))
    };
    let only_frequency = points(true) <= 0.0 && points(false) > 0.0;
    let total = if only_frequency { 0.0 } else { points(false) };
    let against = rows.iter().filter(|e| e.sign == Sign::Against).count();
    let mut reasons = reasons_of(&rows, against, n, table);
    if only_frequency {
        reasons.push(Reason {
            code: R_ONLY_FREQUENCY,
            numbers: vec![],
        });
    }
    Breakdown {
        rows,
        total,
        against,
        n: n.max(1),
        probability: one_over_n(n),
        reasons,
    }
}

impl Breakdown {
    /// Order of the candidates: by total, higher first; with the same total the one with fewer proofs against; equal ones keep their place (stable sort).
    /// A proof against takes no point away, never lowers the level (that stays the cascade of `LIVELLI.assess`) and is always flagged. Revocable decision:
    /// "proofs against last" is the order `(against > 0).cmp(..)` first, then the total.
    pub fn rank(&self, other: &Breakdown) -> Ordering {
        other
            .total
            .total_cmp(&self.total)
            .then_with(|| self.against.cmp(&other.against))
    }
}

fn number(o: &mut Out, v: f64) {
    o.opt(v.is_finite().then(|| round(v, 6)));
}

/// A proof as an entry of `evidence[]` (`prova` of the schema).
pub fn evidence_json(o: &mut Out, e: &Evidence) {
    o.raw("{\"tipo\":").str(e.kind.name()).raw(",\"valore\":");
    number(o, e.value);
    o.raw(",\"soglia\":").opt(e.threshold.map(|t| round(t, 6)));
    o.raw(",\"punti\":").num(round(e.points, 6));
    o.raw(",\"segno\":")
        .str(e.sign.name())
        .raw(",\"fonte\":")
        .str(e.source);
    o.raw(",\"versione\":")
        .int(e.version as i64)
        .raw(",\"parametri\":{");
    for (i, (k, v)) in e.params.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(k).raw(":");
        number(o, *v);
    }
    o.raw("}}");
}

/// The part of an identity that this layer decides (`identita` of the schema, without its members and formula).
pub fn breakdown_json(o: &mut Out, id: u32, b: &Breakdown) {
    o.raw("{");
    breakdown_fields(o, id, b);
    o.raw("}");
}

/// The fields of `breakdown_json` without the braces, to be completed by the graph.
pub fn breakdown_fields(o: &mut Out, id: u32, b: &Breakdown) {
    o.raw("\"id\":").int(id as i64).raw(",\"contributi\":[");
    for (i, e) in b.rows.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        evidence_json(o, e);
    }
    o.raw("],\"totale\":").num(round(b.total, 6));
    o.raw(",\"contro\":")
        .int(b.against as i64)
        .raw(",\"n\":")
        .int(b.n as i64);
    o.raw(",\"probabilita\":")
        .num(round(b.probability, 6))
        .raw(",\"motivi\":[");
    for (i, r) in b.reasons.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.raw("{\"codice\":").str(r.code).raw(",\"numeri\":{");
        for (j, (k, v)) in r.numbers.iter().enumerate() {
            if j > 0 {
                o.raw(",");
            }
            o.str(k).raw(":");
            number(o, *v);
        }
        o.raw("}}");
    }
    o.raw("]");
}
