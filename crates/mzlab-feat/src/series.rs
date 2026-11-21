//! A series of files from the indexes to the aligned table: the analysis of every file (`analyse_file`), the alignment in the mode of the recipe
//! and the gap filling (`align_and_fill`). One index is open at a time; the tables of the files are the only thing kept between the steps.

use crate::axis::half;
use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::detect_route::detect;
use crate::famalign::{align_families, align_ions};
use crate::family::analyse_families;
use crate::gapfill::{fill_apply, fill_find, FillParams};
use crate::lrscreen::centroids_of;
use crate::pipeline::analyse_with;
use crate::prog::Prog;
use crate::quality::assess;
use crate::recipe::{Mode, Recipe, WEAK_RATIO};
use crate::rtmap::{build, RtMap};
use crate::spill::{Spill, Weak};
use crate::table::{Aligned, FileFeatures, FLAG_GOOD, FLAG_ISOTOPE};
use crate::trace::Params;
use crate::weakfile;

/// Detection, quality, isotopes, MS2 and families of one file.
pub fn analyse_file<R: ReadAt>(index: &mut Index<R>, r: &Recipe) -> Result<FileFeatures> {
    analyse_file_with(index, r, &mut |_, _| {})
}

/// `analyse_file` telling `progress(done, total)` how far the detection is; the features are the same with any `progress`.
pub fn analyse_file_with<R: ReadAt>(
    index: &mut Index<R>,
    r: &Recipe,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<FileFeatures> {
    if let Some(mut c) = centroids_of(index, r)? {
        return analyse_plain(&mut c.index, r, progress);
    }
    analyse_plain(index, r, progress)
}

fn analyse_plain<R: ReadAt>(
    index: &mut Index<R>,
    r: &Recipe,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<FileFeatures> {
    let (mut f, links) = analyse_with(index, &r.detect, &r.steps, progress)?;
    crate::prof::time("detect: families", || {
        analyse_families(index, &mut f, &links, &r.detect, &r.fam)
    })?;
    Ok(f)
}

/// The cells file `k` fills, found on its centroids when the recipe centroids it (the same view as the detection).
fn find_screened<R: ReadAt>(
    index: &mut Index<R>,
    al: &Aligned,
    k: usize,
    map: Option<&RtMap>,
    r: &Recipe,
) -> Result<Vec<(usize, f64)>> {
    // the table holds corrected m/z; the raw scans are read at m/z + the file's offset
    let fill = FillParams {
        mz_shift: r.screen.offsets.get(k),
        ..r.fill
    };
    if let Some(mut c) = centroids_of(index, r)? {
        return fill_find(&mut c.index, al, k, map, &r.detect, &fill);
    }
    fill_find(index, al, k, map, &r.detect, &fill)
}

/// The SHA-256 of the file an index was built from (empty when the index does not carry it).
pub fn source_sha256<R: ReadAt>(index: &Index<R>) -> String {
    index
        .source
        .get("sha256")
        .and_then(|v| v.str())
        .unwrap_or("")
        .to_string()
}

/// The weak candidates of a file: the traces between `WEAK_RATIO` of the minimum height and the minimum height, good by the quality limits, that are
/// not an isotopologue. What a file contributes to the weak tier before the strong peaks of its own table take their share (`unheld`); the step that
/// detects the file stores it (`weakfile`) so that the alignment does not detect the file again.
pub fn weak_candidates<R: ReadAt>(index: &mut Index<R>, r: &Recipe) -> Result<FileFeatures> {
    if let Some(mut c) = centroids_of(index, r)? {
        return weak_plain(&mut c.index, r);
    }
    weak_plain(index, r)
}

fn weak_plain<R: ReadAt>(index: &mut Index<R>, r: &Recipe) -> Result<FileFeatures> {
    let p = Params {
        min_height: r.detect.min_height * WEAK_RATIO,
        ..r.detect.clone()
    };
    let mut w = crate::prof::time("weak: traces", || detect(index, &p))?;
    crate::prof::time("weak: quality", || {
        assess(index, &mut w, &p, &r.steps.quality)
    })?;
    crate::prof::time("weak: isotopes", || {
        crate::pipeline::isotopes(index, &mut w, &p, &r.steps)
    })?;
    let mut out = FileFeatures {
        dt: w.dt,
        ..FileFeatures::default()
    };
    for i in (0..w.len()).filter(|&i| {
        f64::from(w.height[i]) < r.detect.min_height
            && w.flags[i] & FLAG_GOOD != 0
            && w.flags[i] & FLAG_ISOTOPE == 0
    }) {
        out.push_lean(&w, i);
    }
    Ok(out)
}

/// The candidates that `strong` (the table of the file) does not already have: same m/z within the ppm of the alignment and RT within its RT gap.
fn unheld(c: &FileFeatures, strong: &FileFeatures, r: &Recipe) -> FileFeatures {
    let (ppm, gap) = (r.align.ion.ppm, r.align.ion.rt_gap);
    let mut out = FileFeatures {
        dt: c.dt,
        ..FileFeatures::default()
    };
    for i in 0..c.len() {
        let tol = half(c.mz[i], ppm);
        let from = strong.mz.partition_point(|&m| m < c.mz[i] - tol);
        let held = (from..strong.len())
            .take_while(|&j| strong.mz[j] <= c.mz[i] + tol)
            .any(|j| f64::from(strong.rt[j] - c.rt[i]).abs() <= gap);
        if !held {
            out.push_lean(c, i);
        }
    }
    out
}

/// The table of a file as the weak tier needs it: its m/z and RT, and the stored candidates when the source of the series has them.
pub(crate) struct StrongFile {
    pub(crate) table: FileFeatures,
    pub(crate) weak: Option<Vec<u8>>,
}

/// The opener of the gap filling (`finish`): shared between threads where there are threads; in wasm there are none, so it needs no `Sync` and the
/// page opens each index without its scan table (`Index::open_lean`).
#[cfg(not(target_family = "wasm"))]
pub type FillOpen<'a, R> = dyn Fn(usize) -> Result<Index<R>> + Sync + 'a;
#[cfg(target_family = "wasm")]
pub type FillOpen<'a, R> = dyn Fn(usize) -> Result<Index<R>> + 'a;

/// Aligns the tables in the mode of the recipe and fills the cells without a peak, file by file: `open(k)` gives the index of file `k`.
/// Returns the table and the number of cells filled.
pub fn align_and_fill<R: ReadAt>(
    files: &[FileFeatures],
    r: &Recipe,
    mut open: impl FnMut(usize) -> Result<Index<R>>,
) -> Result<(Aligned, usize)> {
    let offsets = crate::lrscreen_offset::resolve(files.len(), r, &mut open)?;
    let moved;
    let files = if offsets.is_empty() {
        files
    } else {
        moved = files
            .iter()
            .zip(&offsets)
            .map(|(f, d)| {
                let mut f = f.clone();
                crate::lrscreen_offset::correct(&mut f.mz, *d);
                f
            })
            .collect::<Vec<_>>();
        &moved[..]
    };
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let table = align_rows(&refs, r)?;
    let pos = |k: usize| -> Result<FileFeatures> {
        Ok(FileFeatures {
            mz: files[k].mz.clone(),
            rt: files[k].rt.clone(),
            ..FileFeatures::default()
        })
    };
    let strong = |k: usize| {
        Ok(StrongFile {
            table: pos(k)?,
            weak: None,
        })
    };
    let (mut table, filled) = finish(
        table,
        &mut { strong },
        files.len(),
        r,
        None,
        None,
        open,
        None,
        &mut Prog::off(),
    )?;
    crate::rowmeta::compute_in(&mut table, r.row_window(), &mut |k, (lo, hi)| {
        let mut part = FileFeatures {
            dt: files[k].dt,
            ..FileFeatures::default()
        };
        let a = files[k].mz.partition_point(|&m| m < lo);
        let b = files[k].mz.partition_point(|&m| m <= hi);
        for i in a..b {
            part.push_row(&files[k], i);
            part.family_id.push(files[k].family_id[i]);
            part.role.push(files[k].role[i]);
            part.alt.push(files[k].alt[i]);
        }
        Ok(part)
    })?;
    Ok((table, filled))
}

/// The rows of the aligned table of `files`: the join of mzmine when the recipe chooses it (one row per ion whatever the mode), else the groups of the mode.
pub(crate) fn align_rows(files: &[&FileFeatures], r: &Recipe) -> Result<Aligned> {
    let shifted = crate::lrscreen_recenter::recentred(files, r)?;
    let files: Vec<&FileFeatures> = match &shifted {
        Some(v) => v.iter().collect(),
        None => files.to_vec(),
    };
    let files = files.as_slice();
    if let Some(j) = &r.join {
        return join_table(files, r, j);
    }
    match r.mode {
        Mode::Ion => align_ions(files, &r.align, &r.rt),
        Mode::Family => align_families(files, &r.align),
    }
}

#[cfg(feature = "untargeted")]
fn join_table(
    files: &[&FileFeatures],
    r: &Recipe,
    j: &crate::align_mzmine::JoinAlign,
) -> Result<Aligned> {
    crate::untargeted::flow_align::join_table(files, r, j)
}

#[cfg(not(feature = "untargeted"))]
fn join_table(
    _: &[&FileFeatures],
    _: &Recipe,
    _: &crate::align_mzmine::JoinAlign,
) -> Result<Aligned> {
    Err(mzlab_core::error::Error::new("error.badRecipe"))
}

/// The gap filling of the peak finder of mzmine, file after file (`series::finish` with `Recipe::peak_finder`).
#[cfg(feature = "untargeted")]
fn peak_finder<R: ReadAt>(
    table: &mut Aligned,
    n: usize,
    r: &Recipe,
    f: &crate::align_mzmine::PeakFinder,
    open: &mut dyn FnMut(usize) -> Result<Index<R>>,
    prog: &mut Prog,
) -> Result<usize> {
    crate::untargeted::flow_align::peak_finder(table, n, r, f, open, prog)
}

#[cfg(not(feature = "untargeted"))]
fn peak_finder<R: ReadAt>(
    _: &mut Aligned,
    _: usize,
    _: &Recipe,
    _: &crate::align_mzmine::PeakFinder,
    _: &mut dyn FnMut(usize) -> Result<Index<R>>,
    _: &mut Prog,
) -> Result<usize> {
    Err(mzlab_core::error::Error::new("error.badRecipe"))
}

/// The cells a file fills: (row, area).
#[cfg(not(target_family = "wasm"))]
type Cells = Vec<(usize, f64)>;

/// Waits for the `n` files counted in `finished` and tells `prog` how many are done, as they finish; gives up when `stalled` says the workers are gone.
#[cfg(not(target_family = "wasm"))]
fn report_finished(
    finished: &(std::sync::Mutex<usize>, std::sync::Condvar),
    n: usize,
    prog: &mut Prog,
    stalled: impl Fn() -> bool,
) {
    let mut seen = 0;
    while seen < n {
        let mut g = finished.0.lock().unwrap_or_else(|e| e.into_inner());
        while *g == seen && !stalled() {
            g = finished
                .1
                .wait_timeout(g, std::time::Duration::from_millis(50))
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        let now = *g;
        drop(g);
        if now == seen {
            return;
        }
        prog.tick(now - seen);
        seen = now;
    }
}

/// The gap filling of every file, a few threads at a time (`open` can be called from any thread): the cells of a file are found without changing the table and written in file order,
/// the cells `fill_file` fills one file after the other.
#[cfg(not(target_family = "wasm"))]
fn fill_threads<R: ReadAt>(
    table: &mut Aligned,
    n: usize,
    map: Option<&RtMap>,
    r: &Recipe,
    open: &(dyn Fn(usize) -> Result<Index<R>> + Sync),
    prog: &mut Prog,
) -> Result<usize> {
    let workers = std::thread::available_parallelism().map_or(1, |w| w.get());
    let mut found: Vec<Option<Result<Cells>>> = (0..n).map(|_| None).collect();
    {
        let al: &Aligned = table;
        let find = |k: usize| open(k).and_then(|mut ix| find_screened(&mut ix, al, k, map, r));
        if workers > 1 && n > 1 {
            let next = std::sync::atomic::AtomicUsize::new(0);
            let slots = std::sync::Mutex::new(&mut found);
            // files finished so far: the calling thread reports them while the workers run
            let finished = (std::sync::Mutex::new(0usize), std::sync::Condvar::new());
            std::thread::scope(|s| {
                let handles: Vec<_> = (0..workers.min(n))
                    .map(|_| {
                        s.spawn(|| loop {
                            let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if k >= n {
                                break;
                            }
                            let f = find(k);
                            slots.lock().unwrap_or_else(|e| e.into_inner())[k] = Some(f);
                            *finished.0.lock().unwrap_or_else(|e| e.into_inner()) += 1;
                            finished.1.notify_all();
                        })
                    })
                    .collect();
                report_finished(&finished, n, prog, || {
                    handles.iter().all(|h| h.is_finished())
                });
            });
        }
        for (k, slot) in found.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(find(k));
                prog.tick(1);
            }
        }
    }
    let mut filled = 0;
    for (k, f) in found.into_iter().enumerate() {
        let f = f.unwrap_or_else(|| Ok(Vec::new()))?;
        filled += fill_apply(table, k, &f);
    }
    Ok(filled)
}

/// Weak tier, map of RT and gap filling on the aligned table of the strong peaks. `strong(k)` gives the table of file `k` (only its m/z and RT are read).
#[allow(clippy::too_many_arguments)]
pub(crate) fn finish<R: ReadAt>(
    mut table: Aligned,
    strong: &mut dyn FnMut(usize) -> Result<StrongFile>,
    n: usize,
    r: &Recipe,
    budget: Option<&MemBudget>,
    spill: Option<&mut dyn Spill>,
    mut open: impl FnMut(usize) -> Result<Index<R>>,
    par_open: Option<&FillOpen<R>>,
    prog: &mut Prog,
) -> Result<(Aligned, usize)> {
    if r.weak_files > 0 {
        let mut weak = Weak::new(spill);
        let key = r.weak_hash();
        for k in 0..n {
            let s = strong(k)?;
            let stored = match &s.weak {
                Some(b) => weakfile::from_bytes(b, &key)?,
                None => None,
            };
            let mut candidates = match stored {
                Some(c) => c,
                None => weak_candidates(&mut open(k)?, r)?,
            };
            // raw m/z against the corrected m/z of the strong table
            crate::lrscreen_offset::correct(&mut candidates.mz, r.screen.offsets.get(k));
            weak.add(unheld(&candidates, &s.table, r), budget)?;
            prog.tick(1);
        }
        let ions = weak.recurring(r, budget)?;
        table.append_unheld(&ions, r.align.ion.ppm, r.fill.rt_tol);
    }
    let map: RtMap = build(&table, &r.rt);
    let map = map.ok().then_some(&map);
    if let Some((lo, hi)) = r.rt_range {
        table.retain_rt(lo, hi);
    }
    let mut filled = 0;
    if let (true, Some(f)) = (r.fill_gaps, &r.peak_finder) {
        filled = peak_finder(&mut table, n, r, f, &mut open, prog)?;
    } else if r.fill_gaps {
        #[cfg(not(target_family = "wasm"))]
        if let Some(po) = par_open {
            let filled = fill_threads(&mut table, n, map, r, po, prog)?;
            crate::lrscreen_dedup::merge(&mut table, r);
            table.sort_rows();
            return Ok((table, filled));
        }
        for k in 0..n {
            let mut index = match par_open {
                Some(po) => po(k)?,
                None => open(k)?,
            };
            let found = find_screened(&mut index, &table, k, map, r)?;
            filled += fill_apply(&mut table, k, &found);
            prog.tick(1);
        }
    }
    crate::lrscreen_dedup::merge(&mut table, r);
    table.sort_rows();
    Ok((table, filled))
}
