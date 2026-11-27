//! `feature-slots/1`: the fixed slots of the feature view (structure, peak/XIC, kinetics, MS2, isotopes/adducts, UV-Vis, SIM) of one row. Each slot is `filled`
//! (the table holds evidence, with its source), `empty` (nothing yet: the step or command that fills it is named) or `na` (the table cannot hold it).
//! Facts only: a slot without evidence has `evidence: null`, never a made-up number. The same bytes on the site, in the app and from the CLI.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;

use crate::aligned::{
    no_peak, Aligned, CELL_DETECTED, CELL_FILLED, CELL_NA, CELL_TARGETED, NO_FILE,
};
use crate::norm::measured;
use crate::rowident::adduct_names;
use crate::samples::Sample;

pub const SCHEMA: &str = "feature-slots/1";

/// The slots in the order of the view, and what fills each: `(slot, step, command)`. Every step id is a Flow step and every command a `mzlab-cli` command
/// (a test of the CLI checks both against the catalogs).
pub const SLOTS: [(&str, Option<&str>, Option<&str>); 7] = [
    ("structure", Some("tp.generate"), None),
    ("xic", Some("quant.targets"), Some("trace xic")),
    ("kinetics", None, Some("sample table")),
    ("ms2", Some("features.ms2_link"), Some("spectrum list")),
    ("isotopes", Some("features.group"), Some("ion pattern")),
    ("uv", None, Some("dad spectrum")),
    ("sim", Some("chrom.transitions"), None),
];

/// The step that writes the evidence of a filled slot (by kind: the `.mzfeat` records its recipe, not one step per column).
fn writer(slot: &str) -> Option<&'static str> {
    match slot {
        "structure" => Some("tp.generate"),
        "xic" | "kinetics" => Some("features.align"),
        "ms2" => Some("features.ms2_link"),
        "isotopes" => Some("features.group"),
        "sim" => Some("chrom.transitions"),
        _ => None,
    }
}

fn fin(v: f32) -> Option<f64> {
    Some(f64::from(v)).filter(|x| x.is_finite())
}

/// What one slot says: its state, why when it is `na`, the evidence (JSON text) and the files behind it.
struct Slot<'a> {
    state: &'static str,
    reason: Option<&'static str>,
    evidence: Option<String>,
    files: Vec<&'a Sample>,
}

impl Slot<'_> {
    fn empty() -> Self {
        Slot {
            state: "empty",
            reason: None,
            evidence: None,
            files: Vec::new(),
        }
    }

    fn na(reason: &'static str) -> Self {
        Slot {
            state: "na",
            reason: Some(reason),
            ..Slot::empty()
        }
    }
}

fn structure(t: &Aligned, r: usize) -> Slot<'static> {
    if t.ident.is_empty() {
        return Slot::na("no_identity");
    }
    let src = t.ident.src[r];
    if src >> 1 == 0 {
        return Slot::empty();
    }
    let mut o = Out::default();
    o.raw("{\"neutral\":")
        .num(t.ident.neutral[r])
        .raw(",\"adduct\":");
    match adduct_names().get(usize::from(t.ident.ion[r]).wrapping_sub(1)) {
        Some(a) => o.str(a),
        None => o.raw("null"),
    };
    o.raw(",\"sources\":[");
    for (i, w) in crate::row::hypothesis_words(src).iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(w);
    }
    o.raw("]}");
    Slot {
        state: "filled",
        reason: None,
        evidence: Some(o.0),
        files: Vec::new(),
    }
}

fn xic<'a>(t: &Aligned, s: &'a [Sample], r: usize) -> Slot<'a> {
    let mut n = [0usize; 5];
    let mut files = Vec::new();
    for (k, smp) in s.iter().enumerate() {
        let c = t.cell(r, k);
        let measured = measured(t, r, k).is_some();
        let i = match t.state[c] {
            _ if !measured && t.state[c] != CELL_NA => 3,
            CELL_DETECTED => 0,
            CELL_FILLED => 1,
            CELL_TARGETED => 2,
            CELL_NA => 4,
            _ => 3,
        };
        n[i] += 1;
        if measured && i < 3 {
            files.push(smp);
        }
    }
    if !s.is_empty() && n[4] == s.len() {
        return Slot::na("files_do_not_cover");
    }
    if files.is_empty() {
        return Slot::empty();
    }
    let q = (!t.meta.is_empty()).then(|| t.meta.get(r));
    let mut o = Out::default();
    o.raw("{\"files\":")
        .int(s.len() as i64)
        .raw(",\"detected\":")
        .int(n[0] as i64)
        .raw(",\"filled\":")
        .int(n[1] as i64)
        .raw(",\"targeted\":")
        .int(n[2] as i64)
        .raw(",\"absent\":")
        .int(n[3] as i64)
        .raw(",\"na\":")
        .int(n[4] as i64);
    o.raw(",\"rt_lo\":")
        .opt(q.and_then(|q| fin(q.rt_lo)))
        .raw(",\"rt_hi\":")
        .opt(q.and_then(|q| fin(q.rt_hi)))
        .raw("}");
    Slot {
        state: "filled",
        reason: None,
        evidence: Some(o.0),
        files,
    }
}

/// Kinetics needs a time axis: two or more distinct times with a measured area.
fn kinetics<'a>(t: &Aligned, s: &'a [Sample], r: usize) -> Slot<'a> {
    let mut times: Vec<f64> = s.iter().filter_map(|x| x.time_min).collect();
    times.sort_by(f64::total_cmp);
    times.dedup();
    if times.len() < 2 {
        return Slot::na("no_time_axis");
    }
    let timed: Vec<(&Sample, f64)> = s
        .iter()
        .enumerate()
        .filter(|(k, _)| measured(t, r, *k).is_some() && !no_peak(t.state[t.cell(r, *k)]))
        .filter_map(|(_, x)| x.time_min.map(|m| (x, m)))
        .collect();
    let mut seen: Vec<f64> = timed.iter().map(|(_, m)| *m).collect();
    seen.sort_by(f64::total_cmp);
    seen.dedup();
    if seen.len() < 2 {
        return Slot::empty();
    }
    let mut o = Out::default();
    o.raw("{\"points\":")
        .int(timed.len() as i64)
        .raw(",\"times_min\":");
    o.floats(&seen);
    o.raw("}");
    Slot {
        state: "filled",
        reason: None,
        evidence: Some(o.0),
        files: timed.into_iter().map(|(x, _)| x).collect(),
    }
}

fn ms2<'a>(t: &Aligned, s: &'a [Sample], r: usize) -> Slot<'a> {
    let q = (!t.meta.is_empty()).then(|| t.meta.get(r));
    match q.filter(|q| q.ms2_file != NO_FILE && (q.ms2_file as usize) < s.len()) {
        Some(q) => {
            let file = &s[q.ms2_file as usize];
            let mut o = Out::default();
            o.raw("{\"file\":")
                .str(&file.file)
                .raw(",\"purity\":")
                .opt(fin(q.ms2_purity).filter(|p| *p >= 0.0))
                .raw("}");
            Slot {
                state: "filled",
                reason: None,
                evidence: Some(o.0),
                files: vec![file],
            }
        }
        None => Slot::empty(),
    }
}

fn isotopes(t: &Aligned, r: usize) -> Slot<'static> {
    let q = (!t.meta.is_empty()).then(|| t.meta.get(r));
    let m1 = q.and_then(|q| fin(q.iso_m1));
    let adduct = (!t.ident.is_empty())
        .then(|| {
            adduct_names()
                .get(usize::from(t.ident.ion[r]).wrapping_sub(1))
                .copied()
        })
        .flatten();
    let family = (!t.member_start.is_empty())
        .then(|| t.family_size(r))
        .filter(|n| *n > 1);
    if m1.is_none() && adduct.is_none() && family.is_none() {
        return Slot::empty();
    }
    let mut o = Out::default();
    o.raw("{\"iso_m1\":").opt(m1).raw(",\"adduct\":");
    match adduct {
        Some(a) => o.str(a),
        None => o.raw("null"),
    };
    o.raw(",\"family_size\":")
        .opt(family.map(|n| n as f64))
        .raw("}");
    Slot {
        state: "filled",
        reason: None,
        evidence: Some(o.0),
        files: Vec::new(),
    }
}

/// A row of a transition table (MRM/SIM) has a Q3; any other row has no SIM trace of its own.
fn sim(t: &Aligned, r: usize) -> Slot<'static> {
    match t.q3.get(r) {
        Some(q3) => {
            let mut o = Out::default();
            o.raw("{\"q1\":")
                .num(t.mz[r])
                .raw(",\"q3\":")
                .num(*q3)
                .raw("}");
            Slot {
                state: "filled",
                reason: None,
                evidence: Some(o.0),
                files: Vec::new(),
            }
        }
        None => Slot::na("not_a_transition_table"),
    }
}

fn write_files(o: &mut Out, files: &[&Sample]) {
    o.raw("[");
    for (i, f) in files.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(&f.file)
            .raw(",\"sha256\":")
            .str(&f.sha256)
            .raw("}");
    }
    o.raw("]");
}

/// The `feature-slots/1` of the row with `id`. `output` names the table the row is read from (`null` when empty).
pub fn slots(t: &Aligned, s: &[Sample], id: u32, output: &str) -> Result<String> {
    if s.len() != t.n_files {
        return Err(Error::new("error.badView"));
    }
    let r = t
        .row_of_id(id)
        .ok_or_else(|| Error::new("error.rowNotFound").with("id", id))?;
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(SCHEMA)
        .raw(",\"id\":")
        .int(i64::from(id))
        .raw(",\"mz\":")
        .num(t.mz[r])
        .raw(",\"rt\":")
        .num(t.rt[r])
        .raw(",\"output\":");
    if output.is_empty() {
        o.raw("null");
    } else {
        o.str(output);
    }
    o.raw(",\"slots\":[");
    for (i, (name, step, command)) in SLOTS.iter().enumerate() {
        let slot = match *name {
            "structure" => structure(t, r),
            "xic" => xic(t, s, r),
            "kinetics" => kinetics(t, s, r),
            "ms2" => ms2(t, s, r),
            "isotopes" => isotopes(t, r),
            "sim" => sim(t, r),
            _ => Slot::empty(),
        };
        o.raw(if i > 0 { ",{\"slot\":" } else { "{\"slot\":" })
            .str(name)
            .raw(",\"state\":")
            .str(slot.state)
            .raw(",\"reason\":");
        match slot.reason {
            Some(w) => o.str(w),
            None => o.raw("null"),
        };
        o.raw(",\"evidence\":")
            .raw(slot.evidence.as_deref().unwrap_or("null"))
            .raw(",\"source\":");
        if slot.state == "filled" {
            o.raw("{\"output\":");
            if output.is_empty() {
                o.raw("null");
            } else {
                o.str(output);
            }
            o.raw(",\"step\":");
            match writer(name) {
                Some(w) => o.str(w),
                None => o.raw("null"),
            };
            o.raw(",\"files\":");
            write_files(&mut o, &slot.files);
            o.raw("}").raw(",\"fill\":null");
        } else {
            o.raw("null").raw(",\"fill\":{\"step\":");
            match step {
                Some(w) => o.str(w),
                None => o.raw("null"),
            };
            o.raw(",\"command\":");
            match command {
                Some(w) => o.str(&format!("mzlab-cli {w}")),
                None => o.raw("null"),
            };
            o.raw("}");
        }
        o.raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}
