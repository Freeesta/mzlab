//! The table of the products (`tptab/1`): for every row the view `tp/1` keeps (formed products, formed and not explained, the parent) the numbers a feature
//! table shows next to the status: m/z, RT, the largest area and height, the time of the maximum, the kinetics as areas along the treatment, and whether the
//! ion is in the t0 and in the blank. It reads the `.mzfeat` and the metadata of the samples and nothing else; it computes no identity.
//!
//! The role of a sample (treated, t0, blank) is read in ONE place, `role_of`: a change of the model of the roles touches only that function.

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
    o.opt((has_block && t.tp.parent_row != NO_PARENT).then(|| f64::from(t.id(t.tp.parent_row as usize))));
    o.raw(",\"times\":");
    o.floats(&treated.iter().map(|x| x.0).collect::<Vec<_>>());
    o.raw(",\"controls\":{\"t0\":")
        .int(t0.len() as i64)
        .raw(",\"blank\":")
        .int(blank.len() as i64)
        .raw("},\"rows\":[");
    let mut first = true;
    let all: Vec<usize>;
    let list = match rows {
        Some(r) => r,
        None => {
            all = (0..t.len()).filter(|&r| matches!(t.tp.status[r], ST_TP | ST_UNEXPLAINED | ST_PARENT)).collect();
            &all
        }
    };
    for &r in list {
        let series: Vec<f64> = tr_files
            .iter()
            .map(|&k| round(measured(t, r, k), 1))
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
            if t.state[c] == CELL_ABSENT {
                m
            } else {
                m.max(f64::from(t.height[c]))
            }
        });
        let (m_t0, m_bl) = (largest(t, r, &t0), largest(t, r, &blank));
        let n_det = (0..t.n_files).filter(|&k| measured(t, r, k) > 0.0).count();
        o.raw(if first { "{" } else { ",{" });
        first = false;
        o.raw("\"row\":")
            .int(r as i64)
            .raw(",\"id\":")
            .int(i64::from(t.id(r)))
            .raw(",\"mz\":")
            .num(round(t.mz[r], 6))
            .raw(",\"rt\":")
            .num(round(t.rt[r], 3));
        let block = |v: &[u8]| has_block.then(|| f64::from(v[r]));
        o.raw(",\"status\":").opt(block(&t.tp.status)).raw(",\"kinship\":").opt(block(&t.tp.kinship));
        o.raw(",\"via\":").opt(block(&t.tp.via)).raw(",\"notes\":").opt(block(&t.tp.notes));
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
    o.raw("]}");
    Ok(o.0)
}
