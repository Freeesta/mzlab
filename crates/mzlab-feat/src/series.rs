//! A series of files from the indexes to the aligned table: the analysis of every file (`analyse_file`), the alignment in the mode of the recipe
//! and the gap filling (`align_and_fill`). One index is open at a time; the tables of the files are the only thing kept between the steps.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::align::align;
use crate::famalign::align_families;
use crate::family::analyse_families;
use crate::gapfill::fill_file;
use crate::pipeline::analyse;
use crate::recipe::{Mode, Recipe};
use crate::rtmap::{build, RtMap};
use crate::table::{Aligned, FileFeatures};

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

/// Aligns the tables in the mode of the recipe and fills the cells without a peak, file by file: `open(k)` gives the index of file `k`.
/// Returns the table and the number of cells filled.
pub fn align_and_fill<R: ReadAt>(
    files: &[FileFeatures],
    r: &Recipe,
    mut open: impl FnMut(usize) -> Result<Index<R>>,
) -> Result<(Aligned, usize)> {
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let mut table = match r.mode {
        Mode::Ion => align(&refs, &r.align.ion),
        Mode::Family => align_families(&refs, &r.align),
    };
    let map: RtMap = build(&table, &r.rt);
    let map = map.ok().then_some(&map);
    let mut filled = 0;
    for k in 0..files.len() {
        let mut index = open(k)?;
        filled += fill_file(&mut index, &mut table, k, map, &r.detect, &r.fill)?;
    }
    Ok((table, filled))
}
