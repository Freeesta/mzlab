//! The arrays of one spectrum decoded (`decode_peaks`) and put together with its header (`assemble`): `finish_spectrum` does both on the parser,
//! the browser's helper workers (`help::corriere`) do the first in another instance and send the arrays back.

use super::{Array, ArrayKind, Mobility, Spec, Spectrum};
use crate::error::Result;
use crate::mzmlarray::{decode_array, Inflate, Want};
use crate::numarray::Num;

/// The decoded arrays of a spectrum before they become a `Spectrum` (`assemble`): also what a decoder in another instance sends back (`help::corriere`).
#[derive(Default)]
pub(crate) struct Peaks {
    pub(crate) mz: Vec<f64>,
    pub(crate) it: Vec<f32>,
    pub(crate) wide: Option<Vec<f64>>,
    pub(crate) mob: Option<Vec<f64>>,
}

/// What `decode_peaks` needs of one array.
#[derive(Clone, Copy)]
pub(crate) struct Arr<'a> {
    pub(crate) kind: ArrayKind,
    pub(crate) num: Num,
    pub(crate) zlib: bool,
    pub(crate) text: &'a str,
}

impl Array {
    pub(crate) fn arr(&self) -> Arr<'_> {
        Arr {
            kind: self.kind,
            num: self.num,
            zlib: self.zlib,
            text: &self.text,
        }
    }
}

// one copy in the binary: the parser and the helpers of the browser share it
#[inline(never)]
pub(crate) fn decode_peaks<'a>(
    arrays: impl Iterator<Item = Arr<'a>>,
    len: usize,
    inf: &mut Inflate,
) -> Result<Peaks> {
    let mut p = Peaks::default();
    for a in arrays {
        match a.kind {
            ArrayKind::Mobility => {
                p.mob = Some(decode_array(a.num, a.zlib, a.text, len, inf, Want::F64)?.0)
            }
            ArrayKind::Mz => p.mz = decode_array(a.num, a.zlib, a.text, len, inf, Want::F64)?.0,
            ArrayKind::Intensity => {
                if a.num == Num::F32 {
                    p.it = decode_array(a.num, a.zlib, a.text, len, inf, Want::F32)?.1;
                    p.wide = None;
                } else {
                    let (w, f) = decode_array(a.num, a.zlib, a.text, len, inf, Want::Both)?;
                    p.it = f;
                    p.wide = Some(w);
                }
            }
            _ => {}
        }
    }
    Ok(p)
}

/// The spectrum of the header `s` with its decoded arrays `p`.
#[inline(never)]
pub(crate) fn assemble(s: Spec, p: Peaks) -> (Spectrum, Option<Mobility>) {
    let Peaks {
        mut mz,
        mut it,
        mut wide,
        mob,
    } = p;
    let mob_unit = s
        .arrays
        .iter()
        .rev()
        .find(|a| a.kind == ArrayKind::Mobility)
        .map_or("", |a| a.mobility);
    let n = mz.len().min(it.len());
    mz.truncate(n);
    it.truncate(n);
    if let Some(w) = wide.as_mut() {
        w.truncate(n);
    }
    // an array of another length does not belong to these peaks
    let mob = mob.filter(|m| m.len() == n && n > 0);
    let mobility = (mob.is_some() || s.mobility_scan.is_some()).then_some(Mobility {
        index: s.index,
        values: mob,
        unit: mob_unit,
        scan: s.mobility_scan,
    });
    (
        Spectrum {
            index: s.index,
            id: s.id,
            level: s.level,
            rt: if s.rt_seconds { s.rt / 60.0 } else { s.rt },
            polarity: s.polarity,
            profile: s.profile,
            filter: s.filter,
            precursor: s.selected.or(s.target),
            mz,
            intensity: it,
            wide,
        },
        mobility,
    )
}
