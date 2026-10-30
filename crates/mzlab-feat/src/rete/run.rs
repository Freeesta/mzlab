//! The network of a `.mzfeat` that carries `tp/1` and `interp/1`: the rows come from the interpretation (formula of the best hypothesis, fragments of the
//! MS2 against the parent's), the time of the strongest area of each row from the roles of the files. Used by the `.wasm` `interp` and the native runner.

use std::collections::HashMap;

use mzlab_core::elements::{fmt, parse_formula};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::{self, Value};

use super::{build, ReteParams, ReteRow};
use crate::hypo::Rules;
use crate::interp::store;
use crate::store_read::read_table_from;
use crate::table::Aligned;
use crate::tp::{measured, TpRoles, NO_PARENT};

/// The time of the strongest area of every row among the treated files (None for a row with no area, and for all rows without roles).
fn strongest_times(t: &Aligned, roles: Option<&TpRoles>) -> Vec<Option<f64>> {
    let Some(ro) = roles else {
        return vec![None; t.len()];
    };
    (0..t.len())
        .map(|r| {
            let mut best: Option<(f64, f64)> = None;
            for &k in &ro.treated {
                let (a, m) = (
                    measured(t, r, k),
                    ro.minutes.get(k).copied().unwrap_or(f64::NAN),
                );
                if a > best.map_or(0.0, |b| b.0) && m.is_finite() {
                    best = Some((a, m));
                }
            }
            best.map(|b| b.1)
        })
        .collect()
}

fn floats(v: Option<&Value>) -> Vec<f64> {
    v.map(|v| v.arr().iter().filter_map(Value::f64).collect())
        .unwrap_or_default()
}

/// The neutral formula of a protonated ion formula (the polarity of the series is positive in HR).
fn neutral_of(ion: &str) -> String {
    let mut f = parse_formula(ion).unwrap_or_default();
    if let Some(h) = f.iter_mut().find(|(e, _)| e == "H") {
        h.1 = h.1.saturating_sub(1);
    }
    f.retain(|(_, n)| *n > 0);
    fmt(&f)
}

fn row_of(table: &Aligned, tmax: &[Option<f64>], row: usize) -> ReteRow {
    ReteRow {
        row,
        id: table.id(row),
        mz: table.mz[row],
        rt: table.rt[row],
        tmax: tmax[row],
        ..ReteRow::default()
    }
}

/// `rete/1` of the products of `table` (a `.mzfeat` with the block `tp`; the areas give the times) interpreted in `tp_file` (a `.mzfeat` with the block
/// `interp`; the same file when one carries both). `campioni` is the `campioni/2` document of the series ("" = no roles, no time course).
pub fn network_text<T: ReadAt, P: ReadAt>(
    table: (&mut T, u64),
    tp_file: (&mut P, u64),
    campioni: &str,
    rules: &[u8],
    params: &ReteParams,
) -> Result<String> {
    let (aligned, table_meta) = read_table_from(table.0, table.1)?;
    if aligned.tp.is_empty() || aligned.tp.parent_row == NO_PARENT {
        return Err(Error::new("error.tpNoBlock"));
    }
    let roles = if campioni.trim().is_empty() {
        None
    } else {
        let names = crate::store::input_names(&table_meta);
        Some(TpRoles::from_json(&mzlab_core::campioni::order_like(
            campioni, &names,
        )?)?)
    };
    let tmax = strongest_times(&aligned, roles.as_ref());
    let (meta, ids) =
        store::read_meta(tp_file.0, tp_file.1)?.ok_or_else(|| Error::new("error.rete.noInterp"))?;
    let rules = Rules::parse(rules)?;
    let by_id: HashMap<u32, usize> = (0..aligned.len()).map(|r| (aligned.id(r), r)).collect();
    let pr = aligned.tp.parent_row as usize;
    let mut parent = row_of(&aligned, &tmax, pr);
    parent.tmax = Some(0.0);
    parent.ion_formula = meta
        .get("parent")
        .and_then(|p| p.get("ion_formula"))
        .and_then(Value::str)
        .unwrap_or_default()
        .to_string();
    parent.neutral_formula = neutral_of(&parent.ion_formula);
    let mut rows = Vec::new();
    for id in ids {
        let (Some(&row), Some(text)) = (by_id.get(&id), store::read_row(tp_file.0, tp_file.1, id)?)
        else {
            continue;
        };
        let v = json::parse(text.as_bytes())?;
        let Some(change) = v.get("change").filter(|c| !c.is_null()) else {
            continue;
        };
        let text_of = |k: &str| {
            change
                .get(k)
                .and_then(Value::str)
                .unwrap_or_default()
                .to_string()
        };
        let vs = v.get("vs_parent").filter(|c| !c.is_null());
        let mut r = row_of(&aligned, &tmax, row);
        r.neutral_formula = text_of("neutral_formula");
        r.ion_formula = text_of("ion_formula");
        r.has_ms2 = vs.is_some();
        r.equal = floats(vs.and_then(|s| s.get("equal")));
        r.shifted = floats(vs.and_then(|s| s.get("shifted")));
        rows.push(r);
    }
    Ok(build(&aligned.tp, &parent, &rows, &rules, params).to_json())
}
