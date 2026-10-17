//! The entries of the low-resolution Dig on the table of the pipeline: the parent (its row also measures the calibration offset), one entry per
//! hypothesis of `ipotesi/1` (one per chromatographic peak when its nominal m/z has isomers) and the unexpected ions (rows that match nothing,
//! grow over time and are absent at t0 and in the blank).

use std::collections::HashSet;

use mzlab_core::elements::{ion_mz, mass, parse_formula};
use mzlab_core::json::{self, round, Value};

use super::super::series::FileIn;
use super::super::Res;
use super::labels::{crit, score, status, Cell, GROWS, PERSISTS};
use super::plan::{ParentLr, Settings};
use super::{Entry, Offset};
use crate::aligned::{Aligned, CELL_ABSENT, CELL_DETECTED};

/// The calibration offset is measured on a parent row within this many Da of its [M+H]+ and applied up to the same limit.
pub const OFFSET_MAX: f64 = 0.6;
/// Isomers of one hypothesis: a peak with at least this share of the strongest area, at most this many peaks.
const ISOMER_FRAC: f64 = 0.10;
const ISOMER_MAX: usize = 4;
pub const ISOTOPE: &str = "not an isotope peak";

pub const ISOTOPE_SHIFTS: [(f64, &str); 3] = [
    (1.00335, "13C (M+1)"),
    (2.0067, "M+2"),
    (1.99580, "34S (M+2)"),
];
/// First id of the unexpected ions and step of the ids of the isomers of one hypothesis.
const UID: i64 = 100_000;
const ISO_STEP: i64 = 10_000;

/// A hypothesis of `ipotesi/1` that names a product of the parent.
#[derive(Debug, Clone)]
pub struct Hyp {
    pub label: String,
    pub formula: String,
    pub mz: f64,
    pub neutral: f64,
    pub steps: u32,
    pub alternatives: Vec<String>,
    /// Prediction level of the weakest rule (Löffler: 2 literature, 3 class, 4 generic); 0 without one.
    pub pred: u8,
}

/// The hypotheses of the rules ([M+H]+, at least one step), sorted by steps and m/z.
pub fn hypotheses(registry: &str) -> Res<Vec<Hyp>> {
    if registry.trim().is_empty() {
        return Ok(Vec::new());
    }
    let v = json::parse(registry.as_bytes()).map_err(|e| e.to_string())?;
    let text = |h: &Value, k: &str| {
        h.get(k)
            .and_then(Value::str)
            .unwrap_or_default()
            .to_string()
    };
    let strs = |h: &Value, k: &str| -> Vec<String> {
        h.get(k)
            .map(Value::arr)
            .unwrap_or_default()
            .iter()
            .filter_map(Value::str)
            .map(String::from)
            .collect()
    };
    let out: Vec<Hyp> = v
        .get("hypotheses")
        .map(Value::arr)
        .unwrap_or_default()
        .iter()
        .filter(|h| text(h, "adduct") == "[M+H]+")
        .filter(|h| {
            strs(h, "sources")
                .iter()
                .any(|s| s == "regola" || s == "rottura")
        })
        .map(|h| Hyp {
            label: text(h, "label"),
            formula: text(h, "formula"),
            mz: h.get("mz").and_then(Value::f64).unwrap_or(f64::NAN),
            neutral: h.get("mass").and_then(Value::f64).unwrap_or(f64::NAN),
            steps: h.get("steps").and_then(Value::f64).unwrap_or(1.0) as u32,
            alternatives: strs(h, "alternatives"),
            pred: h.get("level").and_then(Value::f64).unwrap_or(0.0) as u8,
        })
        .filter(|h| h.steps >= 1 && h.mz.is_finite())
        .collect();
    let mut ord: Vec<usize> = (0..out.len()).collect();
    mzlab_core::sortc::by(&mut ord, |&a, &b| {
        out[a]
            .steps
            .cmp(&out[b].steps)
            .then(out[a].mz.total_cmp(&out[b].mz))
    });
    Ok(ord.into_iter().map(|i| promote(out[i].clone())).collect())
}

/// The merge by nominal mass orders the formulas of one ion by their text, so its leader may be a way of two steps while an alternative takes one: the
/// shortest way leads (the Dig names the row after it); the rest stay as alternatives.
fn promote(h: Hyp) -> Hyp {
    let steps = |l: &str| l.matches('+').count() as u32 + 1;
    let parsed = |a: &String| {
        let (f, rest) = a.split_once(" (")?;
        let label = rest.strip_suffix(')')?;
        let fm = parse_formula(f).ok()?;
        let neutral = mass(&fm);
        Some((
            f.to_string(),
            label.to_string(),
            neutral,
            ion_mz(neutral, "[M+H]+")?,
        ))
    };
    let best = h
        .alternatives
        .iter()
        .filter_map(|a| parsed(a).map(|p| (a, p)))
        .filter(|(_, p)| steps(&p.1) < h.steps)
        .min_by_key(|(_, p)| steps(&p.1));
    let Some((chosen, (formula, label, neutral, mz))) = best else {
        return h;
    };
    let mut alternatives: Vec<String> = h
        .alternatives
        .iter()
        .filter(|a| *a != chosen)
        .cloned()
        .collect();
    alternatives.push(format!("{} ({})", h.formula, h.label));
    let st = steps(&label);
    Hyp {
        label,
        formula,
        mz,
        neutral,
        steps: st,
        alternatives,
        pred: h.pred,
    }
}

/// The cells of row `r`, one per column.
pub fn cells(t: &Aligned, r: usize) -> Vec<Cell> {
    (0..t.n_files)
        .map(|k| {
            let c = t.cell(r, k);
            if t.state[c] == CELL_ABSENT || !t.area[c].is_finite() {
                return Cell::default();
            }
            let (a, w) = (f64::from(t.apex_rt[c]), f64::from(t.width[c]));
            let half = if w.is_finite() { w / 2.0 } else { 0.0 };
            let a = round(a, 4);
            Cell {
                area: t.area[c],
                height: f64::from(t.height[c]).max(0.0),
                apex_rt: a.is_finite().then_some(a),
                detected: t.state[c] == CELL_DETECTED,
                lo: a - half,
                hi: a + half,
            }
        })
        .collect()
}

pub(super) fn is_treated(f: &FileIn) -> bool {
    f.kind == "sample" && f.time.is_none_or(|t| t > 0.0)
}

/// Largest detected area of row `r` in the files `sel` keeps.
pub(super) fn best_area(t: &Aligned, files: &[FileIn], r: usize, sel: fn(&FileIn) -> bool) -> f64 {
    (0..t.n_files)
        .filter(|&k| {
            sel(&files[k])
                && t.state[t.cell(r, k)] == CELL_DETECTED
                && t.area[t.cell(r, k)].is_finite()
        })
        .map(|k| t.area[t.cell(r, k)])
        .fold(0.0, f64::max)
}

/// The row of the parent (the largest area in a sample within `OFFSET_MAX` of its ion) and the calibration offset it measures.
pub fn calibrate(
    t: &Aligned,
    files: &[FileIn],
    p: &ParentLr,
    s: &Settings,
) -> (Option<usize>, Option<Offset>) {
    let sample = |f: &FileIn| f.kind == "sample";
    let row = (0..t.len())
        .filter(|&r| (t.mz[r] - p.mz).abs() <= OFFSET_MAX && best_area(t, files, r, sample) > 0.0)
        .max_by(|&a, &b| best_area(t, files, a, sample).total_cmp(&best_area(t, files, b, sample)));
    let Some(r) = row else { return (None, None) };
    let k = (0..t.n_files)
        .filter(|&k| sample(&files[k]) && t.state[t.cell(r, k)] == CELL_DETECTED)
        .max_by(|&a, &b| t.area[t.cell(r, a)].total_cmp(&t.area[t.cell(r, b)]))
        .unwrap_or(0);
    let off = t.mz[r] - p.mz;
    let o = Offset {
        calc: p.mz,
        observed: t.mz[r],
        offset: off,
        sample: files[k].label.clone(),
        applied: s.auto_offset && off.abs() <= OFFSET_MAX,
    };
    (Some(r), Some(o))
}

/// An entry with its identity and no evaluation yet.
pub(super) fn entry(
    id: i64,
    unexpected: bool,
    name: String,
    mz: f64,
    mz_x: f64,
    row: Option<usize>,
) -> Entry {
    Entry {
        id,
        unexpected,
        name,
        mz,
        mz_x,
        row,
        steps: 1,
        ..Entry::default()
    }
}

/// Rows of the table assigned to the hypothesis with the fewest steps, then the nearest, within the tolerance; the others are free for the unexpected ions.
fn assign(t: &Aligned, known: &[f64], steps: &[u32], tol: f64) -> Vec<Option<usize>> {
    (0..t.len())
        .map(|r| {
            (0..known.len())
                .filter(|&i| (t.mz[r] - known[i]).abs() <= tol)
                .min_by(|&a, &b| {
                    steps[a].cmp(&steps[b]).then_with(|| {
                        (t.mz[r] - known[a])
                            .abs()
                            .total_cmp(&(t.mz[r] - known[b]).abs())
                    })
                })
        })
        .collect()
}

/// The hypotheses within the tolerance of the parent: the same trace at unit resolution, alternatives of the parent.
pub fn parent_alternatives(hyps: &[Hyp], parent: &ParentLr, tol: f64) -> Vec<String> {
    hyps.iter()
        .filter(|h| (h.mz - parent.mz).abs() <= tol)
        .map(|h| format!("{} ({}, m/z {:.4})", h.label, h.formula, h.mz))
        .collect()
}

/// One entry per hypothesis (but those of the parent), one per isomer peak when the nominal m/z has several in the treated samples.
pub fn candidates(
    t: &Aligned,
    files: &[FileIn],
    hyps: &[Hyp],
    parent: &ParentLr,
    off: f64,
    tol: f64,
) -> (Vec<Entry>, HashSet<usize>) {
    let known: Vec<f64> = hyps.iter().map(|h| h.mz + off).collect();
    let owner = assign(
        t,
        &known,
        &hyps.iter().map(|h| h.steps).collect::<Vec<_>>(),
        tol,
    );
    let mut out = Vec::new();
    let mut used = HashSet::new();
    for (i, h) in hyps.iter().enumerate() {
        if (h.mz - parent.mz).abs() <= tol {
            continue;
        }
        let rows: Vec<usize> = (0..t.len()).filter(|&r| owner[r] == Some(i)).collect();
        used.extend(rows.iter().copied());
        let area = |r: usize| best_area(t, files, r, is_treated);
        let mut peaks: Vec<usize> = rows.iter().copied().filter(|&r| area(r) > 0.0).collect();
        mzlab_core::sortc::by(&mut peaks, |&a, &b| {
            area(b).total_cmp(&area(a)).then(a.cmp(&b))
        });
        if let Some(&top) = peaks.first() {
            let a0 = area(top);
            peaks.retain(|&r| area(r) >= ISOMER_FRAC * a0);
            peaks.truncate(ISOMER_MAX);
        } else {
            let all = |r: usize| {
                (0..t.n_files)
                    .map(|k| t.area[t.cell(r, k)])
                    .filter(|a| a.is_finite())
                    .sum::<f64>()
            };
            peaks.extend(
                rows.iter()
                    .copied()
                    .max_by(|&a, &b| all(a).total_cmp(&all(b))),
            );
        }
        let id = i as i64 + 1;
        let many = peaks.len() > 1;
        let slots: Vec<Option<usize>> = if peaks.is_empty() {
            vec![None]
        } else {
            peaks.into_iter().map(Some).collect()
        };
        for (j, r) in slots.into_iter().enumerate() {
            let name = match r {
                Some(r) if many => format!("{} @ {:.2} min", h.label, t.rt[r]),
                _ => h.label.clone(),
            };
            let mut e = entry(id + ISO_STEP * j as i64, false, name, h.mz, h.mz + off, r);
            e.formula = h.formula.clone();
            e.alternatives = h.alternatives.clone();
            e.steps = h.steps;
            e.pred = h.pred;
            e.neutral_mass = Some(h.neutral);
            e.delta = super::delta(&h.formula, &parent.formula);
            out.push(e);
        }
    }
    (out, used)
}

/// The free rows (not within the tolerance of the parent or a hypothesis) after `rt_min` whose criteria pass on the cells of the table
/// (`forte` or `possibile`, growth, persistence, absent at t0 and in the blank not failed).
pub fn free_rows(t: &Aligned, files: &[FileIn], known: &[f64], s: &Settings) -> Vec<usize> {
    (0..t.len())
        .filter(|&r| t.rt[r] >= s.rt_min && !known.iter().any(|m| (t.mz[r] - m).abs() <= s.tol_da))
        .filter(|&r| {
            let sc = score(files, &cells(t, r), s.persist());
            let fine = [
                "absent in the blank",
                "absent at time zero",
                GROWS,
                PERSISTS,
            ]
            .iter()
            .all(|n| status(&sc.criteria, n) != "fail");
            matches!(sc.label, "forte" | "possibile") && fine
        })
        .collect()
}

/// The unexpected ions: free rows (not within the tolerance of the parent or a hypothesis) after `rt_min` whose criteria pass
/// (`forte` or `possibile`, growth not failed, absent at t0 and in the blank), not the isotope peak of a stronger ion, the best
/// of two neighbours, at most `max_unexpected` by score and area.
pub fn unexpected(
    t: &Aligned,
    files: &[FileIn],
    known: &[f64],
    s: &Settings,
    off: f64,
) -> Vec<Entry> {
    let mut found: Vec<(Entry, f64)> = Vec::new();
    for r in free_rows(t, files, known, s) {
        let area = best_area(t, files, r, is_treated);
        found.push((
            entry(0, true, String::new(), t.mz[r] - off, t.mz[r], Some(r)),
            area,
        ));
    }
    let tol = s.tol_da;
    let mut ord: Vec<usize> = (0..found.len()).collect();
    mzlab_core::sortc::by(&mut ord, |&a, &b| {
        found[b]
            .1
            .total_cmp(&found[a].1)
            .then(found[a].0.mz_x.total_cmp(&found[b].0.mz_x))
    });
    let mut kept: Vec<(Entry, f64)> = Vec::new();
    for (e, area) in ord.into_iter().map(|i| found[i].clone()) {
        let r = e.row.unwrap_or(0);
        let near = kept.iter().any(|(f, _)| {
            (e.mz_x - f.mz_x).abs() <= 1.2 * tol
                && (t.rt[r] - t.rt[f.row.unwrap_or(0)]).abs() <= s.rt_tol_min
        });
        if !near {
            kept.push((e, area));
        }
    }
    let mut out = Vec::new();
    for (i, (mut e, area)) in kept.into_iter().enumerate() {
        e.id = UID + 1 + i as i64;
        e.name = format!("unexpected ion m/z {:.1}", e.mz);
        e.area = area;
        out.push(e);
    }
    out
}

/// `_isotope_of`: a stronger ion one isotope step below in the same peak (RT within the tolerance) and at least twice its area.
pub fn isotope_of(e: &Entry, all: &[Entry], tol: f64, rt_tol: f64) -> Option<(i64, String)> {
    let area = e
        .cells
        .iter()
        .filter(|c| c.detected)
        .map(|c| c.area)
        .fold(0.0, f64::max);
    let rt = e.ref_rt?;
    if area <= 0.0 {
        return None;
    }
    for d in all.iter().filter(|d| d.id != e.id) {
        let Some(drt) = d.ref_rt else { continue };
        for (shift, name) in ISOTOPE_SHIFTS {
            if (d.mz_x + shift - e.mz_x).abs() > tol || (drt - rt).abs() > rt_tol {
                continue;
            }
            let b = d
                .cells
                .iter()
                .filter(|c| c.detected)
                .map(|c| c.area)
                .fold(0.0, f64::max);
            if b > 0.0 && area <= 0.5 * b {
                return Some((
                    d.id,
                    format!("looks like the {name} peak of m/z {:.4} ({})", d.mz, d.name),
                ));
            }
        }
    }
    None
}

/// The isotope criterion of an entry (it caps the score at 40 and the label at `debole` when it fails).
pub fn isotope_crit(e: &mut Entry, hit: Option<(i64, String)>) {
    match hit {
        Some((id, text)) => {
            e.criteria.push(crit(ISOTOPE, "fail", text));
            e.score = e.score.map(|s| s.min(40.0));
            e.label = "debole";
            e.isotope_of = Some(id);
        }
        None => e.criteria.push(crit(
            "not an isotope peak",
            "pass",
            "no more intense ion at M-1 or M-2 in the same peak".into(),
        )),
    }
}
