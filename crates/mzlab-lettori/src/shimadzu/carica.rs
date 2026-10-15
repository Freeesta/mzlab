//! The data of a Shimadzu file read once at `apri`: the raw stream, the index of the scans (time, TIC, event, SIM or scan), the chromatograms (TIC, and one SRM per transition of an MRM
//! file) and the events seen. What a spectrum holds is decoded on request (`rec.rs`); the stream layout is in docs/agenti/shimadzu.md.

use std::collections::HashMap;

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{Limiti, VoceScansione};
use mzlab_core::mzml::{ChromKind, Chromatogram};
use mzlab_core::ole::Ole;

use super::mrm::{self, Tracce};
use super::rec::{self, Forma};

const UNITA: &str = "number of detector counts";
/// A list of at most this many m/z that repeats in at least `SIM_RIPETUTI` records is a SIM list.
const SIM_PUNTI: usize = 64;
const SIM_RIPETUTI: u32 = 3;

/// The kind of data, told by the storage that holds the raw stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fam {
    /// `GCMS Raw Data` of a `.qgd`.
    Gc,
    /// `Mass Raw Data` of a `.lcd` (LCMS-2020/2050).
    Lc,
    /// `TLM Raw Data` of a `.lcd` of a triple quadrupole (MRM).
    Tlm,
}

impl Fam {
    pub(super) fn storage(self) -> &'static str {
        match self {
            Fam::Gc => "GCMS Raw Data",
            Fam::Lc => "Mass Raw Data",
            Fam::Tlm => "TLM Raw Data",
        }
    }
}

/// The kinds of data, in the order they are looked for.
pub(super) fn famiglie() -> [Fam; 3] {
    [Fam::Gc, Fam::Tlm, Fam::Lc]
}

/// The kind of data of the container: the first storage whose `MS Raw Data` holds bytes (the storages of the other families exist, empty, in every `.lcd`).
pub(super) fn scegli(ole: &Ole) -> Option<Fam> {
    famiglie().into_iter().find(|f| {
        ole.read(&format!("{}/MS Raw Data", f.storage()))
            .is_some_and(|b| !b.is_empty())
    })
}

/// The scans that share one filter text, and the m/z range seen.
pub(super) struct Evento {
    pub filtro: String,
    pub n: usize,
    pub mz: (f64, f64),
}

pub(super) struct Dati {
    pub raw: Vec<u8>,
    /// (offset, length) in `raw` and layout of each scan of the index.
    pub pos: Vec<(usize, usize, Forma)>,
    pub indice: Vec<VoceScansione>,
    pub cromat: Vec<Chromatogram>,
    pub eventi: Vec<Evento>,
    /// (Q1, Q3, polarity) of the SRM chromatograms.
    pub transizioni: Vec<(f64, f64, i8)>,
    pub records: usize,
    pub illeggibili: usize,
}

fn manca(n: &str) -> Error {
    Error::new("err.file.shimadzu").with("detail", format!("stream {n} is missing"))
}

fn u64_at(b: &[u8], i: usize) -> f64 {
    b.get(8 * i..8 * i + 8)
        .and_then(|s| s.try_into().ok())
        .map_or(0.0, |s| u64::from_le_bytes(s) as f64)
}

fn tempo(rt: &[u8], i: usize) -> f64 {
    rt.get(4 * i..4 * i + 4)
        .and_then(|s| s.try_into().ok())
        .map_or(0.0, |s| f64::from(u32::from_le_bytes(s)) / 60_000.0)
}

/// Reads the streams of `fam` and builds the index and the chromatograms.
pub(super) fn carica(ole: &Ole, fam: Fam, limiti: &Limiti) -> Result<Dati> {
    let g = fam.storage();
    let get = |n: &str| ole.read(&format!("{g}/{n}")).ok_or_else(|| manca(n));
    let (raw, idx, rt, tic) = (
        get("MS Raw Data")?,
        get("Spectrum Index")?,
        get("Retention Time")?,
        get("TIC Data")?,
    );
    let n = limiti.voci_ok((rt.len() / 4) as u64)?;
    let mut d = Dati {
        raw: Vec::new(),
        pos: Vec::new(),
        indice: Vec::new(),
        cromat: Vec::new(),
        eventi: Vec::new(),
        transizioni: Vec::new(),
        records: n,
        illeggibili: 0,
    };
    let mut srm = Tracce::default();
    let impronte = if fam == Fam::Tlm {
        d.illeggibili = tlm(&raw, &idx, &rt, n, &mut srm, limiti)?;
        Vec::new()
    } else {
        scansioni(&mut d, (fam, &raw, &idx, &rt, &tic), n, &mut srm, limiti)?
    };
    if !tic.is_empty() {
        d.cromat.push(Chromatogram {
            index: 0,
            id: "TIC".into(),
            kind: ChromKind::Tic,
            q1: None,
            q3: None,
            unit: UNITA.into(),
            time: (0..n).map(|i| tempo(&rt, i)).collect(),
            intensity: (0..n).map(|i| u64_at(&tic, i)).collect(),
        });
    }
    let da = d.cromat.len();
    d.cromat.extend(srm.cromatogrammi(da));
    d.transizioni = srm.serie.iter().map(|s| (s.q1, s.q3, s.pol)).collect();
    segna_sim(&mut d, &impronte);
    d.raw = raw;
    Ok(d)
}

/// The points of the records of `TLM Raw Data` into `srm`; returns the records that are not decoded.
fn tlm(
    raw: &[u8],
    idx: &[u8],
    rt: &[u8],
    n: usize,
    srm: &mut Tracce,
    limiti: &Limiti,
) -> Result<usize> {
    let mut male = 0;
    for (i, off) in mrm::offset_tlm(idx, n, limiti)?.into_iter().enumerate() {
        let Some(r) = mrm::record(raw, off) else {
            male += 1;
            continue;
        };
        let Some(pts) = mrm::punti(&r) else {
            male += 1;
            continue;
        };
        let pol = mrm::polarita_tlm(&r);
        for (q1, q3, y) in pts {
            srm.aggiungi((q1, q3), 100.0, pol, tempo(rt, i), y as f64, limiti)?;
        }
    }
    Ok(male)
}

/// Where `kind` and the point count sit in a `.qgd` record (20 or 24): the one under which most of the first records are valid.
fn offset_ko(offs: &[u64], raw: &[u8], n: usize) -> usize {
    [20, 24]
        .into_iter()
        .max_by_key(|&ko| {
            (0..n.min(16))
                .filter(|&i| {
                    let a = usize::try_from(offs[i]).unwrap_or(usize::MAX);
                    let b = offs.get(i + 1).map_or(raw.len(), |&o| o as usize);
                    a <= b
                        && raw
                            .get(a..b)
                            .is_some_and(|r| rec::forma_gc(r, ko).is_some())
                })
                .count()
        })
        .unwrap_or(20)
}

/// The scan records of `.qgd` and `Mass Raw Data` into `d.pos`/`d.indice` (MRM records of a `.qgd` into `srm`); returns the hash of the m/z list and the m/z range of each scan.
fn scansioni(
    d: &mut Dati,
    (fam, raw, idx, rt, tic): (Fam, &[u8], &[u8], &[u8], &[u8]),
    n: usize,
    srm: &mut Tracce,
    limiti: &Limiti,
) -> Result<Vec<(u64, (f64, f64))>> {
    let offs = rec::offset(idx, n).ok_or_else(|| {
        Error::new("err.file.shimadzu").with("detail", "Spectrum Index does not fit the records")
    })?;
    let ko = offset_ko(&offs, raw, n);
    let mut impronte = Vec::new();
    for i in 0..n {
        let a = usize::try_from(offs[i]).unwrap_or(usize::MAX);
        let b = offs.get(i + 1).map_or(raw.len(), |&o| o as usize);
        let r = (a <= b).then(|| raw.get(a..b)).flatten();
        let f = r.and_then(|r| {
            if fam == Fam::Gc {
                rec::forma_gc(r, ko)
            } else {
                rec::forma_lc(r)
            }
        });
        let (Some(r), Some(f)) = (r, f) else {
            d.illeggibili += 1;
            continue;
        };
        let t = tempo(rt, i);
        if f.mrm {
            for j in 0..f.npk {
                let (q1, q3, y) = rec::punto(r, &f, j);
                srm.aggiungi(
                    (q1.into(), q3.unwrap_or(0).into()),
                    20.0,
                    0,
                    t,
                    y as f64,
                    limiti,
                )?;
            }
            continue;
        }
        let mz = (0..f.npk).map(|j| f64::from(rec::punto(r, &f, j).0) / 20.0);
        let (mn, mx) = mz.fold((f64::MAX, 0.0f64), |(a, b), v| (a.min(v), b.max(v)));
        let filtro = if fam == Fam::Gc {
            "EI".to_string()
        } else {
            format!("Q1 event {}", usize::from(f.ev) + 1)
        };
        limiti.voci_ok(d.indice.len() as u64 + 1)?;
        d.indice.push(VoceScansione {
            id: format!("scan={}", i + 1),
            rt: t,
            livello: 1,
            polarita: 0,
            tic: u64_at(tic, i),
            picco_base: None,
            filtro: format!("{filtro} scan"),
            precursore: None,
            posizione: None,
        });
        d.pos.push((a, b - a, f));
        let h = if f.npk <= SIM_PUNTI {
            rec::impronta(r, &f)
        } else {
            0
        };
        impronte.push((h, (mn, mx)));
    }
    Ok(impronte)
}

/// A scan whose m/z list (at most 64 values) repeats is SIM: its filter says so, and the events are tallied with the m/z range seen.
fn segna_sim(d: &mut Dati, impronte: &[(u64, (f64, f64))]) {
    let mut n: HashMap<u64, u32> = HashMap::new();
    for (h, _) in impronte.iter().filter(|(h, _)| *h != 0) {
        *n.entry(*h).or_default() += 1;
    }
    for (v, (h, (mn, mx))) in d.indice.iter_mut().zip(impronte) {
        if *h != 0 && n.get(h).is_some_and(|&c| c >= SIM_RIPETUTI) {
            v.filtro = v.filtro.replace(" scan", " SIM");
        }
        match d.eventi.iter_mut().find(|e| e.filtro == v.filtro) {
            Some(e) => {
                e.n += 1;
                e.mz = (e.mz.0.min(*mn), e.mz.1.max(*mx));
            }
            None => d.eventi.push(Evento {
                filtro: v.filtro.clone(),
                n: 1,
                mz: (*mn, *mx),
            }),
        }
    }
}
