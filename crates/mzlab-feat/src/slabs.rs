//! The alignment of a series from its stored `.mzfeatf` under a memory budget (`mzlab_core::budget`). Three ways, one result: the bytes of the table do
//! not depend on the way.
//! - In memory: the lean tables of all the files (39 bytes a row, the columns `align` reads) when the estimate fits in half of what is free.
//! - In slabs ("per ion" mode): the files are read by ranges of m/z cut where no two features of the series are closer than the ppm of the alignment, so no
//!   group crosses a cut; each slab is aligned alone and appended in m/z order, which is the order of the whole alignment.
//! - "Per family" mode in slabs too: the groups of ions are made slab by slab (twice, the second time with the map of RT) and only the group of each row
//!   (4 bytes) stays; the votes and links are read from one file at a time (`famalign::Collector`).
//! - The weak tier goes to a `Spill` when the weak tables of all the files do not fit in half of what is free (`spill`).
//! - Refusal `error.memoryBudget` (phase `align`, estimate, budget) when not even one slab fits.

use crate::axis::half;
use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::align::{align, align_grouped};
use crate::famalign::{align_families, assemble, rt_map, Collector};
use crate::recipe::{Mode, Recipe};
use crate::rtmap::RtMap;
use crate::series::finish;
use crate::sortc;
use crate::spill::Spill;
use crate::store::{read_file_from, read_meta_mz, Slice, Want};
use crate::table::{Aligned, FileFeatures};
use crate::trace::filled;

/// Bytes of a lean row in memory (`store::Want::lean`), and of what an alignment holds beside it (items, sorted copies, logarithms).
pub(crate) const LEAN_ROW: usize = 39;
pub(crate) const WORK_ROW: usize = 40;
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
pub(crate) fn cuts(all: &mut [f64], slab_rows: usize, ppm: f64) -> Vec<f64> {
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

/// Aligns and fills a series stored as `.mzfeatf` (see the module). `open(k)` gives the index of file `k`. Returns the table and the cells filled.
pub fn align_and_fill_stored<R: ReadAt>(
    src: &mut dyn Stored,
    budget: &MemBudget,
    r: &Recipe,
    open: impl FnMut(usize) -> Result<Index<R>>,
) -> Result<(Aligned, usize)> {
    align_and_fill_spilling(src, budget, r, open, None)
}

/// `align_and_fill_stored` with a `Spill` for the weak tables of the files (none: they stay in memory, or the phase is refused past half of the free).
pub fn align_and_fill_spilling<R: ReadAt>(
    src: &mut dyn Stored,
    budget: &MemBudget,
    r: &Recipe,
    open: impl FnMut(usize) -> Result<Index<R>>,
    spill: Option<&mut dyn Spill>,
) -> Result<(Aligned, usize)> {
    let n = src.count();
    let mut s = survey(src, budget)?;
    let total: usize = s.rows.iter().sum();
    let widest = s.rows.iter().copied().max().unwrap_or(0);
    let _all = budget.reserve("align", s.all.len() * 8)?;
    let table_guess = widest * 3 / 2 * (n * CELL + TABLE_ROW);
    let whole = total * (LEAN_ROW + WORK_ROW) + table_guess;
    let table = if budget.in_memory(whole) || !s.sorted {
        if whole > budget.left() {
            return Err(budget.refuse("align", whole));
        }
        let _g = budget.reserve("align", whole)?;
        in_memory(src, n, r)?
    } else if r.mode == Mode::Family {
        family_slabs(src, &mut s, budget, r, widest * 3 / 2)?
    } else {
        let cuts = cuts(&mut s.all, slab_rows(budget.left() / 4), r.align.ion.ppm);
        slabs(src, &cuts, budget, r, None, None, Plan::default())?
    };
    drop(s);
    let (mut table, filled) = finish(
        table,
        &mut |k| lean(src, k, None),
        n,
        r,
        Some(budget),
        spill,
        open,
    )?;
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

/// What "per family" mode plans for its slabs: the bytes a slab is cut to take and the rows of room of the table (none: a quarter of the free, and the
/// table grows as it is filled).
#[derive(Default, Clone, Copy)]
struct Plan {
    slab_bytes: Option<usize>,
    table_rows: Option<usize>,
}

/// Rows of a slab that takes `bytes` of lean tables and work.
fn slab_rows(bytes: usize) -> usize {
    (bytes / (LEAN_ROW + WORK_ROW)).max(16)
}

/// Aligns the series slab by slab (`cuts`), with the RT corrected by a map (`adj`) or not. With `group_of`, records the group of every row
/// (`group_of[file][row]`, the rows of a file counted in the whole file, not in the slab). The `plan` says what a slab is cut to take and how many rows
/// the table is given room for.
fn slabs(
    src: &mut dyn Stored,
    cuts: &[f64],
    budget: &MemBudget,
    r: &Recipe,
    adj: Option<&RtMap>,
    mut group_of: Option<&mut Vec<Vec<u32>>>,
    plan: Plan,
) -> Result<Aligned> {
    let n = src.count();
    let mut table = match plan.table_rows {
        Some(rows) => Aligned::with_capacity(n, rows),
        None => Aligned {
            n_files: n,
            ..Aligned::default()
        },
    };
    // rows of each file before the slab: the files are sorted and the slabs are consecutive ranges, so a slab starts where the last one ended
    let mut before = vec![0usize; n];
    for w in cuts.windows(2) {
        let slab = plan.slab_bytes.unwrap_or(budget.left() / 4).max(1);
        // a table that has room for the groups of the slab is not copied when it grows
        let grows =
            plan.table_rows.is_none() || table.len() + slab_rows(slab) > table.mz.capacity();
        let held = table.heap_bytes() * if grows { 2 } else { 1 };
        if held + slab > budget.left() {
            return Err(budget.refuse("align", held + slab));
        }
        let files = (0..n)
            .map(|k| lean(src, k, Some((w[0], w[1]))))
            .collect::<Result<Vec<_>>>()?;
        let rows: usize = files.iter().map(FileFeatures::len).sum();
        let _g = budget.reserve("align", rows * (LEAN_ROW + WORK_ROW))?;
        let refs: Vec<&FileFeatures> = files.iter().collect();
        let g = align_grouped(&refs, &r.align.ion, adj, group_of.is_some())?;
        if let Some(all) = group_of.as_deref_mut() {
            let base = table.len() as u32;
            for (k, local) in g.group_of.iter().enumerate() {
                for (i, &id) in local.iter().enumerate() {
                    all[k][before[k] + i] = if id == u32::MAX { id } else { base + id };
                }
            }
        }
        for (b, f) in before.iter_mut().zip(&files) {
            *b += f.len();
        }
        table.append(&g.table);
    }
    Ok(table)
}

/// "Per family" mode in slabs: the same steps as `align_families`, with the groups of ions made slab by slab and the votes read a file at a time.
fn family_slabs(
    src: &mut dyn Stored,
    s: &mut Survey,
    budget: &MemBudget,
    r: &Recipe,
    table_rows: usize,
) -> Result<Aligned> {
    let p = &r.align;
    // the group of every row stays for the whole alignment
    let group_bytes = s.rows.iter().sum::<usize>() * 4;
    let _groups = budget.reserve("align", group_bytes)?;
    // the table of the ions is held (and copied when it grows): a slab takes half of what is left beside it
    let cell_row = s.rows.len() * CELL + TABLE_ROW;
    let slab_bytes = budget.left().saturating_sub(table_rows * cell_row * 2) / 2;
    let cuts = cuts(&mut s.all, slab_rows(slab_bytes), r.align.ion.ppm);
    let plan = Plan {
        slab_bytes: Some(slab_bytes),
        table_rows: Some(table_rows),
    };
    let first = slabs(src, &cuts, budget, r, None, None, plan)?;
    let map = rt_map(&first, p);
    drop(first);
    let mut group_of = s
        .rows
        .iter()
        .map(|&m| filled(m, u32::MAX))
        .collect::<Result<Vec<_>>>()?;
    let ions = slabs(
        src,
        &cuts,
        budget,
        r,
        map.ok().then_some(&map),
        Some(&mut group_of),
        plan,
    )?;
    // votes and links of the groups and one lean file in memory
    let widest = s.rows.iter().copied().max().unwrap_or(0);
    let _votes = budget.reserve("align", ions.len() * 24 + widest * LEAN_ROW)?;
    let mut c = Collector::new(ions.len())?;
    for (k, g) in group_of.iter().enumerate() {
        c.add(k, &lean(src, k, None)?, g, p);
    }
    drop(group_of);
    Ok(assemble(ions, c, p))
}
