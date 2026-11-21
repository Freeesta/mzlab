//! Compatibility layer of mzFinder: the JSON that the mzFinder API returns, computed in Rust. `classify` and `formula_info` are ports with
//! parity to the former Python engine; `summary` and `detail` translate the series of the pipeline (L1-L9, evid/1, tp/1) into the
//! rows and the detail that the former page read.

pub mod dig;
/// The old path of the MSn tree, in `crate::finder_base` like the rest of what the interpretation reads.
#[cfg(feature = "chem")]
pub use crate::finder_base::msn;
/// The old paths of the part of the engine that the interpretation reads, which lives in `crate::finder_base` so that `interp` does not depend on `legacy`.
pub use crate::finder_base::{edge, identity, ms2, sample, series, similarity};
/// The old path of the fragmentation tree (`crate::finder_base::fragtree`) with `of_dig`, which reads the Dig and so stays here.
#[cfg(feature = "chem")]
pub mod fragtree {
    pub use super::fragtree_dig::of_dig;
    pub use crate::finder_base::fragtree::*;
}
#[cfg(feature = "chem")]
mod fragtree_dig;
/// The old path of the formula of the parent, which lives in `crate::parent_formula` so that `series_parent` does not depend on `legacy`.
pub use crate::parent_formula as formula;
pub mod hidden;
mod hidden_fam;
pub mod kinetics;
pub mod lr;
pub mod lrisf;
pub mod network;
mod network_pred;
pub mod out;
pub mod progress;
pub mod transf;

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;

pub use formula::formula_info;
pub use identity::{identity_graph, polarity_pairs};

/// The old path of `Res`, which lives in `crate::finder_base` with the modules that return it.
pub use crate::finder_base::Res;

/// `api.classify(names)`: one item per name; `open(name)` reads the index of that file (its error text becomes the error item).
pub fn classify<R: ReadAt>(
    names: &[String],
    mut open: impl FnMut(&str) -> Result<Index<R>, String>,
) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, n) in names.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        match open(n) {
            Ok(ix) => sample::item_json(&mut o, n, Ok(&ix)),
            Err(e) => sample::item_json::<R>(&mut o, n, Err(e)),
        }
    }
    o.raw("]");
    o.0
}

/// `api.run` on high-resolution files after the pipeline: `files_json` and `parent_json` as the page passes them, `table` the bytes of the
/// `series_evid.mzfeat` of the LC files (in the order of `series::lc_files`), `registry` the `hypotheses/2` JSON, `open(k)` the index of file `k`.
pub fn dig<R: ReadAt>(
    files_json: &str,
    parent_json: &str,
    table: &[u8],
    registry: &str,
    open: impl FnMut(usize) -> Res<Index<R>>,
) -> Res<dig::Dig> {
    let files = series::lc_files(files_json)?;
    let parent = series::parent(parent_json)?;
    let (t, _) = crate::store_read::read_table(table).map_err(|e| e.to_string())?;
    let evid = crate::store_read::read_evid(table).map_err(|e| e.to_string())?;
    let idents = series::identities(&evid)?;
    let (hyps, _, found) = series::registry(registry)?;
    dig::dig(
        dig::Series {
            files,
            parent,
            table: &t,
            idents,
            hyps,
            found,
        },
        open,
    )
}

/// The peak of the parent in the aligned table of the series (before the proofs): `(m/z, RT)` of its row, None when the table has no row there. The
/// proofs need it to read the kinetics against the parent and to run the gate of the in-source fragments.
pub fn parent_peak(files_json: &str, parent_json: &str, table: &[u8]) -> Res<Option<(f64, f64)>> {
    let files = series::lc_files(files_json)?;
    let parent = series::parent(parent_json)?;
    let (t, _) = crate::store_read::read_table(table).map_err(|e| e.to_string())?;
    if t.n_files != files.len() {
        return Ok(None);
    }
    Ok(dig::parent_row(&t, &parent, &files).map(|r| (parent.mz, t.rt[r])))
}
