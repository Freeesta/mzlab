//! A series of files from the indexes to the aligned table: the analysis of every file (`analyse_file`), the alignment in the mode of the recipe
//! and the gap filling (`align_and_fill`). One index is open at a time; the tables of the files are the only thing kept between the steps.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::align::align;
use crate::famalign::align_families;
use crate::family::analyse_families;
use crate::gapfill::fill_file;
use crate::isotopes::group;
use crate::pipeline::analyse;
use crate::quality::assess;
use crate::recipe::{Mode, Recipe, WEAK_RATIO};
use crate::rtmap::{build, RtMap};
use crate::table::{Aligned, FileFeatures, FLAG_GOOD, FLAG_ISOTOPE};
use crate::trace::{detect, Params};

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

/// The weak peaks of a file: the ones between `WEAK_RATIO` of the minimum height and the minimum height, good by the quality limits, that are not an
/// isotopologue and that `strong` (the table of the file) does not already have (same m/z within the ppm of the alignment and RT within its RT gap).
fn weak_tier<R: ReadAt>(
    index: &mut Index<R>,
    strong: &FileFeatures,
    r: &Recipe,
) -> Result<FileFeatures> {
    let p = Params {
        min_height: r.detect.min_height * WEAK_RATIO,
        ..r.detect.clone()
    };
    let mut w = detect(index, &p)?;
    assess(index, &mut w, &p, &r.steps.quality)?;
    group(&mut w, &r.steps.iso);
    let (ppm, gap) = (r.align.ion.ppm, r.align.ion.rt_gap);
    let mut out = FileFeatures {
        dt: w.dt,
        ..FileFeatures::default()
    };
    for i in (0..w.len()).filter(|&i| {
        f64::from(w.height[i]) < r.detect.min_height
            && w.flags[i] & FLAG_GOOD != 0
            && w.flags[i] & FLAG_ISOTOPE == 0
    }) {
        let tol = w.mz[i] * ppm * 1e-6;
        let from = strong.mz.partition_point(|&m| m < w.mz[i] - tol);
        let held = (from..strong.len())
            .take_while(|&j| strong.mz[j] <= w.mz[i] + tol)
            .any(|j| f64::from(strong.rt[j] - w.rt[i]).abs() <= gap);
        if !held {
            out.push_lean(&w, i);
        }
    }
    Ok(out)
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
        Mode::Ion => align(&refs, &r.align.ion),
        Mode::Family => align_families(&refs, &r.align),
    };
    let pos = |k: usize| {
        Ok(FileFeatures {
            mz: files[k].mz.clone(),
            rt: files[k].rt.clone(),
            ..FileFeatures::default()
        })
    };
    finish(table, &mut { pos }, files.len(), r, open)
}

/// Weak tier, map of RT and gap filling on the aligned table of the strong peaks. `strong(k)` gives the table of file `k` (only its m/z and RT are read).
pub(crate) fn finish<R: ReadAt>(
    mut table: Aligned,
    strong: &mut dyn FnMut(usize) -> Result<FileFeatures>,
    n: usize,
    r: &Recipe,
    mut open: impl FnMut(usize) -> Result<Index<R>>,
) -> Result<(Aligned, usize)> {
    if r.weak_files > 0 {
        let mut weak = Vec::with_capacity(n);
        for k in 0..n {
            weak.push(weak_tier(&mut open(k)?, &strong(k)?, r)?);
        }
        let refs: Vec<&FileFeatures> = weak.iter().collect();
        let mut ions = align(&refs, &r.align.ion);
        ions.retain_recurring(r.weak_files);
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
    Ok((table, filled))
}
