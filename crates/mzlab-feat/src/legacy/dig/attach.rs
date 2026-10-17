//! The MS2 of the parent and of the candidates of a Dig.

use super::*;

/// The consensus MS2 of every row of `rows` (with their limits), reading every file once: `None` for a row without scans.
pub(super) fn ms2_of<R: ReadAt>(
    ix: &mut [Index<R>],
    mzs: &[f64],
    lims: &[&[(f64, f64)]],
) -> Res<Vec<(usize, Option<Consensus>)>> {
    let m2: Vec<Ms2Index> = ix.iter().map(Ms2Index::of).collect();
    let picks: Vec<Vec<Pick>> = mzs
        .iter()
        .zip(lims)
        .map(|(&mz, l)| select(&m2, mz, l, MS2_PPM))
        .collect();
    let mut spectra: HashMap<(usize, usize), (Vec<f64>, Vec<f64>)> = HashMap::new();
    for (k, index) in ix.iter_mut().enumerate() {
        let want: Vec<usize> = picks
            .iter()
            .flatten()
            .filter(|p| p.0 == k)
            .map(|p| p.1)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if want.is_empty() {
            continue;
        }
        for (s, sp) in want
            .iter()
            .zip(read_scans(index, &want).map_err(|e| e.to_string())?)
        {
            spectra.insert((k, *s), sp);
        }
    }
    Ok(picks
        .iter()
        .map(|p| {
            let sp: Vec<(Vec<f64>, Vec<f64>)> = p
                .iter()
                .filter_map(|x| spectra.get(&(x.0, x.1)).cloned())
                .collect();
            (p.len(), (!p.is_empty()).then(|| consensus(&sp)))
        })
        .collect())
}

/// The MS2 of the parent and of every candidate; the candidates get theirs with the similarity to the parent (none without the parent's).
pub(super) fn attach_ms2<R: ReadAt>(
    ix: &mut [Index<R>],
    t: &Aligned,
    parent: &Parent,
    prow: Option<usize>,
    cands: &mut [Cand],
) -> Res<Option<Consensus>> {
    let plim = prow
        .map(|r| limits(t, r))
        .unwrap_or_else(|| vec![(f64::NAN, f64::NAN); t.n_files]);
    let mzs: Vec<f64> = std::iter::once(parent.mz)
        .chain(cands.iter().map(|c| c.mz))
        .collect();
    let lims: Vec<&[(f64, f64)]> = std::iter::once(plim.as_slice())
        .chain(cands.iter().map(|c| c.limits.as_slice()))
        .collect();
    let mut m = ms2_of(ix, &mzs, &lims)?.into_iter();
    let parent_ms2 = m.next().and_then(|x| x.1).filter(|_| prow.is_some());
    let pspec: Option<Spec> = parent_ms2
        .as_ref()
        .map(|c| prepare(&c.mz, &c.rel, parent.mz));
    for (c, (n, cons)) in cands.iter_mut().zip(m) {
        c.ms2_scans = n;
        if let (Some(ps), Some(cons)) = (&pspec, cons) {
            let sp = prepare(&cons.mz, &cons.rel, c.mz);
            c.ms2 = Some(CandMs2 {
                sim: pair_components(ps, parent.mz, &sp, c.mz),
                cons,
            });
        }
    }
    Ok(parent_ms2)
}
