//! The block `gruppi` of the summary (`gruppi/1`, `docs/agenti/gruppi.md`) for a series that compares conditions: the rows of the Dig on the cells of
//! the ONE table of the pipeline (no second peak search, no new alignment), the roles of the columns as `campioni/2` and the options of the settings.
//! A series with fewer than two conditions has no block.

use crate::sortc::SortC;
use mzlab_core::campioni::Campione;

use super::super::series::FileIn;
use super::{cond, Entry, LrDig};
use crate::gruppi::{self, GroupTable, RowMeta};
use crate::table::CELL_ABSENT;

/// Mass of the proton (u): the neutral mass of a row is its m/z minus it in ESI+ and plus it in ESI-.
const PROTON: f64 = 1.007_276;

/// A column of the Dig as a sample of `campioni/2` (role from the kind and the time, condition and replicate as the page gave them).
pub fn campione(f: &FileIn, rank: usize) -> Campione {
    let kind = match f.kind.as_str() {
        "blank" => "blank",
        "control" => "dark",
        "standard" => "std",
        _ => "sample",
    };
    let order = if f.order > 0 {
        f.order
    } else {
        rank as u32 + 1
    };
    let mut c = Campione::new(&f.name, "", order);
    c.kind = kind.into();
    c.set_time(f.time, "min");
    c.series = f.cond.clone();
    c.replicate = f.replica.unwrap_or(1);
    c
}

/// The columns that carry the MS2 of the experiment file `name`: the samples of the same condition and time. A file with no condition is shared:
/// it counts for every condition (at its time when it has one).
fn ms2_columns(d: &LrDig, name: &str) -> Vec<usize> {
    let Some(f) = d.files.iter().find(|f| f.name == name) else {
        return Vec::new();
    };
    (0..d.lc.len())
        .filter(|&k| {
            let c = &d.lc[k];
            if c.kind != "sample" {
                return false;
            }
            if f.cond.is_empty() {
                return f.time.is_none() || c.time == f.time;
            }
            c.cond == f.cond && c.time.is_some() && c.time == f.time
        })
        .collect()
}

/// The row of the opposite polarity with the same neutral mass (within `tol_da`) and RT (within `rt_tol` min), by id; the rows stay separate.
pub fn other_polarity(rows: &mut [RowMeta], tol_da: f64, rt_tol: f64) {
    let neutral = |r: &RowMeta| r.mz - if r.polarity == "-" { -PROTON } else { PROTON };
    let ids: Vec<Option<u32>> = rows
        .iter()
        .map(|a| {
            rows.iter()
                .find(|b| {
                    b.polarity != a.polarity
                        && (neutral(a) - neutral(b)).abs() <= tol_da
                        && (a.rt - b.rt).abs() <= rt_tol
                })
                .map(|b| b.id)
        })
        .collect();
    for (r, id) in rows.iter_mut().zip(ids) {
        r.other_polarity = id;
    }
}

fn meta(d: &LrDig, e: &Entry, name: &str) -> RowMeta {
    let mut ms2: Vec<usize> = d
        .links
        .as_ref()
        .map(|l| {
            l.of(e.id)
                .flat_map(|(_, k)| ms2_columns(d, &l.exps[k.exp].file))
                .collect()
        })
        .unwrap_or_default();
    ms2.sortc();
    ms2.dedup();
    RowMeta {
        id: e.id as u32,
        tp_name: name.to_string(),
        // rounded as the summary shows it: the alignment sums in the order of the files, the last digits would differ
        mz: mzlab_core::json::round(e.observed.unwrap_or(e.mz), 4),
        polarity: "+".into(),
        rt: e.ref_rt.unwrap_or(0.0),
        is_parent: e.id == 0,
        ms2_files: ms2,
        ..RowMeta::default()
    }
}

/// `gruppi/1` of the rows shown (`found`, with their TP names), `None` with fewer than two conditions or when the module refuses the input.
pub fn block(d: &LrDig, names: &[String], found: &[bool]) -> Option<String> {
    if cond::conditions(&d.lc).len() < 2 {
        return None;
    }
    // the order of the conditions is the one of the injections; without it, the order of the names (never the position in the list)
    let mut by_name: Vec<&str> = d.lc.iter().map(|f| f.name.as_str()).collect();
    by_name.sortc();
    let rank = |f: &FileIn| by_name.iter().position(|n| *n == f.name).unwrap_or(0);
    let cs: Vec<Campione> = d.lc.iter().map(|f| campione(f, rank(f))).collect();
    let shown: Vec<usize> = (0..d.entries.len()).filter(|&i| found[i]).collect();
    let n = d.lc.len();
    let mut tb = GroupTable {
        n_rows: shown.len(),
        n_files: n,
        ..GroupTable::default()
    };
    for &i in &shown {
        let e = &d.entries[i];
        for k in 0..n {
            let c = e.row.map(|r| d.table.cell(r, k));
            let ok = |v: f64| if v.is_finite() { v } else { 0.0 };
            tb.area.push(c.map_or(0.0, |c| ok(d.table.area[c])));
            tb.height
                .push(c.map_or(0.0, |c| ok(f64::from(d.table.height[c]))));
            tb.state.push(c.map_or(CELL_ABSENT, |c| d.table.state[c]));
            tb.apex_rt.push(c.map_or(f32::NAN, |c| d.table.apex_rt[c]));
        }
        tb.rows.push(meta(d, e, &names[i]));
    }
    other_polarity(&mut tb.rows, d.settings.tol_da, d.settings.rt_tol_min);
    gruppi::gruppi_json(&tb, &cs, &d.settings.groups).ok()
}
