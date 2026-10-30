//! The feature table with its m/z corrected. Only the `mz` column (and the ions of a family, `member_mz`) is rewritten, in a copy of the file: same length, same meta, same ids (the
//! rows keep their place, so an id is still the position of its row). The m/z of a row is recomputed the way the alignment writes it (`align::AreaMean`: the mean of the m/z of the
//! files that detected it, weighted by their areas), each file's m/z first corrected with its model at the RT of its peak.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;

use crate::align::AreaMean;
use crate::aligned::CELL_DETECTED;
use crate::store::{f64_column_span, origin_from_meta, read_table_from};

use super::analyse::FileResult;

/// The corrected m/z of a measured one under an error in ppm.
fn corrected(mz: f64, err_ppm: f64) -> f64 {
    mz / (1.0 + err_ppm * 1e-6)
}

/// The result for each column of the table: by the SHA-256 of the file, else by position when the table and the check have the same number of files.
fn by_column<'a>(
    inputs: &[(String, String)],
    results: &'a [FileResult],
) -> Vec<Option<&'a FileResult>> {
    inputs
        .iter()
        .enumerate()
        .map(|(k, (_, sha))| {
            let hit = results.iter().find(|r| !sha.is_empty() && r.sha256 == *sha);
            hit.or_else(|| (inputs.len() == results.len()).then(|| &results[k]))
                .filter(|r| r.fitted)
        })
        .collect()
}

fn put(out: &mut [u8], span: std::ops::Range<usize>, values: &[f64]) {
    for (c, v) in out[span].chunks_exact_mut(8).zip(values) {
        c.copy_from_slice(&v.to_le_bytes());
    }
}

pub(super) fn correct<T: ReadAt>(
    table: &mut T,
    size: u64,
    results: &[FileResult],
) -> Result<Vec<u8>> {
    let (t, meta) = read_table_from(table, size)?;
    if !t.q3.is_empty() {
        return Err(Error::new("err.flow.massacc.mrm"));
    }
    let inputs = origin_from_meta(&meta).inputs;
    if inputs.len() != t.n_files {
        return Err(Error::new("error.badMzfeat"));
    }
    let cols = by_column(&inputs, results);
    let mut mz = t.mz.clone();
    for (row, m) in mz.iter_mut().enumerate() {
        let (mut mean, mut any) = (AreaMean::default(), false);
        for (k, fr) in cols.iter().enumerate() {
            let c = t.cell(row, k);
            if t.state[c] != CELL_DETECTED || t.area[c].is_nan() || t.area[c] <= 0.0 {
                continue;
            }
            any |= fr.is_some();
            let here = fr.map_or(t.mz[row], |r| {
                corrected(
                    t.mz[row],
                    r.model.predict(&r.coef, t.mz[row], f64::from(t.apex_rt[c])),
                )
            });
            mean.add(here, t.area[c]);
        }
        if any && !mean.is_empty() {
            *m = mean.mean();
        }
    }
    let mut members = t.member_mz.clone();
    for (row, (new, old)) in mz.iter().zip(&t.mz).enumerate() {
        let factor = new / old;
        for j in t.members(row) {
            members[j] *= factor;
        }
    }
    let mut out = vec![0u8; size as usize];
    table.read_at(0, &mut out)?;
    let (mz_span, member_span) = (
        f64_column_span(&out, "mz")?,
        f64_column_span(&out, "member_mz")?,
    );
    put(&mut out, mz_span, &mz);
    put(&mut out, member_span, &members);
    Ok(out)
}
