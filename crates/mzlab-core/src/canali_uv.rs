//! UV (PDA/DAD) and analog channels (`uv:twc`, `uv:<nm>±<bw>`, `an:<i>`) of a file: listed next to `tic` as roots, read through the routes that already
//! serve them (`pda_trace`, `pda_wavelength`, `pda_spectrum`, `analog`), so `canale=` answers byte for byte what `kind=pda` and `api/analog` answer.

use crate::canali::{Channel, Famiglia, Source};
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::iteminfo::Facts;
use crate::itemroutes::{analog as analog_route, pda_spectrum, pda_trace, pda_wavelength};
use crate::json::fmt_g;
use crate::mzmlhead::{Chrom, Mzml};

/// What a UV or analog channel id asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reading {
    /// `uv:twc`: the total wavelength chromatogram of the file.
    Twc,
    /// `uv:<nm>±<bw>` (or `+-`): the trace between `nm - bw` and `nm + bw`; a bare `uv:<nm>` has bandwidth 0.
    Nm(f64, f64),
    /// `an:<i>`: the chromatogram of that index (pressure, flow, single UV).
    Analog(i64),
}

/// The reading an id asks for; `None` when the id is not a UV or analog one.
pub fn read_id(id: &str) -> Option<Reading> {
    if let Some(i) = id.strip_prefix("an:") {
        return i.parse().ok().map(Reading::Analog);
    }
    let r = id.strip_prefix("uv:")?;
    if r == "twc" {
        return Some(Reading::Twc);
    }
    let (nm, bw) = match r.split_once('±').or_else(|| r.split_once("+-")) {
        Some((a, b)) => (a, b),
        None => (r, "0"),
    };
    let (nm, bw): (f64, f64) = (nm.parse().ok()?, bw.parse().ok()?);
    (nm.is_finite() && bw.is_finite() && nm > 0.0 && bw >= 0.0).then_some(Reading::Nm(nm, bw))
}

/// `uv:twc` and one `an:<i>` per analog trace of the file (a file without a PDA has no `uv:`); roots, the derived `uv:<nm>±<bw>` is never listed.
pub fn add(v: &mut Vec<Channel>, chroms: &[Chrom], facts: &Facts) {
    if let (Some(c), Some((t0, t1, n))) = (chroms.iter().find(|c| c.kind == "pda"), facts.pda_rt) {
        v.push(Channel {
            id: "uv:twc".into(),
            famiglia: Famiglia::Uv,
            parent: None,
            livello: 0,
            pol: 0,
            sigla: Some("TWC"),
            label: "UV TWC".into(),
            omologo: "uv|twc".into(),
            n: n.max(0) as u32,
            rt: (t0, t1),
            spettri: true,
            source: Source::Cromatogramma(c.index as i64),
        });
    }
    for a in &facts.analog {
        let unit = if a.unit.is_empty() {
            String::new()
        } else {
            format!(" ({})", a.unit)
        };
        v.push(Channel {
            id: format!("an:{}", a.i),
            famiglia: Famiglia::Analog,
            parent: None,
            livello: 0,
            pol: 0,
            sigla: None,
            label: format!("{}{unit}", a.id),
            omologo: format!("an|{}|{}", a.kind, fmt_g(a.t1 - a.t0)),
            n: a.n.max(0) as u32,
            rt: (a.t0, a.t1),
            spettri: false,
            source: Source::Cromatogramma(a.i),
        });
    }
}

/// The trace of the channel `id` (`{rt, y}`; an analog one as `api/analog`, `n` points at most per extreme).
pub fn trace<M: ReadAt>(m: &mut Mzml<M>, chroms: &[Chrom], id: &str, n: i64) -> Result<String> {
    let unknown = || Error::new("err.canali.unknown").with("id", id);
    match read_id(id).ok_or_else(unknown)? {
        Reading::Twc => pda_trace(m, chroms),
        Reading::Nm(nm, bw) => pda_wavelength(m, nm - bw, nm + bw),
        Reading::Analog(i) => analog_route(m, chroms, i, n),
    }
}

/// The UV spectrum between `rt0` and `rt1` of `uv:twc` (the spectrum of one scan: `rt0 == rt1`).
pub fn spettro<M: ReadAt>(m: &mut Mzml<M>, id: &str, rt0: f64, rt1: f64) -> Result<String> {
    match read_id(id) {
        Some(Reading::Twc) => pda_spectrum(m, rt0, rt1),
        _ => Err(Error::new("err.canali.unknown").with("id", id)),
    }
}

#[cfg(test)]
mod tests;
