//! Matching of the rules of a `Table` against one MS/MS spectrum, and the `classi/1` answer.

use super::{Kind, Polarity, Role, Rule, Table, FORMAT, MIN_REL, SCORE_3A, SCORE_MIN};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;

/// A rule that matched a peak. `observed` is the m/z of the fragment or the observed Δm of the loss; `ppm` is relative to the theoretical m/z
/// of the fragment, or to the precursor m/z for a loss (both peaks carry the error of the Δm).
#[derive(Clone, Debug)]
pub struct Evidence {
    pub rule: String,
    pub kind: Kind,
    pub role: Role,
    pub formula: String,
    pub theoretical: f64,
    pub observed: f64,
    pub peak_mz: f64,
    pub ppm: f64,
    pub rel_intensity: f64,
    pub why: String,
    /// Other classes with a rule that matches the same peak within its tolerance.
    pub also: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Unmet {
    pub rule: String,
    pub kind: Kind,
    pub role: Role,
    pub theoretical: f64,
    pub why: String,
}

#[derive(Clone, Debug)]
pub struct ClassHit {
    pub id: String,
    pub name: String,
    pub score: f64,
    /// "3a" or "3b": never above 3, and the program never says «identified».
    pub level: &'static str,
    pub ambiguous_with: Vec<String>,
    pub evidence: Vec<Evidence>,
    pub unmet: Vec<Unmet>,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub polarity: Polarity,
    pub precursor_mz: f64,
    pub ppm: f64,
    pub classes: Vec<ClassHit>,
}

/// Peak that a rule matches: the closest one within the tolerance and above `MIN_REL` of the base peak.
fn find(rule: &Rule, peaks: &[(f64, f64)], base: f64, precursor: f64, ppm: f64) -> Option<usize> {
    let (target, tol) = match rule.kind {
        Kind::Frag => (rule.value, rule.value * ppm * 1e-6),
        Kind::Loss => (precursor - rule.value, precursor * ppm * 1e-6),
    };
    let mut best: Option<(usize, f64)> = None;
    for (i, &(mz, inten)) in peaks.iter().enumerate() {
        let d = (mz - target).abs();
        if d <= tol && inten >= MIN_REL * base && best.map_or(true, |(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

/// One spectrum and its settings, shared by the matching helpers.
struct Ctx<'a> {
    table: &'a Table,
    peaks: &'a [(f64, f64)],
    base: f64,
    precursor: f64,
    polarity: Polarity,
    ppm: f64,
}

fn applies(c: &Ctx, r: &Rule) -> bool {
    r.polarity.is_none_or(|p| p == c.polarity)
}

fn hit_of(c: &Ctx, r: &Rule) -> Option<usize> {
    find(r, c.peaks, c.base, c.precursor, c.ppm)
}

fn evidence_of(c: &Ctx, r: &Rule, pi: usize) -> Evidence {
    let (mz, inten) = c.peaks[pi];
    let (observed, ppm) = match r.kind {
        Kind::Frag => (mz, (mz - r.value) / r.value * 1e6),
        Kind::Loss => (
            c.precursor - mz,
            (c.precursor - mz - r.value) / c.precursor * 1e6,
        ),
    };
    let mut also: Vec<String> = c
        .table
        .rules
        .iter()
        .filter(|o| o.class != r.class && applies(c, o) && hit_of(c, o) == Some(pi))
        .map(|o| c.table.classes[o.class].0.clone())
        .collect();
    also.sort();
    also.dedup();
    Evidence {
        rule: r.id.clone(),
        kind: r.kind,
        role: r.role,
        formula: r.formula.clone(),
        theoretical: r.value,
        observed,
        peak_mz: mz,
        ppm,
        rel_intensity: if c.base > 0.0 { inten / c.base } else { 0.0 },
        why: r.why.clone(),
        also,
    }
}

/// The class `ci` if a diagnostic rule matched and the score reaches `SCORE_MIN`.
fn class_hit(c: &Ctx, ci: usize) -> Option<ClassHit> {
    let rules: Vec<&Rule> = c
        .table
        .rules
        .iter()
        .filter(|r| r.class == ci && applies(c, r))
        .collect();
    // Diagnostic rules are alternatives (one is enough): the full score is the strongest of them plus every supporting rule.
    let weights = |role: Role| {
        rules
            .iter()
            .filter(move |r| r.role == role)
            .map(|r| r.weight)
    };
    let total = weights(Role::Diagnostic).fold(0.0, f64::max) + weights(Role::Support).sum::<f64>();
    let (mut got, mut evidence, mut unmet) = (0.0, Vec::new(), Vec::new());
    for r in rules {
        match hit_of(c, r) {
            Some(pi) => {
                got += r.weight;
                evidence.push(evidence_of(c, r, pi));
            }
            None => unmet.push(Unmet {
                rule: r.id.clone(),
                kind: r.kind,
                role: r.role,
                theoretical: r.value,
                why: r.why.clone(),
            }),
        }
    }
    let diagnostic = || evidence.iter().filter(|e| e.role == Role::Diagnostic);
    let diag = diagnostic().count();
    let score = if total > 0.0 {
        (got / total).min(1.0)
    } else {
        0.0
    };
    if diag == 0 || score < SCORE_MIN {
        return None;
    }
    let mut ambiguous_with: Vec<String> = diagnostic().flat_map(|e| e.also.clone()).collect();
    ambiguous_with.sort();
    ambiguous_with.dedup();
    let level = if diag >= 2 && score >= SCORE_3A && ambiguous_with.is_empty() {
        "3a"
    } else {
        "3b"
    };
    let (id, name) = c.table.classes[ci].clone();
    Some(ClassHit {
        id,
        name,
        score,
        level,
        ambiguous_with,
        evidence,
        unmet,
    })
}

pub fn evaluate(
    table: &Table,
    peaks: &[(f64, f64)],
    precursor_mz: f64,
    polarity: Polarity,
    ppm: f64,
) -> Result<Report> {
    if !(ppm > 0.0 && ppm <= 2000.0)
        || !(precursor_mz > 0.0)
        || peaks.iter().any(|p| !p.0.is_finite() || !p.1.is_finite())
    {
        return Err(Error::new("error.classi.input"));
    }
    let base = peaks.iter().map(|p| p.1).fold(0.0, f64::max);
    let ctx = Ctx {
        table,
        peaks,
        base,
        precursor: precursor_mz,
        polarity,
        ppm,
    };
    let mut classes: Vec<ClassHit> = (0..table.classes.len())
        .filter_map(|ci| class_hit(&ctx, ci))
        .collect();
    classes.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
    Ok(Report {
        polarity,
        precursor_mz,
        ppm,
        classes,
    })
}

fn kind_str(k: Kind) -> &'static str {
    if k == Kind::Frag {
        "frag"
    } else {
        "loss"
    }
}

fn role_str(r: Role) -> &'static str {
    if r == Role::Diagnostic {
        "diagnostic"
    } else {
        "support"
    }
}

fn strs(o: &mut Out, a: &[String]) {
    o.raw("[");
    for (i, s) in a.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(s);
    }
    o.raw("]");
}

/// The `classi/1` answer. Numbers are rounded to 4 decimals (m/z, Δm), 2 (ppm) and 3 (score, relative intensity).
pub fn to_json(r: &Report) -> String {
    use mzlab_core::json::round;
    let mut o = Out::default();
    o.raw("{\"format\":")
        .str(FORMAT)
        .raw(",\"polarity\":")
        .str(if r.polarity == Polarity::Pos {
            "+"
        } else {
            "-"
        });
    o.raw(",\"precursor_mz\":")
        .num(r.precursor_mz)
        .raw(",\"ppm\":")
        .num(r.ppm)
        .raw(",\"classes\":[");
    for (i, c) in r.classes.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.raw("{\"id\":")
            .str(&c.id)
            .raw(",\"name\":")
            .str(&c.name)
            .raw(",\"score\":")
            .num(round(c.score, 3));
        o.raw(",\"level\":")
            .str(c.level)
            .raw(",\"ambiguous_with\":");
        strs(&mut o, &c.ambiguous_with);
        o.raw(",\"evidence\":[");
        for (j, e) in c.evidence.iter().enumerate() {
            if j > 0 {
                o.raw(",");
            }
            o.raw("{\"rule\":")
                .str(&e.rule)
                .raw(",\"kind\":")
                .str(kind_str(e.kind))
                .raw(",\"role\":")
                .str(role_str(e.role));
            o.raw(",\"formula\":")
                .str(&e.formula)
                .raw(",\"theoretical\":")
                .num(round(e.theoretical, 4));
            o.raw(",\"observed\":")
                .num(round(e.observed, 4))
                .raw(",\"peak_mz\":")
                .num(round(e.peak_mz, 4));
            o.raw(",\"ppm\":")
                .num(round(e.ppm, 2))
                .raw(",\"rel_intensity\":")
                .num(round(e.rel_intensity, 3));
            o.raw(",\"why\":").str(&e.why).raw(",\"also\":");
            strs(&mut o, &e.also);
            o.raw("}");
        }
        o.raw("],\"unmet\":[");
        for (j, u) in c.unmet.iter().enumerate() {
            if j > 0 {
                o.raw(",");
            }
            o.raw("{\"rule\":")
                .str(&u.rule)
                .raw(",\"kind\":")
                .str(kind_str(u.kind))
                .raw(",\"role\":")
                .str(role_str(u.role));
            o.raw(",\"theoretical\":")
                .num(round(u.theoretical, 4))
                .raw(",\"why\":")
                .str(&u.why)
                .raw("}");
        }
        o.raw("]}");
    }
    o.raw("]}");
    o.0
}
