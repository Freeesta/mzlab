//! The alignment of a native run: the gaps of the files are filled at the same time, and the progress can be followed.

use mzlab_core::budget::{MemBudget, WriteAt};
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::{align_and_fill_progress, Stored};
use mzlab_feat::store::write_table_to;

use super::{facts_of, origin_of};

/// `align_to` filling the gaps of the files at the same time (native runs): `open` can be called from any thread. The same bytes.
pub fn align_to_par<R: ReadAt, W: WriteAt>(
    src: &mut dyn Stored,
    budget: &MemBudget,
    r: &Recipe,
    open: &(dyn Fn(usize) -> Result<Index<R>> + Sync),
    sink: &mut W,
) -> Result<()> {
    align_to_par_progress(src, budget, r, open, sink, &mut |_, _| {})
}

/// `align_to_par` telling `progress(done, total)` how far the alignment is (`slabs::align_and_fill_progress`: slabs, files of the weak tier, files
/// whose gaps are filled); the bytes are the same with any `progress`.
pub fn align_to_par_progress<R: ReadAt, W: WriteAt>(
    src: &mut dyn Stored,
    budget: &MemBudget,
    r: &Recipe,
    open: &(dyn Fn(usize) -> Result<Index<R>> + Sync),
    sink: &mut W,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<()> {
    let mut origin = origin_of(src)?;
    origin.facts = facts_of(src.count(), &mut |k| open(k))?;
    let (table, _) = align_and_fill_progress(src, budget, r, open, None, Some(open), progress)?;
    write_table_to(sink, &table, r, &origin, &mut (&[][..]))
}
