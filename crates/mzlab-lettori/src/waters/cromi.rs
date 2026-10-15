//! The chromatograms of a folder: the TIC of all the functions (the TICs of the index, merged in the order of time), one SRM (or SIC) per channel of each SIR/MRM function, the TIC of
//! the lock mass function, and the analog traces of the LC (`_CHROMS.INF` with `_CHROnnn.DAT`: pressure, flow, gradient, temperatures, a UV wavelength).

use super::cartella::{f32_at, testo, Cartella};
use super::idx;
use super::lettore::{Fun, LettoreWaters};
use super::mrm;
use mzlab_core::error::Result;
use mzlab_core::mzml::{ChromKind, Chromatogram};

const UNITA: &str = "number of detector counts";
const CHROMS_INF: &str = "_CHROMS.INF";
const INTESTAZIONE: usize = 128;
const RECORD_INF: usize = 85;
const RECORD_DAT: usize = 8;

/// One analog trace: `_CHROMS.INF` record `n - 1` is the file `_CHROnnn.DAT`.
#[derive(Debug, Clone)]
pub(super) struct Analogica {
    pub(super) n: u32,
    pub(super) nome: String,
    pub(super) unita: String,
    pub(super) scala: f64,
    /// The text after the name, as the file wrote it (`$CC$,scale,type,low,high,units`).
    pub(super) grezzo: String,
}

pub(super) fn analogiche(c: &mut Cartella) -> Vec<Analogica> {
    let Some(b) = c.leggi(CHROMS_INF).ok().flatten() else {
        return Vec::new();
    };
    let n = b.len().saturating_sub(INTESTAZIONE) / RECORD_INF;
    if c.limiti.voci_ok(n as u64).is_err() {
        return Vec::new();
    }
    (0..n)
        .map(|i| {
            let r = &b[INTESTAZIONE + i * RECORD_INF..INTESTAZIONE + (i + 1) * RECORD_INF];
            let mut campi = r[4..].split(|&x| x == 0);
            let nome = testo(campi.next().unwrap_or(&[])).trim().to_string();
            let grezzo = testo(campi.next().unwrap_or(&[])).trim().to_string();
            let p: Vec<&str> = grezzo.split(',').collect();
            Analogica {
                n: i as u32 + 1,
                nome,
                unita: p.get(5).map_or(String::new(), |u| u.trim().to_string()),
                scala: p
                    .get(1)
                    .and_then(|s| s.trim().parse().ok())
                    .filter(|x: &f64| x.is_finite())
                    .unwrap_or(1.0),
                grezzo,
            }
        })
        .collect()
}

fn tipo_analogico(a: &Analogica) -> ChromKind {
    let n = a.nome.to_ascii_lowercase();
    if n.contains("pressure") {
        ChromKind::Pressure
    } else if a.unita.eq_ignore_ascii_case("AU") || n.contains(" nm") {
        ChromKind::Pda
    } else {
        ChromKind::Analog
    }
}

fn traccia(c: &mut Cartella, a: &Analogica, indice: usize) -> Result<Option<Chromatogram>> {
    let Some(b) = c.leggi(&format!("_CHRO{:03}.DAT", a.n))? else {
        return Ok(None);
    };
    let n = b.len().saturating_sub(INTESTAZIONE) / RECORD_DAT;
    c.limiti.voci_ok(n as u64)?;
    let (mut time, mut intensity) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for r in b[INTESTAZIONE..].chunks_exact(RECORD_DAT) {
        let (t, v) = (f64::from(f32_at(r, 0)), f64::from(f32_at(r, 4)) * a.scala);
        if t.is_finite() && v.is_finite() {
            time.push(t);
            intensity.push(v);
        }
    }
    Ok(Some(Chromatogram {
        index: indice,
        id: a.nome.clone(),
        kind: tipo_analogico(a),
        q1: None,
        q3: None,
        unit: a.unita.clone(),
        time,
        intensity,
    }))
}

/// The id of a channel of an SRM function: `SRM SIC Q1=277.1 Q3=71 ce=34 function=1 offset=0 name=ALA` (a SIR channel: `SIC 277.1 function=… offset=…`).
fn id_canale(f: &Fun, j: usize, q: Option<(f32, f32)>, ce: Option<f64>, nome: &str) -> String {
    let mut id = match q {
        Some((q1, q3)) if q3 > 0.0 && q3 != q1 => format!("SRM SIC Q1={q1} Q3={q3}"),
        Some((q1, _)) => format!("SIC {q1}"),
        None => "SRM SIC".to_string(),
    };
    if let Some(ce) = ce.filter(|&x| x > 0.0) {
        id.push_str(&format!(" ce={ce}"));
    }
    id.push_str(&format!(" function={} offset={j}", f.f.indice));
    if !nome.is_empty() {
        id.push_str(&format!(" name={}", nome.replace(['\r', '\n'], " ")));
    }
    id
}

/// The chromatograms of one SIR/MRM function; `None` when its DAT cannot be read as channels.
fn canali(l: &mut LettoreWaters, i: usize, primo: usize) -> Result<Option<Vec<Chromatogram>>> {
    let f = &l.funs[i];
    let (Some(dat), Some(larg)) = (f.dat, f.larg) else {
        return Ok(None);
    };
    if f.dat_len > l.c.limiti.byte / 4 || !(larg == 2 || larg == 4) {
        return Ok(None);
    }
    let righe: Vec<_> = f.righe.iter().copied().filter(|r| r.conta > 0).collect();
    let n = righe.iter().map(|r| r.conta as usize).max().unwrap_or(0);
    l.c.limiti
        .voci_ok((n as u64).saturating_mul(righe.len() as u64))?;
    let dato = l.c.leggi_a(dat, 0, f.dat_len)?;
    let f = &l.funs[i];
    let mut col: Vec<Vec<f64>> = vec![Vec::with_capacity(righe.len()); n];
    for r in &righe {
        let (o, len) = idx::estensione(std::slice::from_ref(r), 0, larg, f.dat_len)?;
        let v = mrm::valori(
            &dato[o as usize..(o + len) as usize],
            larg,
            r.conta as usize,
        )?;
        for (j, c) in col.iter_mut().enumerate() {
            c.push(v.get(j).copied().unwrap_or(0.0));
        }
    }
    let tempo: Vec<f64> = righe.iter().map(|r| f64::from(r.rt)).collect();
    let can = f.f.canali();
    Ok(Some(
        col.into_iter()
            .enumerate()
            .map(|(j, intensity)| {
                let q = can.get(j).map(|c| (c.0, c.1));
                let ce = f.ee.as_ref().and_then(|t| t.valore("Collision Energy", j));
                let nome = f.nomi.get(j).map_or("", String::as_str);
                Chromatogram {
                    index: primo + j,
                    id: id_canale(f, j, q, ce, nome),
                    kind: if q.is_some_and(|(a, b)| b > 0.0 && b != a) {
                        ChromKind::Srm
                    } else {
                        ChromKind::Sic
                    },
                    q1: q.map(|c| f64::from(c.0)),
                    q3: q
                        .filter(|c| c.1 > 0.0 && c.1 != c.0)
                        .map(|c| f64::from(c.1)),
                    unit: UNITA.into(),
                    time: tempo.clone(),
                    intensity,
                }
            })
            .collect(),
    ))
}

/// The TIC of the scans of a set of functions, in the order of time.
fn tic<'a>(funs: impl Iterator<Item = &'a Fun>, id: &str, kind: ChromKind) -> Option<Chromatogram> {
    let mut v: Vec<(f32, f32)> = funs
        .flat_map(|f| f.righe.iter().map(|r| (r.rt, r.tic)))
        .collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    Some(Chromatogram {
        index: 0,
        id: id.into(),
        kind,
        q1: None,
        q3: None,
        unit: UNITA.into(),
        time: v.iter().map(|x| f64::from(x.0)).collect(),
        intensity: v.iter().map(|x| f64::from(x.1)).collect(),
    })
}

pub(super) fn tutte(l: &mut LettoreWaters) -> Result<Vec<Chromatogram>> {
    let mut out: Vec<Chromatogram> = Vec::new();
    out.extend(tic(
        l.funs.iter().filter(|f| !f.f.riferimento()),
        "TIC",
        ChromKind::Tic,
    ));
    for f in l.funs.iter().filter(|f| f.f.riferimento()) {
        let id = format!("lock mass function={} TIC", f.f.indice);
        out.extend(tic(std::iter::once(f), &id, ChromKind::Other));
    }
    for i in 0..l.funs.len() {
        if !l.funs[i].f.mirato() {
            continue;
        }
        match canali(l, i, out.len() + 1)? {
            Some(v) => out.extend(v),
            None => {
                let a = format!("{}: function {} channels", l.c.nome, l.funs[i].f.indice);
                l.avvisi.push(("meta.nonDecodificato".into(), a));
            }
        }
    }
    for a in l.analogiche.clone() {
        let n = out.len();
        out.extend(traccia(&mut l.c, &a, n)?);
    }
    for (i, h) in out.iter_mut().enumerate() {
        h.index = i;
    }
    Ok(out)
}
