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
    let applies = |r: &Rule| r.polarity.map_or(true, |p| p == polarity);
    let hit_of = |r: &Rule| find(r, peaks, base, precursor_mz, ppm);
    let mut classes = Vec::new();
    for (ci, (id, name)) in table.classes.iter().enumerate() {
        let rules: Vec<&Rule> = table
            .rules
            .iter()
            .filter(|r| r.class == ci && applies(r))
            .collect();
        // Diagnostic rules are alternatives (one is enough): the full score is the strongest of them plus every supporting rule.
        let strongest = rules
            .iter()
            .filter(|r| r.role == Role::Diagnostic)
            .map(|r| r.weight)
            .fold(0.0, f64::max);
        let total: f64 = strongest
            + rules
                .iter()
                .filter(|r| r.role == Role::Support)
                .map(|r| r.weight)
                .sum::<f64>();
        let (mut got, mut evidence, mut unmet) = (0.0, Vec::new(), Vec::new());
        for r in rules {
            match hit_of(r) {
                Some(pi) => {
                    let (mz, inten) = peaks[pi];
                    let (observed, ppm_err) = match r.kind {
                        Kind::Frag => (mz, (mz - r.value) / r.value * 1e6),
                        Kind::Loss => (
                            precursor_mz - mz,
                            (precursor_mz - mz - r.value) / precursor_mz * 1e6,
                        ),
                    };
                    let mut also: Vec<String> = table
                        .rules
                        .iter()
                        .filter(|o| o.class != ci && applies(o) && hit_of(o) == Some(pi))
                        .map(|o| table.classes[o.class].0.clone())
                        .collect();
                    also.sort();
                    also.dedup();
                    got += r.weight;
                    evidence.push(Evidence {
                        rule: r.id.clone(),
                        kind: r.kind,
                        role: r.role,
                        formula: r.formula.clone(),
                        theoretical: r.value,
                        observed,
                        peak_mz: mz,
                        ppm: ppm_err,
                        rel_intensity: if base > 0.0 { inten / base } else { 0.0 },
                        why: r.why.clone(),
                        also,
                    });
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
        let diag = evidence
            .iter()
            .filter(|e| e.role == Role::Diagnostic)
            .count();
        let score = if total > 0.0 {
            (got / total).min(1.0)
        } else {
            0.0
        };
        if diag == 0 || score < SCORE_MIN {
            continue;
        }
        let mut ambiguous_with: Vec<String> = evidence
            .iter()
            .filter(|e| e.role == Role::Diagnostic)
            .flat_map(|e| e.also.clone())
            .collect();
        ambiguous_with.sort();
        ambiguous_with.dedup();
        let level = if diag >= 2 && score >= SCORE_3A && ambiguous_with.is_empty() {
            "3a"
        } else {
            "3b"
        };
        classes.push(ClassHit {
            id: id.clone(),
            name: name.clone(),
            score,
            level,
            ambiguous_with,
            evidence,
            unmet,
        });
    }
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
