//! Which spectra a timsTOF run has and how a frame is cut into them (docs/agenti/bruker.md): an MS1 frame is one spectrum, a DDA-PASEF frame one per precursor, a diaPASEF frame one
//! per isolation window of its group, a PRM-PASEF frame one per target, any other frame one. Mobility is summed: the points of a spectrum with the same TOF index add up.

use super::blocco::Picchi;

use super::tabelle::{Dati, Finestra, InfoFrame};

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
    let per_frame = |righe: &[(i64, i64, Finestra)]| {
        let mut m: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
        for (i, r) in righe.iter().enumerate() {
            m.entry(r.0).or_default().push(i);
        }
        m
    };
    let (pasef, prm) = (per_frame(&d.pasef), per_frame(&d.prm));
    let mut out: Vec<Desc> = Vec::new();
    for (fi, f) in d.frames.iter().enumerate() {
        let n0 = out.len();
        match f.tipo {
            0 => out.push(frame_intero(d, fi, None)),
            8 => dda(d, fi, pasef.get(&f.id).map_or(&[][..], |v| v), &mut out),
            9 => dia(d, fi, &mut out),
            10 => riga_prm(d, fi, prm.get(&f.id).map_or(&[][..], |v| v), &mut out),
            _ => {}
        }
        if out.len() == n0 {
            // a frame with its own isolation (MRM, MALDI MS/MS) or none: one spectrum
            out.push(frame_intero(d, fi, d.info_frame.get(&f.id)));
        }
    }
    out
}

fn scansioni_intere(d: &Dati, f: usize) -> Vec<(u32, u32)> {
    vec![(0, u32::try_from(d.frames[f].scansioni).unwrap_or(u32::MAX))]
}

/// A frame as one spectrum: MS1, or MS2 when the frame table says it has an isolation.
fn frame_intero(d: &Dati, fi: usize, info: Option<&InfoFrame>) -> Desc {
    let f = &d.frames[fi];
    let ms2 = info.is_some() || f.tipo != 0;
    Desc {
        frame: fi,
        scan: scansioni_intere(d, fi),
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
    }
}

/// A MS2 spectrum of one isolation window of a frame.
fn ms2(fi: usize, id: String, tipo: &str, w: Finestra) -> Desc {
    Desc {
        frame: fi,
        scan: vec![w.scan],
        livello: 2,
        id,
        filtro: format!("TOF {tipo} MS2 {}@{}", fmt_mz(w.mz), w.ce),
        iso: Some(w),
        precursore: None,
        carica: None,
        mz_prec: Some(w.mz),
    }
}

/// DDA-PASEF: the rows of the same precursor in one frame are one spectrum, in the order of the mobility ramp (the first scan of their window).
fn dda(d: &Dati, fi: usize, righe: &[usize], out: &mut Vec<Desc>) {
    let mut per_prec: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for &i in righe {
        per_prec.entry(d.pasef[i].1).or_default().push(i);
    }
    let mut per_prec: Vec<(i64, Vec<usize>)> = per_prec.into_iter().collect();
    per_prec.sort_by_key(|(_, r)| r.iter().map(|&i| d.pasef[i].2.scan.0).min());
    for (p, righe) in per_prec {
        let w = d.pasef[righe[0]].2;
        let pr = d.precursori.get(&p);
        let mut s = ms2(
            fi,
            format!("frame={} precursor={p}", d.frames[fi].id),
            "PASEF",
            w,
        );
        s.scan = righe.iter().map(|&i| d.pasef[i].2.scan).collect();
        s.precursore = Some(p);
        s.carica = pr.and_then(|x| x.carica);
        s.mz_prec = pr.and_then(|x| x.mz).or(Some(w.mz));
        out.push(s);
    }
}

/// diaPASEF: one spectrum per window of the group of the frame.
fn dia(d: &Dati, fi: usize, out: &mut Vec<Desc>) {
    let id = d.frames[fi].id;
    let g = d.dia_frame.get(&id).and_then(|g| d.dia_gruppi.get(g));
    for (k, w) in g.into_iter().flatten().enumerate() {
        out.push(ms2(
            fi,
            format!("frame={id} window={}", k + 1),
            "diaPASEF",
            *w,
        ));
    }
}

/// PRM-PASEF: one spectrum per row.
fn riga_prm(d: &Dati, fi: usize, righe: &[usize], out: &mut Vec<Desc>) {
    for &i in righe {
        let (_, t, w) = d.prm[i];
        out.push(ms2(
            fi,
            format!("frame={} target={t}", d.frames[fi].id),
            "PRM",
            w,
        ));
    }
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
