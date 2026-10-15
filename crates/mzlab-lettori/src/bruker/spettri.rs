//! Which spectra a timsTOF run has and how a frame is cut into them (docs/agenti/bruker.md): an MS1 frame is one spectrum, a DDA-PASEF frame one per precursor, a diaPASEF frame one
//! per isolation window of its group, a PRM-PASEF frame one per target, any other frame one. Mobility is summed: the points of a spectrum with the same TOF index add up.

use super::blocco::Picchi;
use super::tabelle::{Dati, Finestra};
use std::collections::BTreeMap;

/// A spectrum of the run: a frame (index into `Dati::frames`) and the scans of it that make the spectrum, `[begin, end)`.
#[derive(Debug, Clone)]
pub struct Desc {
    pub frame: usize,
    pub scan: Vec<(u32, u32)>,
    pub livello: u8,
    pub id: String,
    pub filtro: String,
    pub iso: Option<Finestra>,
    pub precursore: Option<i64>,
    pub carica: Option<i64>,
    /// Selected ion m/z (monoisotopic when the file says it) or the isolation target.
    pub mz_prec: Option<f64>,
}

fn fmt_mz(x: f64) -> String {
    format!("{x:.4}")
}

/// The spectra of the run, in the order of the frames.
pub fn costruisci(d: &Dati) -> Vec<Desc> {
    let tutto = |f: usize| vec![(0, u32::try_from(d.frames[f].scansioni).unwrap_or(u32::MAX))];
    let mut out: Vec<Desc> = Vec::new();
    let mut pasef: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (i, r) in d.pasef.iter().enumerate() {
        pasef.entry(r.0).or_default().push(i);
    }
    let mut prm: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (i, r) in d.prm.iter().enumerate() {
        prm.entry(r.0).or_default().push(i);
    }
    for (fi, f) in d.frames.iter().enumerate() {
        let n0 = out.len();
        match f.tipo {
            0 => out.push(Desc {
                frame: fi,
                scan: tutto(fi),
                livello: 1,
                id: format!("frame={}", f.id),
                filtro: "TOF MS".into(),
                iso: None,
                precursore: None,
                carica: None,
                mz_prec: None,
            }),
            8 if pasef.contains_key(&f.id) => {
                // rows of the same precursor in one frame are one spectrum
                let mut per_prec: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
                for &i in &pasef[&f.id] {
                    per_prec.entry(d.pasef[i].1).or_default().push(i);
                }
                // the spectra of a frame follow the mobility ramp: by the first scan of their window
                let mut per_prec: Vec<(i64, Vec<usize>)> = per_prec.into_iter().collect();
                per_prec.sort_by_key(|(_, r)| r.iter().map(|&i| d.pasef[i].2.scan.0).min());
                for (p, righe) in per_prec {
                    let w = d.pasef[righe[0]].2;
                    let pr = d.precursori.get(&p);
                    out.push(Desc {
                        frame: fi,
                        scan: righe.iter().map(|&i| d.pasef[i].2.scan).collect(),
                        livello: 2,
                        id: format!("frame={} precursor={p}", f.id),
                        filtro: format!("TOF PASEF MS2 {}@{}", fmt_mz(w.mz), w.ce),
                        iso: Some(w),
                        precursore: Some(p),
                        carica: pr.and_then(|x| x.carica),
                        mz_prec: pr.and_then(|x| x.mz).or(Some(w.mz)),
                    });
                }
            }
            9 => {
                let g = d.dia_frame.get(&f.id).and_then(|g| d.dia_gruppi.get(g));
                for (k, w) in g.into_iter().flatten().enumerate() {
                    out.push(Desc {
                        frame: fi,
                        scan: vec![w.scan],
                        livello: 2,
                        id: format!("frame={} window={}", f.id, k + 1),
                        filtro: format!("TOF diaPASEF MS2 {}@{}", fmt_mz(w.mz), w.ce),
                        iso: Some(*w),
                        precursore: None,
                        carica: None,
                        mz_prec: Some(w.mz),
                    });
                }
            }
            10 => {
                for &i in prm.get(&f.id).into_iter().flatten() {
                    let (_, t, w) = d.prm[i];
                    out.push(Desc {
                        frame: fi,
                        scan: vec![w.scan],
                        livello: 2,
                        id: format!("frame={} target={t}", f.id),
                        filtro: format!("TOF PRM MS2 {}@{}", fmt_mz(w.mz), w.ce),
                        iso: Some(w),
                        precursore: None,
                        carica: None,
                        mz_prec: Some(w.mz),
                    });
                }
            }
            _ => {}
        }
        if out.len() == n0 {
            // a frame with its own isolation (MRM, MALDI MS/MS) or none: one spectrum
            let info = d.info_frame.get(&f.id);
            let ms2 = info.is_some() || f.tipo != 0;
            out.push(Desc {
                frame: fi,
                scan: tutto(fi),
                livello: if ms2 { 2 } else { 1 },
                id: format!("frame={}", f.id),
                filtro: match info {
                    Some(i) => format!("TOF MS2 {}@{}", fmt_mz(i.mz), i.ce),
                    None => "TOF MS".into(),
                },
                iso: info.map(|i| Finestra {
                    scan: (0, 0),
                    mz: i.mz,
                    larghezza: i.larghezza,
                    ce: i.ce,
                }),
                precursore: None,
                carica: info.and_then(|i| i.carica),
                mz_prec: info.map(|i| i.mz),
            });
        }
    }
    out
}

/// The points of `p` whose scan is in one of `scan`, added by TOF index (ordered by TOF, and so by m/z).
pub fn somma(p: &Picchi, scan: &[(u32, u32)]) -> Vec<(u32, u64)> {
    let mut v: Vec<(u32, u32)> = (0..p.len())
        .filter(|&i| scan.iter().any(|&(a, b)| p.scan[i] >= a && p.scan[i] < b))
        .map(|i| (p.tof[i], p.int[i]))
        .collect();
    v.sort_unstable_by_key(|x| x.0);
    let mut out: Vec<(u32, u64)> = Vec::with_capacity(v.len());
    for (t, y) in v {
        match out.last_mut() {
            Some(l) if l.0 == t => l.1 += u64::from(y),
            _ => out.push((t, u64::from(y))),
        }
    }
    out
}
