//! `features.union`: the `.mzfeat` tables of several series written as the CSV of `mzlab-cli features` (`id, mz, rt, rt_range:min, rt_range:max`, the
//! same decimals as `export::csv_full`) and joined by `mzlab_core::series_union` into ONE target list for `quant.targets`, with the `union/1` document
//! of every merge, conflict, drop and automatic choice. The CLI executor and the `flowtab` `.wasm` both call `union_text`, so the bytes are the same.

use crate::export::fix;
use crate::store::read_table;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};
use mzlab_core::series_union::{run, Params};

/// The target list (CSV) and the `union/1` document.
#[derive(Debug, Clone)]
pub struct Made {
    pub csv: String,
    pub doc: String,
}

/// The CSV of one series from its `.mzfeat`: the ids are the stable row ids of the table, the window is the median RT range of the row (empty without one).
pub fn series_csv(table: &[u8]) -> Result<String> {
    let (t, _) = read_table(table)?;
    let mut out = String::from("id,mz,rt,rt_range:min,rt_range:max\n");
    for r in 0..t.len() {
        let q = (!t.meta.is_empty()).then(|| t.meta.get(r));
        let edge = |v: Option<f32>| {
            v.filter(|x| x.is_finite())
                .map_or(String::new(), |x| fix(f64::from(x), 4))
        };
        out += &format!(
            "{},{},{},{},{}\n",
            t.id(r),
            fix(t.mz[r], 5),
            fix(t.rt[r], 4),
            edge(q.as_ref().map(|q| q.rt_lo)),
            edge(q.as_ref().map(|q| q.rt_hi)),
        );
    }
    Ok(out)
}

/// `tables`: `(name, .mzfeat bytes)` of each series in the order of the plan (the first fixes the RT frame); `params` the canonical parameters of the step.
pub fn union_text(params: &[(String, Value)], tables: &[(String, Vec<u8>)]) -> Result<Made> {
    let p = Params::from_json(Some(&Value::Obj(params.to_vec())))?;
    let named = tables
        .iter()
        .map(|(n, b)| Ok((n.clone(), series_csv(b)?)))
        .collect::<Result<Vec<_>>>()?;
    let doc = run(&named, &p)?;
    let csv = parse(doc.as_bytes())
        .ok()
        .and_then(|v| v.get("csv").and_then(Value::str).map(String::from))
        .ok_or_else(|| Error::new("err.union.request"))?;
    Ok(Made { csv, doc })
}
