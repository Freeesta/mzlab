//! How far the running `feat/align` of mzFinder is, for the route `feat/progress` the page asks while it waits (one alignment at a time per app): the same
//! `(done, total)` the `.wasm` hands the page (`align_series_to`), at most once per 1 % (`Pace`). The bytes of the table do not depend on it.

use std::sync::Mutex;

use mzlab_core::budget::{MemBudget, WriteAt};
use mzlab_core::pace::Pace;
use mzlab_core::Result;
use mzlab_feat::legacy::progress;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::Stored;
use mzlab_feat_wasm::run;

use crate::engine::State;
use crate::rpc::{text, Answer};

fn set(at: &Mutex<(usize, usize)>, d: usize, t: usize) {
    *at.lock().unwrap_or_else(|e| e.into_inner()) = (d, t);
}

impl State {
    /// `run::align_to` over the indexes named `ns`, telling `feat/progress` how far it is (in `feat_at`).
    pub(crate) fn align_told<W: WriteAt>(
        &self,
        src: &mut dyn Stored,
        budget: &MemBudget,
        r: &Recipe,
        ns: &[String],
        sink: &mut W,
    ) -> Result<()> {
        let at = &self.feat_at;
        set(at, 0, 0);
        let mut tell = |d, t| set(at, d, t);
        let mut pace = Pace::new(&mut tell);
        let open = |k: usize| self.open_idx(&ns[k]);
        run::align_to_spilling(src, budget, r, None, open, sink, None, None, &mut |d, t| {
            pace.at(d, t)
        })
    }
}

/// `{}` -> the step of the Dig (`finder_progress` of the `.wasm`: label, `done`, `total`, fraction) of the alignment running, or of the last one.
pub(crate) fn route(at: &Mutex<(usize, usize)>) -> Result<Answer> {
    let (done, total) = *at.lock().unwrap_or_else(|e| e.into_inner());
    text(progress::json("hr", "align", done as u32, total as u32))
}
