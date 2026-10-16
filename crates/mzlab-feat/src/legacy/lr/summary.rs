//! The JSON of `api.run` for a low-resolution series: the keys and types of `Experiment.summary` of the Python engine.

use mzlab_core::json::{round, Out};

use super::kin::{kinetics, Point};
use super::labels::Crit;
use super::{isf, nomi, plan, Entry, LrDig};
use crate::legacy::lrisf::IsfVerdict;

const LEVEL_TEXT: [(u8, &str); 3] = [
    (5, "Level 5: mass of interest. Ion with a compatible time course, nothing more."),
    (4, "Level 4: proposed formula. Derives from one known transformation of the parent and passes the criteria; at unit resolution the formula is not proven."),
    (3, "Level 3: tentative candidate. In addition, the product ions (MS2) are consistent with the parent (shared fragments or fragments shifted by Δ)."),
];
const TWO_STEPS: &str = "Level 5: mass of interest. The m/z matches two transformations of the parent: at unit resolution so many combinations cover most nominal masses that a match is no evidence of the formula.";

/// Status of the parent row when `parent_check` fails: no product is proposed.
pub const PARENT_NOT_FOUND: &str = "Parent not found";

const BY_CHANCE: &str = "Level 5: mass of interest. In this series the transformations of the parent match no more ions than the same masses shifted by a few Da (decoys): the match may be a coincidence.";

/// `confidence`: Schymanski level 5 or 4 as far as unit-resolution data without MS2 allow (never above 3, never 2 or 1). A level needs a chromatographic
/// peak (`peakchk`) behind the label; 4 needs a formula reached by one transformation (two steps match by chance at unit resolution) whose class of
/// hypotheses matches more ions than the decoys (`chance`).
pub fn level(e: &Entry) -> (Option<u8>, &'static str) {
    if e.id == 0 {
        return (None, "");
    }
    if !matches!(e.label, "forte" | "possibile") {
        return (None, "No level: the criteria are not met.");
    }
    if !e.formula.is_empty() && e.label == "forte" && e.steps > 1 {
        return (Some(5), TWO_STEPS);
    }
    if !e.formula.is_empty() && e.label == "forte" && e.by_chance {
        return (Some(5), BY_CHANCE);
    }
    let l = if !e.formula.is_empty() && e.label == "forte" {
        4
    } else {
        5
    };
    (
        Some(l),
        LEVEL_TEXT.iter().find(|x| x.0 == l).map_or("", |x| x.1),
    )
}

pub fn opt_str(o: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => o.str(s),
        None => o.raw("null"),
    };
}

pub fn strs(o: &mut Out, v: &[String]) {
    o.raw("[");
    for (i, s) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(s);
    }
    o.raw("]");
}

pub fn criteria(o: &mut Out, c: &[Crit]) {
    o.raw("[");
    for (i, x) in c.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(x.name);
        o.raw(",\"status\":")
            .str(x.status)
            .raw(",\"text\":")
            .str(&x.text)
            .raw("}");
    }
    o.raw("]");
}

fn kin_json(o: &mut Out, kin: &[Point]) {
    o.raw("[");
    for (i, p) in kin.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"time\":")
            .num(p.time);
        o.raw(",\"area\":")
            .num(p.area)
            .raw(",\"height\":")
            .num(p.height)
            .raw("}");
    }
    o.raw("]");
}

/// With a verdict (`isf`), a lighter ion is in-source when the probability of `isf` is at least one half; without one, an unexpected ion lighter than the
/// parent that co-elutes with it (within half the RT tolerance) is most likely an in-source fragment.
pub(super) fn insource(d: &LrDig, e: &Entry, p_rt: Option<f64>, v: Option<&IsfVerdict>) -> bool {
    if let Some(v) = v {
        return e.mz < d.parent.mz && isf::p_isf(v) >= 0.5;
    }
    e.unexpected
        && e.mz < d.parent.mz
        && matches!((p_rt, e.ref_rt), (Some(a), Some(b)) if (a - b).abs() <= 0.5 * d.settings.rt_tol_min)
}

struct Row<'a> {
    e: &'a Entry,
    kin: Vec<Point>,
    max_area: f64,
    insource: bool,
    isf: Option<&'a IsfVerdict>,
    ms2: usize,
    /// Total cosine of the MS2 with the parent's (`ms2sim`).
    sim: Option<f64>,
}

fn row_json(o: &mut Out, r: &Row, p_mz: f64, tp_name: &str, parent_formula: &str, status: &str) {
    let e = r.e;
    o.raw("{\"id\":")
        .int(e.id)
        .raw(",\"kind\":")
        .str(if e.unexpected {
            "unexpected"
        } else {
            "candidate"
        });
    o.raw(",\"name\":").str(&e.name);
    o.raw(",\"tp_name\":").str(tp_name);
    o.raw(",\"status\":").str(status);
    nomi::words_json(o, e, parent_formula);
    o.raw(",\"alternatives\":");
    strs(o, &e.alternatives);
    o.raw(",\"formula\":")
        .str(&e.formula)
        .raw(",\"delta\":")
        .str(&e.delta);
    o.raw(",\"mz\":")
        .num(round(e.mz, 4))
        .raw(",\"steps\":")
        .int(e.steps as i64);
    o.raw(",\"score\":")
        .opt(e.score)
        .raw(",\"label\":")
        .str(e.label);
    let (lev, ltxt) = level(e);
    o.raw(",\"level\":")
        .opt(lev.map(f64::from))
        .raw(",\"level_text\":")
        .str(ltxt);
    o.raw(",\"ref_rt\":")
        .opt(e.ref_rt.map(|v| round(v, 2)))
        .raw(",\"kinetics\":");
    kin_json(o, &r.kin);
    let tmax = r
        .kin
        .iter()
        .filter(|p| p.area > 0.0)
        .fold(None::<Point>, |m, p| match m {
            Some(q) if q.area >= p.area => Some(q),
            _ => Some(*p),
        });
    o.raw(",\"max_area\":")
        .num(r.max_area)
        .raw(",\"tmax\":")
        .opt(tmax.map(|p| p.time));
    o.raw(",\"isotope_of\":")
        .opt(e.isotope_of.map(|v| v as f64));
    o.raw(",\"generation\":");
    opt_str(o, e.generation);
    o.raw(",\"delta_mz\":").num(round(e.mz - p_mz, 2));
    o.raw(",\"isf\":");
    match r.isf {
        Some(v) => isf::verdict_json(o, v),
        None => {
            o.raw("null");
        }
    }
    o.raw(",\"insource\":")
        .raw(if r.insource { "true" } else { "false" });
    o.raw(",\"ms2\":").int(r.ms2 as i64);
    if let Some(v) = r.sim {
        o.raw(",\"sim\":").num(round(v, 3));
    }
    if e.ms2_target {
        o.raw(",\"ms2_target\":true");
    }
    o.raw("}");
}

fn rank(label: &str) -> u8 {
    match label {
        "progenitore" => 0,
        "forte" => 1,
        "possibile" => 2,
        "debole" => 3,
        _ => 4,
    }
}

/// The rows by label, then candidates before unexpected ions before in-source fragments, then score and area.
fn ordered(d: &LrDig) -> Vec<Row<'_>> {
    let p_rt = d.entries.first().and_then(|p| p.ref_rt);
    let rows: Vec<Row> = d
        .entries
        .iter()
        .map(|e| {
            let kin = kinetics(&d.lc, &e.cells);
            let max_area = kin.iter().map(|p| p.area).fold(0.0, f64::max);
            let isf = d.isf.iter().find(|x| x.0 == e.id).map(|x| &x.1);
            Row {
                insource: insource(d, e, p_rt, isf),
                ms2: d.links.as_ref().map_or(0, |l| l.of(e.id).count()),
                sim: d
                    .links
                    .as_ref()
                    .and_then(|l| l.sims.iter().find(|s| s.0 == e.id))
                    .map(|s| s.1.cos_total()),
                isf,
                e,
                kin,
                max_area,
            }
        })
        .collect();
    let group = |r: &Row| {
        if r.insource {
            2
        } else {
            u8::from(r.e.unexpected)
        }
    };
    let mut ord: Vec<usize> = (0..rows.len()).collect();
    mzlab_core::sortc::by(&mut ord, |&a, &b| {
        let (x, y) = (&rows[a], &rows[b]);
        (rank(x.e.label), group(x))
            .cmp(&(rank(y.e.label), group(y)))
            .then(
                y.e.score
                    .unwrap_or(0.0)
                    .total_cmp(&x.e.score.unwrap_or(0.0)),
            )
            .then(y.max_area.total_cmp(&x.max_area))
    });
    let mut slots: Vec<Option<Row>> = rows.into_iter().map(Some).collect();
    ord.into_iter().filter_map(|i| slots[i].take()).collect()
}

fn head_json(o: &mut Out, d: &LrDig) {
    o.raw("{\"offset\":");
    match &d.offset {
        Some(x) => {
            o.raw("{\"calc\":")
                .num(round(x.calc, 4))
                .raw(",\"observed\":")
                .num(round(x.observed, 3));
            o.raw(",\"offset\":")
                .num(round(x.offset, 3))
                .raw(",\"sample\":")
                .str(&x.sample);
            o.raw(",\"applied\":")
                .raw(if x.applied { "true" } else { "false" })
                .raw("}");
        }
        None => {
            o.raw("null");
        }
    }
    o.raw(",\"decay\":");
    match &d.decay {
        Some(x) => {
            o.raw("{\"k_per_min\":")
                .num(round(x.k, 5))
                .raw(",\"half_life_min\":")
                .opt(x.half_life.map(|v| round(v, 2)));
            o.raw(",\"r2\":")
                .num(round(x.r2, 3))
                .raw(",\"points\":")
                .int(x.points as i64)
                .raw("}");
        }
        None => {
            o.raw("null");
        }
    }
    o.raw(",\"parent\":{\"formula\":")
        .str(&d.parent.formula)
        .raw(",\"neutral_mass\":")
        .num(round(d.parent.neutral_mass, 4));
    o.raw(",\"adduct\":\"[M+H]+\",\"mz\":")
        .num(round(d.parent.mz, 4))
        .raw("}");
}

fn files_json(o: &mut Out, d: &LrDig) {
    let mut times: Vec<f64> = d.lc.iter().filter_map(|f| f.time).collect();
    mzlab_core::sortc::by(&mut times, |a, b| a.total_cmp(b));
    times.dedup();
    o.raw(",\"times\":")
        .floats(&times)
        .raw(",\"n_samples\":")
        .int(d.lc.len() as i64)
        .raw(",\"files\":[");
    for (i, f) in d.files.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(&f.name)
            .raw(",\"label\":")
            .str(&f.label);
        o.raw(",\"time\":")
            .opt(f.time)
            .raw(",\"type\":")
            .str(&f.kind_type)
            .raw(",\"kind\":")
            .str(&f.kind)
            .raw("}");
    }
    o.raw("]");
}

/// `api.run`: `timing` is a JSON object copied into it.
pub fn summary(d: &LrDig, timing: &str) -> String {
    let mut o = Out::default();
    head_json(&mut o, d);
    files_json(&mut o, d);
    o.raw(",\"rows\":[");
    let rows = ordered(d);
    // a hypothesis of the rules with no peak in any sample was searched and not found: no TP name, no area
    let found: Vec<bool> = d
        .entries
        .iter()
        .map(|e| {
            rows.iter()
                .find(|r| std::ptr::eq(r.e, e))
                .is_some_and(|r| e.label == "progenitore" || e.unexpected || r.max_area > 0.0)
        })
        .collect();
    let names = nomi::tp_names(&d.entries, &found);
    for (i, r) in rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        let at = d
            .entries
            .iter()
            .position(|x| std::ptr::eq(x, r.e))
            .unwrap_or(0);
        row_json(
            &mut o,
            r,
            d.parent.mz,
            &names[at],
            &d.parent.formula,
            if r.e.id == 0 && !d.parent_ok {
                PARENT_NOT_FOUND
            } else if found[at] {
                "Detected"
            } else {
                "Searched, not detected"
            },
        );
    }
    o.raw("],\"settings\":");
    plan::settings_json(&mut o, &d.settings);
    o.raw(",\"parent_ok\":")
        .raw(if d.parent_ok { "true" } else { "false" })
        .raw(",\"chance\":");
    super::chance::json(&mut o, &d.chance);
    if let Some(l) = &d.links {
        o.raw(",\"ms2_links\":");
        super::ms2link::json(&mut o, l);
    }
    let timing = if timing.trim().starts_with('{') {
        timing.trim()
    } else {
        "{}"
    };
    o.raw(",\"mrm\":null,\"timing\":").raw(timing).raw("}");
    o.0
}

/// The live feed of the page: the candidates that pass the criteria, `[{kind, id, name, mz, rt, score}]`.
pub fn live(d: &LrDig) -> String {
    let mut o = Out::default();
    o.raw("[");
    let pass = d
        .entries
        .iter()
        .filter(|e| !e.unexpected && e.id != 0 && matches!(e.label, "forte" | "possibile"));
    for (i, e) in pass.enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"kind\":\"candidate\",\"id\":")
            .int(e.id);
        o.raw(",\"name\":")
            .str(&e.name)
            .raw(",\"mz\":")
            .num(round(e.mz, 4));
        o.raw(",\"rt\":")
            .opt(e.ref_rt.map(|v| round(v, 2)))
            .raw(",\"score\":")
            .opt(e.score)
            .raw("}");
    }
    o.raw("]");
    o.0
}
