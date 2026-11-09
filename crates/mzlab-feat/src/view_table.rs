//! The table a view stands for: the rows the view keeps, in their order, with the areas the view shows (normalised, blank subtracted). A Flow step that
//! reads a `table` and is not an export runs on this table, so a `table.filter` wired into it gives what the same step gives on a table holding those rows.
//! The rows take the ids 1..n of the new table; the block `tp/1` and the graph of the proofs are left out (both point at rows of the old table).

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::{parse, Value};

use crate::aligned::Aligned;
use crate::samples;
use crate::store::{input_names, origin_from_meta, read_table_from, write_table_like};
use crate::store_read::Slice;
use crate::view::{self, View};

/// Is `bytes` the text of a view (`view_json_origin` of the `table` `.wasm`) rather than a `.mzfeat` (which begins with its magic)?
pub fn is_view(bytes: &[u8]) -> bool {
    bytes.first() == Some(&b'{')
}

fn bad() -> Error {
    Error::new("err.flow.export.view")
}

fn origin(view: &str) -> Result<Value> {
    let v = parse(view.as_bytes()).map_err(|_| bad())?;
    v.get("origine").cloned().ok_or_else(bad)
}

/// The SHA-256 of the table and the rules a view was made with; `err.flow.export.view` for any other text.
pub fn view_origin(view: &str) -> Result<(String, String)> {
    let o = origin(view)?;
    let get = |k: &str| o.get(k).and_then(|x| x.str()).map(str::to_string);
    Ok((
        get("tabella").ok_or_else(bad)?,
        get("spec").ok_or_else(bad)?,
    ))
}

/// The `.mzfeat` the view stands for, made from the table it names (`table`, SHA-256 checked) with the samples and rules it carries: the rows kept
/// with the areas of the view, the recipe and the inputs of the table. `err.flow.export.view` for another table or a view without its samples.
pub fn table_of_view(table: &[u8], view: &str) -> Result<Vec<u8>> {
    table_of_view_from(&mut Slice(table), table.len() as u64, view)
}

/// `table_of_view` of the table that `r` reads (`size` bytes): the page reads it through the same reader as its other table functions.
pub fn table_of_view_from<R: ReadAt>(r: &mut R, size: u64, view: &str) -> Result<Vec<u8>> {
    let o = origin(view)?;
    let (sha, spec) = view_origin(view)?;
    let samples_text = o.get("campioni").and_then(|x| x.str()).ok_or_else(bad)?;
    let mut bytes = vec![0u8; usize::try_from(size).map_err(|_| bad())?];
    r.read_at(0, &mut bytes)?;
    if mzlab_core::budget::sha256_hex(&bytes) != sha {
        return Err(bad());
    }
    drop(bytes);
    let (t, meta) = read_table_from(r, size)?;
    let text = mzlab_core::samples::order_like(samples_text, &input_names(&meta))?;
    let s = samples::from_json(text.as_bytes())?;
    if s.len() != t.n_files {
        return Err(Error::new("error.badView"));
    }
    let v = view::from_spec(&t, &s, spec.as_bytes())?;
    let recipe = meta.get("recipe").and_then(|x| x.str()).unwrap_or("");
    Ok(write_table_like(
        &table_of(&t, &v),
        recipe,
        &origin_from_meta(&meta),
    ))
}

/// The table of `v` over `t`: cells, members, summaries, identity and isotopes follow their row; a cell with a peak gets the area of the view.
pub fn table_of(t: &Aligned, v: &View) -> Aligned {
    let nf = t.n_files;
    let mut out = Aligned::with_capacity(nf, v.len());
    let members = !t.member_start.is_empty();
    if members {
        out.member_start.push(0);
    }
    for &r in &v.rows {
        let r = r as usize;
        out.mz.push(t.mz[r]);
        out.rt.push(t.rt[r]);
        for k in 0..nf {
            let c = t.cell(r, k);
            out.area.push(v.area_at(t, r, k).unwrap_or(t.area[c]));
            out.height.push(t.height[c]);
            out.apex_rt.push(t.apex_rt[c]);
            out.width.push(t.width[c]);
            out.state.push(t.state[c]);
        }
        if !t.q3.is_empty() {
            out.q3.push(t.q3[r]);
        }
        if !t.meta.is_empty() {
            out.meta.push(&t.meta.get(r));
        }
        if !t.ident.is_empty() {
            out.ident.neutral.push(t.ident.neutral[r]);
            out.ident.ion.push(t.ident.ion[r]);
            out.ident.src.push(t.ident.src[r]);
        }
        if !t.iso.is_empty() {
            out.iso.group.push(t.iso.group[r]);
            out.iso.charge.push(t.iso.charge[r]);
            out.iso.family.push(t.iso.family[r]);
        }
        for m in t.members(r) {
            out.member_mz.push(t.member_mz[m]);
            out.member_rt.push(t.member_rt[m]);
            out.member_role.push(t.member_role[m]);
        }
        if members {
            out.member_start.push(out.member_mz.len() as u32);
        }
    }
    out
}
