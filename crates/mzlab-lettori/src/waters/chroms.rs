//! The chromatograms of a folder: the TIC of all the functions (the TICs of the index, merged in the order of time), one SRM (or SIC) per channel of each SIR/MRM function, the TIC of
//! the lock mass function, and the analog traces of the LC (`_CHROMS.INF` with `_CHROnnn.DAT`: pressure, flow, gradient, temperatures, a UV wavelength).

use super::folder::{f32_at, text, Folder};
use super::idx;
use super::mrm;
use super::reader::{Fun, WatersReader};
use mzlab_core::error::Result;
use mzlab_core::mzml::{ChromKind, Chromatogram};

const UNITS: &str = "number of detector counts";
const CHROMS_INF: &str = "_CHROMS.INF";
const HEADER: usize = 128;
const RECORD_INF: usize = 85;
const RECORD_DAT: usize = 8;

/// One analog trace: `_CHROMS.INF` record `n - 1` is the file `_CHROnnn.DAT`.
#[derive(Debug, Clone)]
pub(super) struct Analog {
    pub(super) n: u32,
    pub(super) name: String,
    pub(super) units: String,
    pub(super) scale: f64,
    /// The text after the name, as the file wrote it (`$CC$,scale,type,low,high,units`).
    pub(super) raw_value: String,
}

pub(super) fn analog(c: &mut Folder) -> Vec<Analog> {
    let Some(b) = c.read(CHROMS_INF).ok().flatten() else {
        return Vec::new();
    };
    let n = b.len().saturating_sub(HEADER) / RECORD_INF;
    if c.limits.ok_entries(n as u64).is_err() {
        return Vec::new();
    }
    (0..n)
        .map(|i| {
            let r = &b[HEADER + i * RECORD_INF..HEADER + (i + 1) * RECORD_INF];
            let mut fields = r[4..].split(|&x| x == 0);
            let nome = text(fields.next().unwrap_or(&[])).trim().to_string();
            let raw_value = text(fields.next().unwrap_or(&[])).trim().to_string();
            let p: Vec<&str> = raw_value.split(',').collect();
            Analog {
                n: i as u32 + 1,
                name: nome,
                units: p.get(5).map_or(String::new(), |u| u.trim().to_string()),
                scale: p
                    .get(1)
                    .and_then(|s| s.trim().parse().ok())
                    .filter(|x: &f64| x.is_finite())
                    .unwrap_or(1.0),
                raw_value,
            }
        })
        .collect()
}

fn analog_kind(a: &Analog) -> ChromKind {
    let n = a.name.to_ascii_lowercase();
    if n.contains("pressure") {
        ChromKind::Pressure
    } else if a.units.eq_ignore_ascii_case("AU") || n.contains(" nm") {
        ChromKind::Pda
    } else {
        ChromKind::Analog
    }
}

fn trace(c: &mut Folder, a: &Analog, indice: usize) -> Result<Option<Chromatogram>> {
    let Some(b) = c.read(&format!("_CHRO{:03}.DAT", a.n))? else {
        return Ok(None);
    };
    let n = b.len().saturating_sub(HEADER) / RECORD_DAT;
    c.limits.ok_entries(n as u64)?;
    let (mut time, mut intensity) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for r in b[HEADER..].chunks_exact(RECORD_DAT) {
        let (t, v) = (f64::from(f32_at(r, 0)), f64::from(f32_at(r, 4)) * a.scale);
        if t.is_finite() && v.is_finite() {
            time.push(t);
            intensity.push(v);
        }
    }
    Ok(Some(Chromatogram {
        index: indice,
        id: a.name.clone(),
        kind: analog_kind(a),
        q1: None,
        q3: None,
        unit: a.units.clone(),
        time,
        intensity,
    }))
}

/// The id of a channel of an SRM function: `SRM SIC Q1=277.1 Q3=71 ce=34 function=1 offset=0 name=ALA` (a SIR channel: `SIC 277.1 function=… offset=…`).
fn channel_id(f: &Fun, j: usize, q: Option<(f32, f32)>, ce: Option<f64>, name: &str) -> String {
    let mut id = match q {
        Some((q1, q3)) if q3 > 0.0 && q3 != q1 => format!("SRM SIC Q1={q1} Q3={q3}"),
        Some((q1, _)) => format!("SIC {q1}"),
        None => "SRM SIC".to_string(),
    };
    if let Some(ce) = ce.filter(|&x| x > 0.0) {
        id.push_str(&format!(" ce={ce}"));
    }
    id.push_str(&format!(" function={} offset={j}", f.f.indice));
    if !name.is_empty() {
        id.push_str(&format!(" name={}", name.replace(['\r', '\n'], " ")));
    }
    id
}

/// The chromatograms of one SIR/MRM function; `None` when its DAT cannot be read as channels.
fn channels(l: &mut WatersReader, i: usize, first_one: usize) -> Result<Option<Vec<Chromatogram>>> {
    let f = &l.funs[i];
    let (Some(dat), Some(larg)) = (f.dat, f.larg) else {
        return Ok(None);
    };
    if f.dat_len > l.c.limits.byte / 4 || !(larg == 2 || larg == 4) {
        return Ok(None);
    }
    let rows: Vec<_> = f.righe.iter().copied().filter(|r| r.count > 0).collect();
    let n = rows.iter().map(|r| r.count as usize).max().unwrap_or(0);
    l.c.limits
        .ok_entries((n as u64).saturating_mul(rows.len() as u64))?;
    let datum = l.c.read_from(dat, 0, f.dat_len)?;
    let f = &l.funs[i];
    let mut col: Vec<Vec<f64>> = vec![Vec::with_capacity(rows.len()); n];
    for r in &rows {
        let (o, len) = idx::extension(std::slice::from_ref(r), 0, larg, f.dat_len)?;
        let v = mrm::values(
            &datum[o as usize..(o + len) as usize],
            larg,
            r.count as usize,
        )?;
        for (j, c) in col.iter_mut().enumerate() {
            c.push(v.get(j).copied().unwrap_or(0.0));
        }
    }
    let moment: Vec<f64> = rows.iter().map(|r| f64::from(r.rt)).collect();
    let can = f.f.channels();
    Ok(Some(
        col.into_iter()
            .enumerate()
            .map(|(j, intensity)| {
                let q = can.get(j).map(|c| (c.0, c.1));
                let ce = f.ee.as_ref().and_then(|t| t.value("Collision Energy", j));
                let name = f.names.get(j).map_or("", String::as_str);
                Chromatogram {
                    index: first_one + j,
                    id: channel_id(f, j, q, ce, name),
                    kind: if q.is_some_and(|(a, b)| b > 0.0 && b != a) {
                        ChromKind::Srm
                    } else {
                        ChromKind::Sic
                    },
                    q1: q.map(|c| f64::from(c.0)),
                    q3: q
                        .filter(|c| c.1 > 0.0 && c.1 != c.0)
                        .map(|c| f64::from(c.1)),
                    unit: UNITS.into(),
                    time: moment.clone(),
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
        unit: UNITS.into(),
        time: v.iter().map(|x| f64::from(x.0)).collect(),
        intensity: v.iter().map(|x| f64::from(x.1)).collect(),
    })
}

pub(super) fn all_of_them(l: &mut WatersReader) -> Result<Vec<Chromatogram>> {
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
        match channels(l, i, out.len() + 1)? {
            Some(v) => out.extend(v),
            None => {
                let a = format!("{}: function {} channels", l.c.name, l.funs[i].f.indice);
                l.warnings.push(("meta.nonDecodificato".into(), a));
            }
        }
    }
    for a in l.analogiche.clone() {
        let n = out.len();
        out.extend(trace(&mut l.c, &a, n)?);
    }
    for (i, h) in out.iter_mut().enumerate() {
        h.index = i;
    }
    Ok(out)
}
