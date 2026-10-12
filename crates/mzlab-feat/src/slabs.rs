//! The alignment of a series from its stored `.mzfeatf` under a memory budget (`mzlab_core::budget`). Three ways, one result: the bytes of the table do
//! not depend on the way.
//! - In memory: the lean tables of all the files (39 bytes a row, the columns `align` reads) when the estimate fits in half of what is free.
//! - In slabs ("per ion" mode): the files are read by ranges of m/z cut where no two features of the series are closer than the ppm of the alignment, so no
//!   group crosses a cut; each slab is aligned alone and appended in m/z order, which is the order of the whole alignment.
//! - Refusal `error.memoryBudget` (phase `align`, estimate, budget) when not even one slab fits, or in "per family" mode when the series does not fit in
//!   memory (the families link ions of any m/z, so the series cannot be cut).

use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::align::align;
use crate::famalign::align_families;
use crate::recipe::{Mode, Recipe};
use crate::series::finish;
use crate::sortc;
use crate::store::{read_file_from, read_meta_mz, Slice, Want};
use crate::table::{Aligned, FileFeatures};

/// Bytes of a lean row in memory (`store::Want::lean`), and of what an alignment holds beside it (items, sorted copies, logarithms).
const LEAN_ROW: usize = 39;
const WORK_ROW: usize = 40;
/// Bytes of a cell of the aligned table (area, height, apex, width, state) and of a row (m/z, RT, member start).
const CELL: usize = 21;
const TABLE_ROW: usize = 28;

/// The stored `.mzfeatf` of a series, a file at a time: the page keeps them (in JavaScript or in OPFS) and gives a copy when asked.
pub trait Stored {
    fn count(&self) -> usize;
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>>;
}

impl Stored for Vec<Vec<u8>> {
    fn count(&self) -> usize {
        self.len()
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        Ok(self[k].clone())
    }
}

/// The lean table of file `k`, only the rows with m/z in `range`.
fn lean(src: &mut dyn Stored, k: usize, range: Option<(f64, f64)>) -> Result<FileFeatures> {
    let b = src.bytes(k)?;
    let want = Want {
        lean: true,
        mz: range,
    };
    Ok(read_file_from(&mut Slice(&b), b.len() as u64, want)?.0)
}

/// What the series is: rows of each file and all the m/z, and whether every file is sorted by m/z (the slabs need it).
struct Survey {
    rows: Vec<usize>,
    all: Vec<f64>,
    sorted: bool,
}

fn survey(src: &mut dyn Stored, budget: &MemBudget) -> Result<Survey> {
    let mut s = Survey {
        rows: Vec::new(),
        all: Vec::new(),
        sorted: true,
    };
    for k in 0..src.count() {
        let (_, mz) = read_meta_mz(&src.bytes(k)?)?;
        s.sorted &= mz.windows(2).all(|w| w[0] <= w[1]);
        s.all
            .try_reserve(mz.len())
            .map_err(|_| budget.refuse("align", (s.all.len() + mz.len()) * 8))?;
        s.rows.push(mz.len());
        s.all.extend_from_slice(&mz);
    }
    Ok(s)
}

/// Cuts of m/z, from `-inf` to `inf`: a slab holds about `slab_rows` features and ends in a gap wider than the ppm of the alignment.
fn cuts(all: &mut [f64], slab_rows: usize, ppm: f64) -> Vec<f64> {
    sortc::by(all, |a, b| a.total_cmp(b));
    let (mut cuts, mut since) = (vec![f64::NEG_INFINITY], 0usize);
    for i in 1..all.len() {
        since += 1;
        if since >= slab_rows && all[i] - all[i - 1] > all[i] * ppm * 1e-6 {
            cuts.push(all[i]);
            since = 0;
        }
    }
    cuts.push(f64::INFINITY);
    cuts
}

/// Aligns and fills a series stored as `.mzfeatf` (see the module). `open(k)` gives the index of file `k`. Returns the table and the cells filled.
pub fn align_and_fill_stored<R: ReadAt>(
    src: &mut dyn Stored,
    budget: &MemBudget,
    r: &Recipe,
    open: impl FnMut(usize) -> Result<Index<R>>,
) -> Result<(Aligned, usize)> {
    let n = src.count();
    let mut s = survey(src, budget)?;
    let total: usize = s.rows.iter().sum();
    let widest = s.rows.iter().copied().max().unwrap_or(0);
    let _all = budget.reserve("align", s.all.len() * 8)?;
    let table_guess = widest * 3 / 2 * (n * CELL + TABLE_ROW);
    let whole = total * (LEAN_ROW + WORK_ROW) + table_guess;
    let table = if r.mode == Mode::Family || budget.in_memory(whole) || !s.sorted {
        if whole > budget.left() {
            return Err(budget.refuse("align", whole));
        }
        let _g = budget.reserve("align", whole)?;
        in_memory(src, n, r)?
    } else {
        let row = LEAN_ROW + WORK_ROW;
        let slab_rows = (budget.left() / 4 / row).max(16);
        let cuts = cuts(&mut s.all, slab_rows, r.align.ion.ppm);
        slabs(src, n, &cuts, budget, r)?
    };
    drop(s);
    finish(table, &mut |k| lean(src, k, None), n, r, open)
}

fn in_memory(src: &mut dyn Stored, n: usize, r: &Recipe) -> Result<Aligned> {
    let files = (0..n)
        .map(|k| lean(src, k, None))
        .collect::<Result<Vec<_>>>()?;
    let refs: Vec<&FileFeatures> = files.iter().collect();
    Ok(match r.mode {
        Mode::Ion => align(&refs, &r.align.ion),
        Mode::Family => align_families(&refs, &r.align),
    })
}

fn slabs(src: &mut dyn Stored, n: usize, cuts: &[f64], budget: &MemBudget, r: &Recipe) -> Result<Aligned> {
    let mut table = Aligned {
        n_files: n,
        ..Aligned::default()
    };
    for w in cuts.windows(2) {
        let held = table.heap_bytes() * 2;
        let slab = (budget.left() / 4).max(1);
        if held + slab > budget.left() {
            return Err(budget.refuse("align", held + slab));
        }
        let files = (0..n)
            .map(|k| lean(src, k, Some((w[0], w[1]))))
            .collect::<Result<Vec<_>>>()?;
        let rows: usize = files.iter().map(FileFeatures::len).sum();
        let _g = budget.reserve("align", rows * (LEAN_ROW + WORK_ROW))?;
        let refs: Vec<&FileFeatures> = files.iter().collect();
        table.append(&align(&refs, &r.align.ion));
    }
    Ok(table)
}
