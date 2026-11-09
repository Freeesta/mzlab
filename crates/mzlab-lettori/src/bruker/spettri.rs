//! Which spectra a timsTOF run has and how a frame is cut into them (docs/agenti/bruker.md): an MS1 frame is one spectrum, a DDA-PASEF frame one per precursor, a diaPASEF frame one
//! per isolation window of its group, a PRM-PASEF frame one per target, any other frame one. Mobility is summed: the points of a spectrum with the same TOF index add up.

use super::blocco::Peaks;

use super::tabelle::{Data, InfoFrame, Window};

use std::collections::BTreeMap;

/// A spectrum of the run: a frame (index into `Dati::frames`) and the scans of it that make the spectrum, `[begin, end)`.
#[derive(Debug, Clone)]
pub struct Desc {
    pub frame: usize,
    pub scan: Vec<(u32, u32)>,
    pub livello: u8,
    pub id: String,
    pub filter: String,
    pub iso: Option<Window>,
    pub precursor: Option<i64>,
    pub load: Option<i64>,
    /// Selected ion m/z (monoisotopic when the file says it) or the isolation target.
    pub mz_prec: Option<f64>,
}

fn fmt_mz(x: f64) -> String {
    format!("{x:.4}")
}

/// The spectra of the run, in the order of the frames.
pub fn build(d: &Data) -> Vec<Desc> {
    let per_frame = |rows: &[(i64, i64, Window)]| {
        let mut m: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
        for (i, r) in rows.iter().enumerate() {
            m.entry(r.0).or_default().push(i);
        }
        m
    };
    let (pasef, prm) = (per_frame(&d.pasef), per_frame(&d.prm));
    let mut out: Vec<Desc> = Vec::new();
    for (fi, f) in d.frames.iter().enumerate() {
        let n0 = out.len();
        match f.kind {
            0 => out.push(whole_frame(d, fi, None)),
            8 => dda(d, fi, pasef.get(&f.id).map_or(&[][..], |v| v), &mut out),
            9 => dia(d, fi, &mut out),
            10 => prm_row(d, fi, prm.get(&f.id).map_or(&[][..], |v| v), &mut out),
            _ => {}
        }
        if out.len() == n0 {
            // a frame with its own isolation (MRM, MALDI MS/MS) or none: one spectrum
            out.push(whole_frame(d, fi, d.info_frame.get(&f.id)));
        }
    }
    out
}

fn whole_scans(d: &Data, f: usize) -> Vec<(u32, u32)> {
    vec![(0, u32::try_from(d.frames[f].scans).unwrap_or(u32::MAX))]
}

/// A frame as one spectrum: MS1, or MS2 when the frame table says it has an isolation.
fn whole_frame(d: &Data, fi: usize, info: Option<&InfoFrame>) -> Desc {
    let f = &d.frames[fi];
    let ms2 = info.is_some() || f.kind != 0;
    Desc {
        frame: fi,
        scan: whole_scans(d, fi),
        livello: if ms2 { 2 } else { 1 },
        id: format!("frame={}", f.id),
        filter: match info {
            Some(i) => format!("TOF MS2 {}@{}", fmt_mz(i.mz), i.ce),
            None => "TOF MS".into(),
        },
        iso: info.map(|i| Window {
            scan: (0, 0),
            mz: i.mz,
            width: i.larghezza,
            ce: i.ce,
        }),
        precursor: None,
        load: info.and_then(|i| i.charge),
        mz_prec: info.map(|i| i.mz),
    }
}

/// A MS2 spectrum of one isolation window of a frame.
fn ms2(fi: usize, id: String, kind: &str, w: Window) -> Desc {
    Desc {
        frame: fi,
        scan: vec![w.scan],
        livello: 2,
        id,
        filter: format!("TOF {kind} MS2 {}@{}", fmt_mz(w.mz), w.ce),
        iso: Some(w),
        precursor: None,
        load: None,
        mz_prec: Some(w.mz),
    }
}

/// DDA-PASEF: the rows of the same precursor in one frame are one spectrum, in the order of the mobility ramp (the first scan of their window).
fn dda(d: &Data, fi: usize, rows: &[usize], out: &mut Vec<Desc>) {
    let mut per_prec: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for &i in rows {
        per_prec.entry(d.pasef[i].1).or_default().push(i);
    }
    let mut per_prec: Vec<(i64, Vec<usize>)> = per_prec.into_iter().collect();
    per_prec.sort_by_key(|(_, r)| r.iter().map(|&i| d.pasef[i].2.scan.0).min());
    for (p, rows) in per_prec {
        let w = d.pasef[rows[0]].2;
        let pr = d.precursori.get(&p);
        let mut s = ms2(
            fi,
            format!("frame={} precursor={p}", d.frames[fi].id),
            "PASEF",
            w,
        );
        s.scan = rows.iter().map(|&i| d.pasef[i].2.scan).collect();
        s.precursor = Some(p);
        s.load = pr.and_then(|x| x.load);
        s.mz_prec = pr.and_then(|x| x.mz).or(Some(w.mz));
        out.push(s);
    }
}

/// diaPASEF: one spectrum per window of the group of the frame.
fn dia(d: &Data, fi: usize, out: &mut Vec<Desc>) {
    let id = d.frames[fi].id;
    let g = d.dia_frame.get(&id).and_then(|g| d.dia_groups.get(g));
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
fn prm_row(d: &Data, fi: usize, rows: &[usize], out: &mut Vec<Desc>) {
    for &i in rows {
        let (_, t, w) = d.prm[i];
        out.push(ms2(
            fi,
            format!("frame={} target={t}", d.frames[fi].id),
            "PRM",
            w,
        ));
    }
}

/// The point ranges of `p` in the scan ranges `scan`, merged first so that overlapping ranges do not count a point twice.
pub fn stretches(p: &Peaks, scan: &[(u32, u32)]) -> Vec<std::ops::Range<usize>> {
    let mut r: Vec<(u32, u32)> = scan.iter().copied().filter(|x| x.0 < x.1).collect();
    r.sort_unstable();
    let mut joined: Vec<(u32, u32)> = Vec::with_capacity(r.len());
    for (a, b) in r {
        match joined.last_mut() {
            Some(u) if a <= u.1 => u.1 = u.1.max(b),
            _ => joined.push((a, b)),
        }
    }
    joined.into_iter().map(|(a, b)| p.points(a, b)).collect()
}

/// The points of `p` whose scan is in one of `scan`, added by TOF index (ordered by TOF, and so by m/z).
pub fn summed(p: &Peaks, scan: &[(u32, u32)]) -> Vec<(u32, u64)> {
    let t = stretches(p, scan);
    let mut v: Vec<(u32, u32)> = Vec::with_capacity(t.iter().map(ExactSizeIterator::len).sum());
    for r in t {
        v.extend(r.map(|i| (p.tof[i], p.int[i])));
    }
    sort_tof(&mut v);
    let mut out: Vec<(u32, u64)> = Vec::with_capacity(v.len());
    for (t, y) in v {
        match out.last_mut() {
            Some(l) if l.0 == t => l.1 += u64::from(y),
            _ => out.push((t, u64::from(y))),
        }
    }
    out
}

/// Sorts by TOF index: least-significant-digit radix on 11-bit digits for many points (a MS1 frame), the library sort for a few. Equal TOF indices are added
/// afterwards, so the order among them does not matter.
fn sort_tof(v: &mut Vec<(u32, u32)>) {
    const BIT: u32 = 11;
    if v.len() < 1024 {
        v.sort_unstable_by_key(|x| x.0);
        return;
    }
    let max = v.iter().map(|x| x.0).max().unwrap_or(0);
    let mut tmp = vec![(0u32, 0u32); v.len()];
    let mut shift = 0;
    while shift < 32 && (max >> shift) > 0 {
        let mut count = [0usize; 1 << BIT];
        for x in v.iter() {
            count[((x.0 >> shift) & ((1 << BIT) - 1)) as usize] += 1;
        }
        let mut pos = 0;
        for c in count.iter_mut() {
            let n = *c;
            *c = pos;
            pos += n;
        }
        for &x in v.iter() {
            let d = ((x.0 >> shift) & ((1 << BIT) - 1)) as usize;
            tmp[count[d]] = x;
            count[d] += 1;
        }
        std::mem::swap(v, &mut tmp);
        shift += BIT;
    }
}
