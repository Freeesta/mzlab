//! The checks of `tabcheck/1` on the shape of the cells: long traces, cells missing in a group, polarity and units.

use super::*;
use crate::aligned::{CELL_ABSENT, CELL_DETECTED};

/// A trace this many times wider than the median is long; the share of the rows that warns.
const LONG_FACTOR: f64 = 5.0;
const LONG_SHARE: f64 = 0.05;
/// Missing cells concentrated in a group: its share at least this and at least twice the one of all the others.
const MISSING_GROUP: f64 = 0.3;
/// Beyond these an RT or a width is read as seconds, not minutes.
const RT_MAX_MIN: f64 = 240.0;
const WIDTH_MAX_MIN: f64 = 3.0;

/// Median width of the detected peaks of each row; rows far wider than the median of all the rows (bleed of the column, a contaminant).
pub(super) fn long_traces(e: &Env) -> Check {
    let width = |r: usize| -> f64 {
        median(
            (0..e.t.n_files)
                .filter(|&k| e.t.state[r * e.t.n_files + k] == CELL_DETECTED)
                .map(|k| f64::from(e.t.width[r * e.t.n_files + k]))
                .filter(|w| w.is_finite() && *w > 0.0)
                .collect(),
        )
    };
    let w: Vec<(usize, f64)> = e
        .rows
        .iter()
        .map(|&r| (r, width(r)))
        .filter(|x| x.1.is_finite())
        .collect();
    if w.is_empty() {
        return Check::missing("long_traces", "widths");
    }
    let med = median(w.iter().map(|x| x.1).collect());
    let mut long: Vec<(usize, f64)> = w
        .iter()
        .copied()
        .filter(|x| x.1 > LONG_FACTOR * med)
        .collect();
    rank(&mut long, true);
    let share = long.len() as f64 / w.len() as f64;
    let mut p = Proof::new();
    p.n("median_width_min", med)
        .n("factor", LONG_FACTOR)
        .i("long_rows", long.len())
        .i("rows", w.len());
    let flag = if share > LONG_SHARE {
        Flag::Warn
    } else if long.is_empty() {
        Flag::Ok
    } else {
        Flag::Note
    };
    Check {
        id: "long_traces",
        value: Some(share),
        threshold: Some(LONG_SHARE),
        flag,
        rows: e.ids(&long.iter().map(|x| x.0).collect::<Vec<_>>()),
        files: Vec::new(),
        proof: p.done(),
    }
}

/// JSON list of the groups with their kind, group name, file count and share of missing cells, cut to the example cap.
fn groups_list(e: &Env, keys: &[(String, String)], groups: &[(usize, f64, usize)]) -> String {
    let mut list = String::from("[");
    for (i, g) in groups.iter().take(e.caps.pairs.max(2) * 2).enumerate() {
        let mut o = Out::default();
        o.raw(if i > 0 { ",{\"kind\":" } else { "{\"kind\":" })
            .str(&keys[g.0].0)
            .raw(",\"group\":")
            .str(&keys[g.0].1)
            .raw(",\"files\":")
            .int(g.2 as i64);
        o.raw(",\"share\":").num(round(g.1, 5)).raw("}");
        list.push_str(&o.0);
    }
    list.push(']');
    list
}

/// Share of cells without a peak in each group of files (type and group), blanks apart; a group with far more than the others lost its peaks, not at random.
pub(super) fn missing_cells(e: &Env) -> Check {
    let pool: Vec<usize> = e
        .live
        .iter()
        .copied()
        .filter(|k| !e.blanks.contains(k))
        .collect();
    let mut keys: Vec<(String, String)> = Vec::new();
    for &k in &pool {
        let key = (e.s[k].kind.clone(), e.s[k].group.clone());
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    if keys.len() < 2 {
        return Check::missing("missing", "groups");
    }
    let mut groups: Vec<(usize, f64, usize)> = Vec::new();
    let mut total_missing = 0;
    for (g, key) in keys.iter().enumerate() {
        let files: Vec<usize> = pool
            .iter()
            .copied()
            .filter(|&k| (e.s[k].kind.clone(), e.s[k].group.clone()) == *key)
            .collect();
        let absent = e
            .rows
            .iter()
            .map(|&r| {
                files
                    .iter()
                    .filter(|&&k| e.t.state[r * e.t.n_files + k] == CELL_ABSENT)
                    .count()
            })
            .sum::<usize>();
        total_missing += absent;
        groups.push((
            g,
            absent as f64 / (e.rows.len() * files.len()) as f64,
            files.len(),
        ));
    }
    let cells = e.rows.len() * pool.len();
    let (worst, share) = groups
        .iter()
        .fold((0, -1.0), |b, g| if g.1 > b.1 { (g.0, g.1) } else { b });
    let worst_cells = e.rows.len() * groups[worst].2;
    let rest = (total_missing as f64 - share * worst_cells as f64) / (cells - worst_cells) as f64;
    let list = groups_list(e, &keys, &groups);
    let mut p = Proof::new();
    p.n("overall_share", total_missing as f64 / cells as f64)
        .n("others_share", rest)
        .raw("groups", &list);
    let concentrated = share >= MISSING_GROUP && share >= 2.0 * rest;
    let files: Vec<usize> = pool
        .iter()
        .copied()
        .filter(|&k| (e.s[k].kind.clone(), e.s[k].group.clone()) == keys[worst])
        .collect();
    Check {
        id: "missing",
        value: Some(share),
        threshold: Some(MISSING_GROUP),
        flag: if concentrated { Flag::Warn } else { Flag::Ok },
        rows: Vec::new(),
        files: if concentrated {
            e.names(&files)
        } else {
            Vec::new()
        },
        proof: p.done(),
    }
}

/// The polarity the table was made with against the one found in the files, and values that do not read as m/z, minutes and areas.
pub(super) fn polarity_units(e: &Env) -> Check {
    let mut issues: Vec<&str> = Vec::new();
    if e.opts
        .expected_polarity
        .is_some_and(|x| (x < 0) != (e.opts.polarity < 0))
    {
        issues.push("polarity_mismatch");
    }
    if e.rows
        .iter()
        .any(|&r| !(e.t.mz[r].is_finite() && e.t.mz[r] > 0.0))
    {
        issues.push("mz_not_positive");
    }
    let max_rt = e.rows.iter().map(|&r| e.t.rt[r]).fold(0.0, f64::max);
    let widths: Vec<f64> = e
        .rows
        .iter()
        .flat_map(|&r| (0..e.t.n_files).map(move |k| r * e.t.n_files + k))
        .filter(|&c| e.t.state[c] == CELL_DETECTED)
        .map(|c| f64::from(e.t.width[c]))
        .filter(|w| w.is_finite() && *w > 0.0)
        .collect();
    let median_width = median(widths);
    if max_rt > RT_MAX_MIN || median_width > WIDTH_MAX_MIN {
        issues.push("rt_not_minutes");
    }
    if e.rows.iter().any(|&r| {
        (0..e.t.n_files).any(|k| {
            e.t.state[r * e.t.n_files + k] != CELL_ABSENT
                && !(e.t.area[r * e.t.n_files + k].is_finite()
                    && e.t.area[r * e.t.n_files + k] >= 0.0)
        })
    }) {
        issues.push("area_not_valid");
    }
    let mut list = String::from("[");
    for (i, s) in issues.iter().enumerate() {
        list.push_str(if i > 0 { ",\"" } else { "\"" });
        list.push_str(s);
        list.push('"');
    }
    list.push(']');
    let mut p = Proof::new();
    p.raw("issues", &list).n("max_rt", max_rt);
    if let Some(x) = e.opts.expected_polarity {
        p.s(
            "expected_polarity",
            if x < 0 { "negative" } else { "positive" },
        );
    }
    p.s(
        "table_polarity",
        if e.opts.polarity < 0 {
            "negative"
        } else {
            "positive"
        },
    );
    Check {
        id: "polarity_units",
        value: Some(issues.len() as f64),
        threshold: Some(1.0),
        flag: if issues.is_empty() {
            Flag::Ok
        } else {
            Flag::Warn
        },
        rows: Vec::new(),
        files: Vec::new(),
        proof: p.done(),
    }
}
