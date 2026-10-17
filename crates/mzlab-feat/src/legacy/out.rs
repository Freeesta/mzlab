//! The JSON of `api.run` (HR) and `api.detail` that `TP_Mine/js/21_tpmine_hr.js` reads, and the inclusion list, written from a `Dig`.

use crate::sortc::SortC;
use mzlab_core::json::{round, Out};

use super::dig::{Cand, Dig, INCLUSION, KINDS, MAX_ROWS};
use super::kinetics;
use super::series::Proof;

const LEVEL_TEXT: [&str; 2] = [
    "Level 4: formula of a hypothesis derived from the parent, within the mass tolerance.",
    "Level 5: mass of interest. Appears with the treatment and has a proof in favour; nothing more.",
];
const PRED_TEXT: [&str; 5] = [
    "",
    "confirmed by the data",
    "observed in the literature",
    "specific to the class",
    "generic rule",
];

fn opt_round(o: &mut Out, v: Option<f64>, nd: usize) {
    o.opt(v.map(|x| round(x, nd)));
}

fn opt_str(o: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => o.str(s),
        None => o.raw("null"),
    };
}

fn strs(o: &mut Out, v: &[impl AsRef<str>]) {
    o.raw("[");
    for (i, s) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(s.as_ref());
    }
    o.raw("]");
}

fn ints(o: &mut Out, v: impl Iterator<Item = i64>) {
    o.raw("[");
    for (i, x) in v.enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(x);
    }
    o.raw("]");
}

/// `_sim_row`: [equal, shifted, loss] of the cosine and of the entropy, rounded to 3, and the matched peaks.
fn sim_row(o: &mut Out, c: &Cand) {
    let Some(m) = &c.ms2 else {
        o.raw("null");
        return;
    };
    o.raw("{\"cos\":")
        .rounded(&m.sim.cos, 3)
        .raw(",\"ent\":")
        .rounded(&m.sim.ent, 3)
        .raw(",\"n\":");
    ints(o, m.sim.n.iter().map(|&n| n as i64));
    o.raw("}");
}

fn components(o: &mut Out, c: &Cand, text: bool) {
    o.raw("{");
    for (i, (_, k, _)) in KINDS.iter().enumerate() {
        o.raw(if i > 0 { ",\"" } else { "\"" })
            .raw(k)
            .raw("\":")
            .num(if text {
                c.comps[i]
            } else {
                round(c.comps[i], 3)
            });
    }
    if text {
        o.raw(",\"text\":{");
        for (i, (k, key, _)) in KINDS.iter().enumerate() {
            let p: Vec<&Proof> = c
                .ident
                .proofs
                .iter()
                .filter(|p| p.kind == k.name())
                .collect();
            o.raw(if i > 0 { ",\"" } else { "\"" })
                .raw(key)
                .raw("\":")
                .str(&proof_text(&p));
        }
        o.raw("}");
    }
    o.raw("}");
}

fn proof_text(p: &[&Proof]) -> String {
    if p.is_empty() {
        return "not measured".into();
    }
    let one = |p: &Proof| {
        let v = p.value.map_or("not measured".to_string(), |v| {
            format!("value {}", round(v, 4))
        });
        let t = p
            .threshold
            .map_or(String::new(), |t| format!(", threshold {}", round(t, 4)));
        format!("{v}{t}; points {}", round(p.points, 3))
    };
    p.iter().map(|x| one(x)).collect::<Vec<_>>().join("; ")
}

/// `_row` of the Python engine.
fn row(o: &mut Out, c: &Cand, files: usize) {
    let kin = c.kin.as_ref().map(|k| &k.0);
    o.raw("{\"id\":")
        .int(c.id())
        .raw(",\"mz\":")
        .num(round(c.mz, 4))
        .raw(",\"rt\":")
        .num(round(c.rt, 2));
    o.raw(",\"formula\":");
    opt_str(o, c.formula.as_deref());
    o.raw(",\"ppm\":");
    opt_round(o, c.ppm, 1);
    o.raw(",\"n_formulas\":")
        .int(i64::from(c.formula.is_some()))
        .raw(",\"delta\":");
    opt_str(o, c.delta.as_deref());
    o.raw(",\"derivation\":");
    opt_str(
        o,
        c.hyp
            .as_ref()
            .map(|h| h.label.as_str())
            .filter(|s| !s.is_empty()),
    );
    o.raw(",\"area_max\":")
        .num(c.area_max)
        .raw(",\"tmax\":")
        .opt(kin.map(|k| k.tmax))
        .raw(",\"class\":");
    opt_str(o, kin.map(|k| k.class_text()).as_deref());
    o.raw(",\"onset\":")
        .opt(kin.map(|k| k.onset))
        .raw(",\"score\":")
        .num(round(c.score, 1))
        .raw(",\"components\":");
    components(o, c, false);
    o.raw(",\"level\":")
        .int(i64::from(c.level()))
        .raw(",\"diagnostic\":false,\"ms2_scans\":")
        .int(c.ms2_scans as i64);
    let m = c.ms2.as_ref();
    o.raw(",\"modcos\":");
    opt_round(o, m.map(|m| m.sim.cos_total()), 3);
    o.raw(",\"n_matched\":")
        .int(m.map_or(0, |m| m.sim.n.iter().sum::<usize>()) as i64)
        .raw(",\"sim\":");
    sim_row(o, c);
    o.raw(",\"pred\":");
    match c.pred() {
        Some(l) => o.int(i64::from(l)),
        None => o.raw("null"),
    };
    o.raw(",\"entropy\":");
    opt_round(o, m.map(|m| m.sim.ent_total()), 3);
    o.raw(",\"region\":null,\"margin\":null,\"predecessor\":null,\"transformation\":");
    opt_str(
        o,
        c.hyp
            .as_ref()
            .map(|h| h.label.as_str())
            .filter(|s| !s.is_empty()),
    );
    o.raw(",\"flags\":");
    strs(o, &c.flags);
    o.raw(",\"filled\":");
    ints(o, c.filled.iter().map(|&k| k as i64));
    o.raw(",\"series\":");
    ints(
        o,
        c.areas
            .iter()
            .take(files)
            .map(|a| a.round_ties_even() as i64),
    );
    o.raw("}");
}

fn rt_span(c: &Cand) -> (f64, f64) {
    let lo = c
        .limits
        .iter()
        .map(|l| l.0)
        .filter(|v| v.is_finite())
        .fold(f64::INFINITY, f64::min);
    let hi = c
        .limits
        .iter()
        .map(|l| l.1)
        .filter(|v| v.is_finite())
        .fold(f64::NEG_INFINITY, f64::max);
    if lo.is_finite() && hi.is_finite() {
        (lo, hi)
    } else {
        (c.rt, c.rt)
    }
}

/// The candidates of the inclusion list: without MS2, best score first.
fn inclusion(d: &Dig) -> Vec<&Cand> {
    let mut v: Vec<&Cand> = d.cands.iter().filter(|c| c.ms2.is_none()).collect();
    v.sortc_by(|a, b| b.score.total_cmp(&a.score));
    v.truncate(INCLUSION);
    v
}

/// `rt_align` of the summary: the curves of the RT map (`null` without enough anchors).
fn rt_align(o: &mut Out, d: &Dig) {
    let m = &d.rt_map;
    if !m.ok() {
        o.raw("null");
        return;
    }
    let nodes = m.grid.len();
    o.raw("{\"files\":");
    strs(
        o,
        &d.files.iter().map(|f| f.label.as_str()).collect::<Vec<_>>(),
    );
    o.raw(",\"rt\":").floats(&m.grid).raw(",\"shift\":[");
    for k in 0..m.n_files {
        o.raw(if k > 0 { "," } else { "" })
            .floats(&m.shift[k * nodes..(k + 1) * nodes]);
    }
    o.raw("],\"anchors\":")
        .int(m.n_anchors as i64)
        .raw(",\"rms\":")
        .floats(&m.rms)
        .raw("}");
}

fn files_json(o: &mut Out, d: &Dig) {
    o.raw("[");
    for (k, f) in d.files.iter().enumerate() {
        o.raw(if k > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(&f.name)
            .raw(",\"label\":")
            .str(&f.label);
        o.raw(",\"time\":")
            .opt(f.time)
            .raw(",\"type\":")
            .str(&f.kind)
            .raw(",\"kind\":\"hr\",\"session\":\"S1\"");
        o.raw(",\"n_features\":")
            .int(d.n_features[k] as i64)
            .raw("}");
    }
    o.raw("]");
}

/// The fragments in source left out of the candidates.
fn isf_json(o: &mut Out, d: &Dig) {
    let hidden = d
        .cands
        .iter()
        .filter(|c| c.hidden.is_some())
        .map(|c| (c.mz, c.rt, true));
    let left_out = d.isf_hidden.iter().map(|&(mz, rt)| (mz, rt, false));
    for (i, (mz, rt, cand)) in left_out.chain(hidden).enumerate() {
        o.raw(if i > 0 { ",{\"mz\":" } else { "{\"mz\":" })
            .num(round(mz, 4))
            .raw(",\"rt\":")
            .num(round(rt, 2))
            .raw(if cand {
                ",\"candidate\":true}"
            } else {
                ",\"candidate\":false}"
            });
    }
}

/// The inclusion list: m/z, RT window, formula and score of the candidates worth a targeted MS2.
fn inclusion_json(o: &mut Out, d: &Dig) {
    for (i, c) in inclusion(d).iter().enumerate() {
        let (a, b) = rt_span(c);
        o.raw(if i > 0 { ",{\"mz\":" } else { "{\"mz\":" })
            .num(round(c.mz, 4))
            .raw(",\"rt\":")
            .num(round(c.rt, 2));
        o.raw(",\"rt_from\":")
            .num(round(a, 2))
            .raw(",\"rt_to\":")
            .num(round(b, 2))
            .raw(",\"formula\":");
        opt_str(o, c.formula.as_deref());
        o.raw(",\"score\":").num(round(c.score, 1)).raw("}");
    }
}

/// `api.run` (HR): `settings` and `timing` are JSON objects written as they come.
pub fn summary(d: &Dig, settings: &str, timing: &str) -> String {
    summary_with(d, settings, timing, "null")
}

/// The same with the network of mass differences (`network::mass_network`, a JSON object or `null`).
pub fn summary_with(d: &Dig, settings: &str, timing: &str, network: &str) -> String {
    let mut o = Out::default();
    let p = &d.parent;
    o.raw("{\"mode\":\"hr\",\"engine\":\"rust\",\"parent\":{\"formula\":")
        .str(&p.formula())
        .raw(",\"ion\":")
        .str(&p.ion_formula())
        .raw(",\"adduct\":")
        .str(&p.adduct);
    o.raw(",\"mz\":").num(round(p.mz, 4)).raw(",\"rt\":");
    o.opt(d.parent_rt.map(|r| round(r, 2)));
    o.raw(",\"name\":").str(&p.name).raw("},\"files\":");
    files_json(&mut o, d);
    o.raw(",\"times\":")
        .floats(&d.files.iter().map(|f| f.t).collect::<Vec<_>>());
    o.raw(",\"sessions\":{\"labels\":");
    strs(&mut o, &vec!["S1"; d.files.len()]);
    o.raw(",\"factors\":{\"applied\":false,\"factors\":")
        .floats(&vec![1.0; d.files.len()])
        .raw("}}");
    o.raw(",\"saturation\":{\"saturated\":false,\"note\":\"\"},\"rt_align\":");
    rt_align(&mut o, d);
    o.raw(",\"funnel\":[");
    for (i, (n, t)) in d.funnel.iter().enumerate() {
        o.raw(if i > 0 { ",{\"n\":" } else { "{\"n\":" })
            .int(*n as i64)
            .raw(",\"text\":")
            .str(t)
            .raw("}");
    }
    let rows = &d.cands[..d.cands.len().min(MAX_ROWS)];
    o.raw("],\"n_candidates\":")
        .int(d.cands.len() as i64)
        .raw(",\"n_rows\":")
        .int(rows.len() as i64)
        .raw(",\"rows\":[");
    for (i, c) in rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        row(&mut o, c, d.files.len());
    }
    o.raw("],\"mass_network\":")
        .raw(network)
        .raw(",\"isf_hidden\":[");
    isf_json(&mut o, d);
    o.raw("],\"tree\":null,\"calibration\":null,\"inclusion\":[");
    inclusion_json(&mut o, d);
    o.raw("],\"timing\":")
        .raw(if timing.trim().is_empty() {
            "{}"
        } else {
            timing
        })
        .raw(",\"warnings\":");
    strs(&mut o, &d.warnings);
    o.raw(",\"settings\":")
        .raw(if settings.trim().is_empty() {
            "{}"
        } else {
            settings
        })
        .raw(",\"weights\":{");
    for (i, (_, k, _)) in KINDS.iter().enumerate() {
        o.raw(if i > 0 { ",\"" } else { "\"" })
            .raw(k)
            .raw("\":")
            .num(d.weights[i]);
    }
    o.raw("}}");
    o.0
}

mod parts;
use parts::{criteria, isotopes, prediction, spectrum};

/// `api.detail(id)`: `Err` when the row is not a candidate.
pub fn detail(d: &Dig, id: i64) -> Result<String, String> {
    let c = d
        .cands
        .iter()
        .find(|c| c.id() == id)
        .ok_or("candidate not found")?;
    let mut o = Out::default();
    let lv = c.level();
    o.raw("{\"id\":").int(id).raw(",\"row\":");
    row(&mut o, c, d.files.len());
    o.raw(",\"criteria\":");
    criteria(&mut o, c);
    o.raw(",\"level_text\":")
        .str(LEVEL_TEXT[usize::from(lv - 4)])
        .raw(",\"components\":");
    components(&mut o, c, true);
    o.raw(",\"times\":")
        .floats(&d.files.iter().map(|f| f.t).collect::<Vec<_>>())
        .raw(",\"areas\":")
        .floats(&c.areas);
    o.raw(",\"sessions\":");
    strs(&mut o, &vec!["S1"; d.files.len()]);
    o.raw(",\"kinetics\":");
    match &c.kin {
        Some((k, absent)) => kinetics::detail_json(&mut o, k, *absent),
        None => {
            o.raw("{}");
        }
    }
    o.raw(",\"iimn\":null,\"evidence_family\":[],\"predecessor\":null,\"isotopes\":");
    isotopes(&mut o, c);
    o.raw(",\"prediction\":");
    prediction(&mut o, c);
    o.raw(",\"similarity\":");
    match &c.ms2 {
        Some(m) => m.sim.json(&mut o),
        None => {
            o.raw("null");
        }
    }
    if let (Some(m), Some(p)) = (&c.ms2, &d.parent_ms2) {
        o.raw(",\"ms2\":{\"candidate\":");
        spectrum(&mut o, &m.cons.mz, &m.cons.rel, m.cons.n_scans);
        o.raw(",\"parent\":");
        spectrum(&mut o, &p.mz, &p.rel, p.n_scans);
        o.raw(",\"evidence\":[]}");
    }
    o.raw(",\"localization\":null}");
    Ok(o.0)
}

/// `str(float)` of Python for the numbers of the list (`5.0`, `254.0597`).
fn num(v: f64) -> String {
    let s = format!("{v}");
    if s.contains(['.', 'e', 'N', 'i']) {
        s
    } else {
        s + ".0"
    }
}

/// `inclusion_csv(n)`: m/z, charge, start and end RT, note.
pub fn inclusion_csv(d: &Dig, n: usize) -> String {
    let mut lines = vec!["m/z,charge,RT_start_min,RT_end_min,note".to_string()];
    for c in inclusion(d)
        .into_iter()
        .take(if n == 0 { INCLUSION } else { n })
    {
        let (a, b) = rt_span(c);
        lines.push(format!(
            "{},1,{},{},\"{} priority {}\"",
            num(round(c.mz, 4)),
            num(round(a, 2)),
            num(round(b, 2)),
            c.formula.as_deref().unwrap_or(""),
            num(round(c.score, 1))
        ));
    }
    lines.join("\n")
}
