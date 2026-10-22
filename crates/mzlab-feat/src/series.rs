//! A series of files from the indexes to the aligned table: the analysis of every file (`analyse_file`), the alignment in the mode of the recipe
//! and the gap filling (`align_and_fill`). One index is open at a time; the tables of the files are the only thing kept between the steps.

use crate::axis::half;
use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::famalign::{align_families, align_ions};
use crate::family::analyse_families;
use crate::gapfill::fill_file;
use crate::isotopes::group;
use crate::pipeline::analyse;
use crate::quality::assess;
use crate::recipe::{Mode, Recipe, WEAK_RATIO};
use crate::rtmap::{build, RtMap};
use crate::spill::{Spill, Weak};
use crate::table::{Aligned, FileFeatures, FLAG_GOOD, FLAG_ISOTOPE};
use crate::trace::{detect, Params};
use crate::weakfile;

/// Detection, quality, isotopes, MS2 and families of one file.
pub fn analyse_file<R: ReadAt>(index: &mut Index<R>, r: &Recipe) -> Result<FileFeatures> {
    let (mut f, links) = analyse(index, &r.detect, &r.steps)?;
    analyse_families(index, &mut f, &links, &r.detect, &r.fam)?;
    Ok(f)
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
    let p = Params {
        min_height: r.detect.min_height * WEAK_RATIO,
        ..r.detect.clone()
    };
    let mut w = detect(index, &p)?;
    assess(index, &mut w, &p, &r.steps.quality)?;
    group(&mut w, &r.steps.iso)?;
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

/// Aligns the tables in the mode of the recipe and fills the cells without a peak, file by file: `open(k)` gives the index of file `k`.
/// Returns the table and the number of cells filled.
pub fn align_and_fill<R: ReadAt>(
    files: &[FileFeatures],
    r: &Recipe,
    open: impl FnMut(usize) -> Result<Index<R>>,
) -> Result<(Aligned, usize)> {
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let table = match r.mode {
        Mode::Ion => align_ions(&refs, &r.align, &r.rt)?,
        Mode::Family => align_families(&refs, &r.align)?,
    };
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
    let (mut table, filled) = finish(table, &mut { strong }, files.len(), r, None, None, open)?;
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

/// Weak tier, map of RT and gap filling on the aligned table of the strong peaks. `strong(k)` gives the table of file `k` (only its m/z and RT are read).
pub(crate) fn finish<R: ReadAt>(
    mut table: Aligned,
    strong: &mut dyn FnMut(usize) -> Result<StrongFile>,
    n: usize,
    r: &Recipe,
    budget: Option<&MemBudget>,
    spill: Option<&mut dyn Spill>,
    mut open: impl FnMut(usize) -> Result<Index<R>>,
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
            let candidates = match stored {
                Some(c) => c,
                None => weak_candidates(&mut open(k)?, r)?,
            };
            weak.add(unheld(&candidates, &s.table, r), budget)?;
        }
        let ions = weak.recurring(r, budget)?;
        table.append_unheld(&ions, r.align.ion.ppm, r.fill.rt_tol);
    }
    let map: RtMap = build(&table, &r.rt);
    let map = map.ok().then_some(&map);
    let mut filled = 0;
    for k in 0..n {
        if !r.fill_gaps {
            break;
        }
        let mut index = open(k)?;
        filled += fill_file(&mut index, &mut table, k, map, &r.detect, &r.fill)?;
    }
    table.sort_rows();
    Ok((table, filled))
}
