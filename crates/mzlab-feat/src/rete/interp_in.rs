//! The rows of the network from the interpretation of the rows (`interp/1`) and the table: the formula of the best hypothesis of each row and the
//! fragments of its MS2 that equal or are shifted from the parent's.

use super::ReteRow;
use crate::interp::RowInterp;
use crate::legacy::series::Parent;
use crate::table::Aligned;
use crate::tp::TpBlock;
use mzlab_core::elements::fmt;

/// `(parent, rows)`: the parent as node 0 and every interpreted row that has a modification. `tmax[row]` is the time of the strongest area of the
/// row (None without a series).
pub fn rows_from_interp(
    table: &Aligned,
    tp: &TpBlock,
    parent: &Parent,
    interp: &[RowInterp],
    tmax: &[Option<f64>],
) -> (ReteRow, Vec<ReteRow>) {
    let at = |row: usize| {
        (
            table.mz[row],
            table.rt[row],
            tmax.get(row).copied().flatten(),
        )
    };
    let p = tp.parent_row as usize;
    let (mz, rt, t) = at(p);
    let parent_row = ReteRow {
        row: p,
        id: p as u32 + 1,
        mz,
        rt,
        tmax: t.or(Some(0.0)),
        neutral_formula: fmt(&parent.neutral),
        ion_formula: fmt(&parent.ion),
        ..ReteRow::default()
    };
    let rows = interp
        .iter()
        .filter_map(|r| {
            let c = r.change.first()?;
            let (mz, rt, tmax) = at(r.row);
            let v = r.vs_parent.as_ref();
            Some(ReteRow {
                row: r.row,
                id: r.id,
                mz,
                rt,
                tmax,
                neutral_formula: c.neutral_formula.clone(),
                ion_formula: c.ion_formula.clone(),
                equal: v.map(|v| v.equal.clone()).unwrap_or_default(),
                shifted: v.map(|v| v.shifted.clone()).unwrap_or_default(),
                has_ms2: v.is_some(),
            })
        })
        .collect();
    (parent_row, rows)
}
