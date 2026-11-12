//! The two `.mzidx` of a sample whose scans may be profile (as they are, and centroided) from one decoding of its scans, with the fingerprints of the
//! members asked for at the end, as `index_reader_with`.

use super::{sample_of, sorgente, table, with_lost, Fingerprint};
use crate::error::Result;
use crate::mzidx_reader;
use crate::readers::Reader as _;

/// The two `.mzidx` of a sample whose scans may be profile, from ONE decoding of them: `out` gets the index with the scans as the file has them (what
/// `index_reader_with` writes) and `out_cen` the one with the profile scans as centroids (what it writes for a reader wrapped by `centroidato`).
/// The bytes are those of the two separate builds; the budget is shared (half each) and so is the scratch file (`mzidx_spill::Meta`).
#[allow(clippy::too_many_arguments)]
pub fn index_reader_twice(
    l: &mut crate::readers_centroids::Centroidato,
    c: usize,
    members: &mut dyn FnMut() -> Result<Vec<Fingerprint>>,
    name: &str,
    out: &mut dyn crate::mzidx::Output,
    out_cen: &mut dyn crate::mzidx::Output,
    budget: usize,
    spill: Option<&mut dyn crate::mzidx_spill::Spill>,
    fase: &mut dyn FnMut(u32),
) -> Result<((crate::mzidx::Stats, String), (crate::mzidx::Stats, String))> {
    use crate::mzidx_spill::{Meta, Spill};
    let meta = budget / 2;
    let cell = spill.map(std::cell::RefCell::new);
    let mut m0 = cell.as_ref().map(|sp| Meta { sp, which: 0 });
    let mut m1 = cell.as_ref().map(|sp| Meta { sp, which: 1 });
    let mut before = crate::mzidx::First::new(meta, m0.as_mut().map(|m| m as &mut dyn Spill));
    let mut first_cen = crate::mzidx::First::new(meta, m1.as_mut().map(|m| m as &mut dyn Spill));
    let (t, t_cen) = mzidx_reader::rows_before_double(l, c, &mut before, &mut first_cen)?;
    let sample = sample_of(l.simple(), c)?;
    let none = String::new;
    let p = table(t, l.simple(), c, none(), none(), none())?;
    let mut p_cen = table(t_cen, l.simple(), c, none(), none(), none())?;
    fase(3);
    let n = l.index(c)?.len();
    // as `index_reader_with`: the members are asked for once, where the first source is written; the second index has the same
    let (mut sha256, mut src) = (String::new(), String::new());
    // the losses are asked once the scans are decoded, as `index_reader_with` asks them
    let cell = std::cell::RefCell::new(&mut *l);
    let mut late = || -> Result<String> {
        let (source, sha, _) = sorgente(&members()?, &sample, name);
        let source = with_lost(source, &cell.borrow_mut().simple().losses(c)?);
        (sha256, src) = (sha, source.clone());
        Ok(source)
    };
    let st = crate::mzidx::build_after(
        &p.table,
        before,
        &mut |on| {
            (0..n).try_for_each(|k| {
                mzidx_reader::spettro(cell.borrow_mut().simple(), c, k).map(&mut *on)
            })
        },
        out,
        meta,
        true,
        Some(&mut late),
    )?;
    p_cen.table.source_json = src;
    let st_cen = crate::mzidx::build_after(
        &p_cen.table,
        first_cen,
        &mut |on| (0..n).try_for_each(|k| mzidx_reader::spettro(l, c, k).map(&mut *on)),
        out_cen,
        meta,
        true,
        None,
    )?;
    Ok(((st, sha256.clone()), (st_cen, sha256)))
}
