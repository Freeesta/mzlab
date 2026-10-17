//! The table of the products (`tptab/1`): for every row the view `tp/1` keeps (formed products, formed and not explained, the parent) the numbers a feature
//! table shows next to the status: m/z, RT, the largest area and height, the time of the maximum, the kinetics as areas along the treatment, and whether the
//! ion is in the t0 and in the blank. It reads the `.mzfeat` and the metadata of the samples and nothing else; it computes no identity.
//!
//! The role of a sample (treated, t0, blank) is read in ONE place, `role_of`: a change of the model of the roles touches only that function.

use crate::sortc::SortC;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{round, Out};

use super::*;
use crate::aligned::CELL_ABSENT;
use crate::samples::Sample;

pub const TABLE_SCHEMA: &str = "tptab/1";

/// What a sample is for the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Role {
    /// Treated for this many minutes (> 0).
    Treated(f64),
    T0,
    /// A control that must not show the product: blank or dark.
    Blank,
    /// Not used (standards, QC, calibration, a sample with no time).
    Other,
}

/// The role of a sample: the only reader of `Sample::kind` and `Sample::time_min` for this table.
pub fn role_of(s: &Sample) -> Role {
    match s.kind.as_str() {
        "t0" => Role::T0,
        "blank" | "dark" => Role::Blank,
        "sample" => match s.time_min {
            Some(t) if t > 0.0 => Role::Treated(t),
            Some(_) => Role::T0,
            None => Role::Other,
        },
        _ => Role::Other,
    }
}

/// The area of the cell when the file has a peak there.
fn measured(t: &Aligned, row: usize, file: usize) -> f64 {
    let c = t.cell(row, file);
    if t.state[c] != CELL_ABSENT && t.area[c].is_finite() && t.area[c] > 0.0 {
        t.area[c]
    } else {
        0.0
    }
}

fn largest(t: &Aligned, row: usize, files: &[usize]) -> f64 {
    files.iter().fold(0.0, |m, &k| m.max(measured(t, row, k)))
}

/// `tptab/1` of a table under the samples `s` (one per file). With `rows` (indices of the table) those rows, with the status of the block `tp/1` when the table
/// has one (`null` otherwise); without, the rows of status product, formed and not explained, or parent, in the order of the table, and a table with no
/// block is `error.tpNoBlock`. A row out of the table is `error.badView`.
pub fn table_json(t: &Aligned, s: &[Sample], rows: Option<&[usize]>) -> Result<String> {
    table_json_cond(t, s, rows, None)
}

/// The rows of status product, formed and not explained, or parent, in the order of the table.
fn default_rows(t: &Aligned) -> Vec<usize> {
    (0..t.len())
        .filter(|&r| matches!(t.tp.status[r], ST_TP | ST_UNEXPLAINED | ST_PARENT))
        .collect()
}

/// The conditions of a table: the files each one uses and the controls that apply to it.
struct CondCx {
    id: String,
    /// The treated files of the condition, per file of the table.
    keep: Vec<bool>,
    t0: Vec<usize>,
    blank: Vec<usize>,
}

/// `table_json` with the metadata `campioni/2` of the files and what the caller knows of the best hypothesis of each row. With two or more conditions
/// `times` stays the time axis of all the treated files; a row's `series`, `in_t0` and `in_blank` are the ones of the condition where the row is strongest
/// (the files of the others count 0 in `series`, ties go to the first condition), and `gruppi` (`gruppi/1`) holds the detail of every condition together
/// with `network` (the times of each condition and whether the network read it). With fewer than two conditions the text is the one of `table_json`.
pub fn table_json_cond(
    t: &Aligned,
    s: &[Sample],
    rows: Option<&[usize]>,
    cond: Option<(&[mzlab_core::campioni::Campione], &[super::RowInfo])>,
) -> Result<String> {
    let has_block = !t.tp.is_empty() && t.tp.len() == t.len();
    if rows.is_none() && !has_block {
        return Err(Error::new("error.tpNoBlock"));
    }
    if rows.is_some_and(|r| r.iter().any(|&x| x >= t.len())) {
        return Err(Error::new("error.badView"));
    }
    let roles: Vec<Role> = s.iter().map(role_of).collect();
    let mut treated: Vec<(f64, usize)> = roles
        .iter()
        .enumerate()
        .filter_map(|(k, r)| {
            if let Role::Treated(m) = *r {
                Some((m, k))
            } else {
                None
            }
        })
        .collect();
    crate::sortc::by(&mut treated, |a, b| a.0.total_cmp(&b.0));
    let files_of =
        |want: Role| -> Vec<usize> { (0..roles.len()).filter(|&k| roles[k] == want).collect() };
    let (t0, blank) = (files_of(Role::T0), files_of(Role::Blank));
    let tr_files: Vec<usize> = treated.iter().map(|x| x.1).collect();
    let mut o = Out::default();
    o.raw("{\"schema\":").str(TABLE_SCHEMA).raw(",\"parent\":");
    o.opt(
        (has_block && t.tp.parent_row != NO_PARENT)
            .then(|| f64::from(t.id(t.tp.parent_row as usize))),
    );
    o.raw(",\"times\":");
    o.floats(&treated.iter().map(|x| x.0).collect::<Vec<_>>());
    o.raw(",\"controls\":{\"t0\":")
        .int(t0.len() as i64)
        .raw(",\"blank\":")
        .int(blank.len() as i64)
        .raw("},\"rows\":[");
    let mut first = true;
    let all = rows.is_none().then(|| default_rows(t));
    let list = rows.or(all.as_deref()).unwrap_or(&[]);
    let conds = match cond {
        Some((cs, _)) if cs.len() == t.n_files => cond_cx(cs, &roles),
        Some(_) => return Err(Error::new("error.badView")),
        None => Vec::new(),
    };
    for &r in list {
        let mut cx = Cx {
            has_block,
            treated: &treated,
            tr_files: &tr_files,
            t0: &t0,
            blank: &blank,
            keep: None,
        };
        if let Some(c) = strongest(t, r, &conds) {
            (cx.keep, cx.t0, cx.blank) = (Some(c.keep.as_slice()), &c.t0, &c.blank);
        }
        row_json(&mut o, t, r, &cx, first);
        first = false;
    }
    o.raw("]");
    if let Some((cs, infos)) = cond.filter(|_| !conds.is_empty()) {
        let g = super::groups_json(t, cs, list, infos)?;
        o.raw(",\"gruppi\":").raw(g.as_deref().unwrap_or("null"));
        network_json(&mut o, &conds, &treated);
    }
    o.raw("}");
    Ok(o.0)
}

/// The condition of each file's role and controls (empty with fewer than two conditions).
fn cond_cx(cs: &[mzlab_core::campioni::Campione], roles: &[Role]) -> Vec<CondCx> {
    super::group_conditions(cs)
        .into_iter()
        .map(|c| {
            let mut keep = vec![false; roles.len()];
            for &f in &c.files {
                keep[f] = matches!(roles[f], Role::Treated(_));
            }
            let mut blank: Vec<usize> = c
                .controls
                .blank
                .iter()
                .chain(&c.controls.dark)
                .copied()
                .collect();
            blank.sortc();
            CondCx {
                id: c.id,
                keep,
                t0: c.controls.t0,
                blank,
            }
        })
        .collect()
}

/// The condition where the row has its largest area (the first on a tie), `None` without conditions.
fn strongest<'a>(t: &Aligned, row: usize, conds: &'a [CondCx]) -> Option<&'a CondCx> {
    let best = |c: &CondCx| {
        (0..c.keep.len())
            .filter(|&k| c.keep[k])
            .fold(0.0, |m, k| f64::max(m, measured(t, row, k)))
    };
    let mut top: Option<(&CondCx, f64)> = None;
    for c in conds {
        let a = best(c);
        if top.is_none_or(|(_, m)| a > m) {
            top = Some((c, a));
        }
    }
    top.map(|x| x.0)
}

/// `"network"`: the number of treated times of each condition and whether the network read its correlations (it needs at least `NetParams::points`).
fn network_json(o: &mut Out, conds: &[CondCx], treated: &[(f64, usize)]) {
    let need = super::NetParams::default().points.max(2);
    o.raw(",\"network\":{\"min_times\":")
        .int(need as i64)
        .raw(",\"conditions\":[");
    for (i, c) in conds.iter().enumerate() {
        let n = treated.iter().filter(|x| c.keep[x.1]).count();
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .str(&c.id);
        o.raw(",\"times\":")
            .int(n as i64)
            .raw(",\"used\":")
            .raw(if n >= need { "true}" } else { "false}" });
    }
    o.raw("]}");
}

/// What every row of the table shares: the block flag, the treated files in time order, and the t0 / blank files.
struct Cx<'a> {
    has_block: bool,
    treated: &'a [(f64, usize)],
    tr_files: &'a [usize],
    t0: &'a [usize],
    blank: &'a [usize],
    /// With conditions: the treated files of the condition the row is read in (the others count 0).
    keep: Option<&'a [bool]>,
}

fn row_json(o: &mut Out, t: &Aligned, r: usize, cx: &Cx, first: bool) {
    let Cx {
        has_block,
        treated,
        tr_files,
        t0,
        blank,
        keep,
    } = *cx;
    let kept = |k: usize| keep.is_none_or(|v| v[k]);
    let series: Vec<f64> = tr_files
        .iter()
        .map(|&k| {
            if kept(k) {
                round(measured(t, r, k), 1)
            } else {
                0.0
            }
        })
        .collect();
    let (mut max_a, mut tmax) = (0.0, None);
    for (i, &a) in series.iter().enumerate() {
        if a > max_a {
            max_a = a;
            tmax = Some(treated[i].0);
        }
    }
    let max_h = tr_files.iter().fold(0.0f64, |m, &k| {
        let c = t.cell(r, k);
        if t.state[c] == CELL_ABSENT || !kept(k) {
            m
        } else {
            m.max(f64::from(t.height[c]))
        }
    });
    let (m_t0, m_bl) = (largest(t, r, t0), largest(t, r, blank));
    let n_det = (0..t.n_files).filter(|&k| measured(t, r, k) > 0.0).count();
    o.raw(if first { "{" } else { ",{" });
    o.raw("\"row\":")
        .int(r as i64)
        .raw(",\"id\":")
        .int(i64::from(t.id(r)))
        .raw(",\"mz\":")
        .num(round(t.mz[r], 6))
        .raw(",\"rt\":")
        .num(round(t.rt[r], 3));
    let block = |v: &[u8]| has_block.then(|| f64::from(v[r]));
    o.raw(",\"status\":")
        .opt(block(&t.tp.status))
        .raw(",\"kinship\":")
        .opt(block(&t.tp.kinship));
    o.raw(",\"via\":")
        .opt(block(&t.tp.via))
        .raw(",\"notes\":")
        .opt(block(&t.tp.notes));
    o.raw(",\"n_det\":")
        .int(n_det as i64)
        .raw(",\"max_area\":")
        .num(max_a)
        .raw(",\"max_height\":")
        .num(round(max_h, 1));
    o.raw(",\"tmax\":").opt(tmax);
    o.raw(",\"in_t0\":")
        .raw(if m_t0 > 0.0 { "true" } else { "false" });
    o.raw(",\"in_blank\":")
        .raw(if m_bl > 0.0 { "true" } else { "false" });
    o.raw(",\"t0_ratio\":")
        .opt((max_a > 0.0).then(|| round(m_t0 / max_a, 4)));
    o.raw(",\"blank_ratio\":")
        .opt((max_a > 0.0).then(|| round(m_bl / max_a, 4)));
    o.raw(",\"series\":").floats(&series).raw("}");
}
