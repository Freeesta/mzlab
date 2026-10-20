//! The alignment of a series from its stored `.mzfeatf` under a memory budget (`mzlab_core::budget`). Three ways, one result: the bytes of the table do
//! not depend on the way.
//! - In memory: the lean tables of all the files (39 bytes a row, the columns `align` reads) when the estimate fits in half of what is free.
//! - In slabs ("per ion" mode): the files are read by ranges of m/z cut where no two features of the series are closer than the ppm of the alignment, so no
//!   group crosses a cut; each slab is aligned alone and appended in m/z order, which is the order of the whole alignment.
//! - Refusal `error.memoryBudget` (phase `align`, estimate, budget) when not even one slab fits, or in "per family" mode when the series does not fit in
//!   memory (the families link ions of any m/z, so the series cannot be cut).

use crate::axis::half;
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
        if since >= slab_rows && all[i] - all[i - 1] > half(all[i], ppm) {
            cuts.push(all[i]);
            since = 0;
        }
    }
    cuts.push(f64::INFINITY);
    cuts
}

/// The way a series is aligned (the three ways of the module).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    Memory,
    Slabs,
    Refused,
}

impl Via {
    /// The word `prov/1` and `corsa/1` report: `memory`, `slabs` or `refused`.
    pub fn name(self) -> &'static str {
        match self {
            Via::Memory => "memory",
            Via::Slabs => "slabs",
            Via::Refused => "refused",
        }
    }
}

/// What the alignment of a series asks for: the way and the bytes it holds at once (the whole estimate in memory, the survey, the slab and the table in
/// slabs, what it would need when refused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stima {
    pub bytes: usize,
    pub via: Via,
}

/// Features in each file of a stored series, from their metadata only.
pub fn rows_of(src: &mut dyn Stored) -> Result<Vec<usize>> {
    (0..src.count())
        .map(|k| Ok(read_meta_mz(&src.bytes(k)?)?.1.len()))
        .collect()
}

/// The way for a series of `rows` features per file under a free budget of `left` bytes (after the survey of m/z is reserved); `sorted` is false when a
/// file does not list its features by m/z (the slabs need that order). The one rule `align_and_fill_stored` follows.
fn route(rows: &[usize], r: &Recipe, left: usize, sorted: bool) -> Stima {
    let (total, widest, n) = (
        rows.iter().sum::<usize>(),
        rows.iter().copied().max().unwrap_or(0),
        rows.len(),
    );
    let table_guess = widest * 3 / 2 * (n * CELL + TABLE_ROW);
    let whole = total * (LEAN_ROW + WORK_ROW) + table_guess;
    if r.mode == Mode::Family || whole <= left / 2 || !sorted {
        let via = if whole > left {
            Via::Refused
        } else {
            Via::Memory
        };
        return Stima { bytes: whole, via };
    }
    let slab = (left / 4).max(16 * (LEAN_ROW + WORK_ROW));
    Stima {
        bytes: total * 8 + slab + table_guess,
        via: Via::Slabs,
    }
}

/// The way and the memory of aligning a series whose files hold `rows` features each, under `budget_bytes` (the features listed by m/z, as the
/// files are written). What `flow` reports for the step and what the page asks before it starts.
pub fn stima(rows: &[usize], r: &Recipe, budget_bytes: usize) -> Stima {
    let survey = rows.iter().sum::<usize>() * 8;
    route(rows, r, budget_bytes.saturating_sub(survey), true)
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
    let _all = budget.reserve("align", s.all.len() * 8)?;
    let way = route(&s.rows, r, budget.left(), s.sorted);
    let table = if way.via != Via::Slabs {
        if way.via == Via::Refused {
            return Err(budget.refuse("align", way.bytes));
        }
        let _g = budget.reserve("align", way.bytes)?;
        in_memory(src, n, r)?
    } else {
        let row = LEAN_ROW + WORK_ROW;
        let slab_rows = (budget.left() / 4 / row).max(16);
        let cuts = cuts(&mut s.all, slab_rows, r.align.ion.ppm);
        slabs(src, n, &cuts, budget, r)?
    };
    drop(s);
    let (mut table, filled) = finish(table, &mut |k| lean(src, k, None), n, r, Some(budget), open)?;
    crate::rowmeta::compute_in(&mut table, r.row_window(), &mut |k, range| {
        let b = src.bytes(k)?;
        let want = Want {
            lean: false,
            mz: Some(range),
        };
        Ok(read_file_from(&mut Slice(&b), b.len() as u64, want)?.0)
    })?;
    Ok((table, filled))
}

fn in_memory(src: &mut dyn Stored, n: usize, r: &Recipe) -> Result<Aligned> {
    let files = (0..n)
        .map(|k| lean(src, k, None))
        .collect::<Result<Vec<_>>>()?;
    let refs: Vec<&FileFeatures> = files.iter().collect();
    Ok(match r.mode {
        Mode::Ion => align(&refs, &r.align.ion)?,
        Mode::Family => align_families(&refs, &r.align)?,
    })
}

fn slabs(
    src: &mut dyn Stored,
    n: usize,
    cuts: &[f64],
    budget: &MemBudget,
    r: &Recipe,
) -> Result<Aligned> {
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
        table.append(&align(&refs, &r.align.ion)?);
    }
    Ok(table)
}
