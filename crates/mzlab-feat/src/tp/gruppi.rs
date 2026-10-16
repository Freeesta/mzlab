//! The conditions of a high-resolution series in the table of the products: `gruppi/1` built from the table of the features and the facts the caller knows of
//! the best hypothesis of each row, and the treated files of each condition for the network. Nothing here is read with fewer than two conditions: one
//! condition (or none) leaves `tptab/1` and the network exactly as they are.

use mzlab_core::campioni::Campione;
use mzlab_core::error::Result;
use mzlab_core::json::Value;

use super::*;
use crate::gruppi::{self, CondFiles, GroupOpts, GroupTable, RowMeta};

/// What the caller knows of a row from its best hypothesis: the table itself carries no formula.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RowInfo {
    /// Index of the row in the table.
    pub row: usize,
    pub neutral_formula: Option<String>,
    /// Formula of the ion, e.g. of `[M+H]+`.
    pub ion_formula: Option<String>,
    /// Error of the ion in ppm.
    pub ppm: Option<f64>,
}

/// The conditions when there are two or more, else none.
pub fn conditions(cs: &[Campione]) -> Vec<CondFiles> {
    let c = gruppi::condition_files(cs);
    if c.len() < 2 {
        Vec::new()
    } else {
        c
    }
}

/// The treated files (time above 0) of each condition, in the order of the conditions; empty with fewer than two conditions.
pub fn treated_by_condition(cs: &[Campione]) -> Vec<Vec<usize>> {
    conditions(cs)
        .into_iter()
        .map(|c| {
            c.files
                .into_iter()
                .filter(|&f| cs[f].time_min.is_some_and(|t| t > 0.0))
                .collect()
        })
        .collect()
}

/// Reads `[{"row":n,"neutral_formula":..,"ion_formula":..,"ppm":..}, ..]` (each key but `row` optional).
pub fn infos_from_json(text: &str) -> Result<Vec<RowInfo>> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let v = mzlab_core::json::parse(text.as_bytes())?;
    Ok(v.arr()
        .iter()
        .filter_map(|o| {
            Some(RowInfo {
                row: o.get("row").and_then(Value::f64)? as usize,
                neutral_formula: o
                    .get("neutral_formula")
                    .and_then(Value::str)
                    .map(String::from),
                ion_formula: o.get("ion_formula").and_then(Value::str).map(String::from),
                ppm: o.get("ppm").and_then(Value::f64),
            })
        })
        .collect())
}

/// The `GroupTable` of the rows `rows` of the table.
fn group_table(t: &Aligned, rows: &[usize], infos: &[RowInfo]) -> GroupTable {
    let mut g = GroupTable {
        n_rows: rows.len(),
        n_files: t.n_files,
        ..GroupTable::default()
    };
    for &r in rows {
        let a = r * t.n_files..(r + 1) * t.n_files;
        g.area.extend_from_slice(&t.area[a.clone()]);
        g.height
            .extend(t.height[a.clone()].iter().map(|&h| f64::from(h)));
        g.state.extend_from_slice(&t.state[a.clone()]);
        g.apex_rt.extend_from_slice(&t.apex_rt[a]);
        let info = infos.iter().find(|i| i.row == r);
        let is_parent =
            !t.tp.is_empty() && t.tp.parent_row as usize == r && t.tp.parent_row != NO_PARENT;
        g.rows.push(RowMeta {
            id: t.id(r),
            tp_name: if is_parent {
                "Parent".into()
            } else {
                format!("TP {}", t.id(r))
            },
            mz: t.mz[r],
            polarity: "+".into(),
            rt: t.rt[r],
            neutral_formula: info.and_then(|i| i.neutral_formula.clone()),
            ion_formula: info.and_then(|i| i.ion_formula.clone()),
            ppm: info.and_then(|i| i.ppm),
            is_parent,
            ..RowMeta::default()
        });
    }
    g
}

/// `gruppi/1` of the rows `rows`, or `None` with fewer than two conditions.
pub fn groups_json(
    t: &Aligned,
    cs: &[Campione],
    rows: &[usize],
    infos: &[RowInfo],
) -> Result<Option<String>> {
    if conditions(cs).is_empty() {
        return Ok(None);
    }
    let g = group_table(t, rows, infos);
    gruppi::gruppi_json(&g, cs, &GroupOpts::default()).map(Some)
}
