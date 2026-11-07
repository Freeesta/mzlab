//! The two `.mzidx` of a sample whose scans may be profile (as they are, and centroided) from one decoding of its scans, with the fingerprints of the
//! members asked for at the end, as `indicizza_lettore_con`.

use super::{campione_di, sorgente, tabella, Impronta};
use crate::error::Result;
use crate::lettori::Lettore as _;
use crate::mzidx_lettore;

/// The two `.mzidx` of a sample whose scans may be profile, from ONE decoding of them: `out` gets the index with the scans as the file has them (what
/// `indicizza_lettore_con` writes) and `out_cen` the one with the profile scans as centroids (what it writes for a reader wrapped by `centroidato`).
/// The bytes are those of the two separate builds; the budget is shared (half each) and so is the scratch file (`mzidx_spill::Meta`).
#[allow(clippy::too_many_arguments)]
pub fn indicizza_lettore_doppio(
    l: &mut crate::lettori_centroidi::Centroidato,
    c: usize,
    membri: &mut dyn FnMut() -> Result<Vec<Impronta>>,
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
    let mut m0 = cell.as_ref().map(|sp| Meta { sp, quale: 0 });
    let mut m1 = cell.as_ref().map(|sp| Meta { sp, quale: 1 });
    let mut prima = crate::mzidx::Prima::new(meta, m0.as_mut().map(|m| m as &mut dyn Spill));
    let mut prima_cen = crate::mzidx::Prima::new(meta, m1.as_mut().map(|m| m as &mut dyn Spill));
    let (t, t_cen) = mzidx_lettore::righe_prima_doppie(l, c, &mut prima, &mut prima_cen)?;
    let campione = campione_di(l.semplice(), c)?;
    let none = String::new;
    let p = tabella(t, l.semplice(), c, none(), none(), none())?;
    let mut p_cen = tabella(t_cen, l.semplice(), c, none(), none(), none())?;
    fase(3);
    let n = l.indice(c)?.len();
    // as `indicizza_lettore_con`: the members are asked for once, where the first source is written; the second index has the same
    let (mut sha256, mut src) = (String::new(), String::new());
    let mut tardi = || -> Result<String> {
        let (source, sha, _) = sorgente(&membri()?, &campione, name);
        (sha256, src) = (sha, source.clone());
        Ok(source)
    };
    let st = crate::mzidx::build_dopo(
        &p.table,
        prima,
        &mut |on| (0..n).try_for_each(|k| mzidx_lettore::spettro(l.semplice(), c, k).map(&mut *on)),
        out,
        meta,
        true,
        Some(&mut tardi),
    )?;
    p_cen.table.source_json = src;
    let st_cen = crate::mzidx::build_dopo(
        &p_cen.table,
        prima_cen,
        &mut |on| (0..n).try_for_each(|k| mzidx_lettore::spettro(l, c, k).map(&mut *on)),
        out_cen,
        meta,
        true,
        None,
    )?;
    Ok(((st, sha256.clone()), (st_cen, sha256)))
}
