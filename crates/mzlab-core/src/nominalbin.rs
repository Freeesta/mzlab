//! `rebin=nominal|0.1` of `/api/spectrum` on unit-resolution data: the points of a window summed into half-open bins around integers (or tenths)
//! AFTER the m/z offset is taken off, so a fragment that the profile splits between two integers (106.7 and 107.0) lands in one bin. Site, app and
//! CLI answer through here; `mzoff=DA|auto:MZ[@RT]` is the offset of `mzcal`, the answer states the rule, the bins and the offset.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::json::Out;
use crate::mzcal::{Offset, Spec};
use crate::routes::{Query, Routes};
use crate::spectrum::Binned;

/// What the bins are around: whole numbers or multiples of 0.1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rule {
    Nominal,
    Tenth,
}

impl Rule {
    /// `nominal` or `0.1`; anything else is `None`.
    pub fn parse(s: &str) -> Option<Rule> {
        match s {
            "nominal" => Some(Rule::Nominal),
            "0.1" => Some(Rule::Tenth),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Rule::Nominal => "nominal",
            Rule::Tenth => "0.1",
        }
    }
    /// Width of a bin, Da.
    pub fn width(self) -> f64 {
        match self {
            Rule::Nominal => 1.0,
            Rule::Tenth => 0.1,
        }
    }
}

/// One bin: the integer (or tenth) it is around, the centre of mass of its points on the corrected scale, the intensity summed and the points.
#[derive(Debug, Clone, PartialEq)]
pub struct Bin {
    pub centre: f64,
    pub mz: f64,
    pub y: f64,
    pub points: usize,
}

/// The index of the half-open bin `[k - w/2, k + w/2)` that holds the corrected m/z `m`.
fn cell(m: f64, w: f64) -> i64 {
    (m / w + 0.5).floor() as i64
}

/// The points (m/z ascending, as the route builds them) summed per bin after `offset` Da is taken off every m/z (`offset = observed - true`). The
/// centre of mass is intensity-weighted on the corrected scale; a bin whose intensities do not add up to more than zero takes the plain mean of its
/// m/z, never m/z 0.
pub fn rebin(mz: &[f64], y: &[f64], rule: Rule, offset: f64) -> Vec<Bin> {
    let w = rule.width();
    let mut out: Vec<Bin> = Vec::new();
    let (mut c, mut sy, mut smy, mut sm, mut n) = (i64::MIN, 0.0, 0.0, 0.0, 0usize);
    let mut flush = |c: i64, sy: f64, smy: f64, sm: f64, n: usize| {
        if n > 0 {
            let mz = if sy > 0.0 { smy / sy } else { sm / n as f64 };
            out.push(Bin {
                centre: c as f64 * w,
                mz,
                y: sy,
                points: n,
            });
        }
    };
    for (&m, &v) in mz.iter().zip(y) {
        let m = m - offset;
        let k = cell(m, w);
        if k != c {
            flush(c, sy, smy, sm, n);
            (c, sy, smy, sm, n) = (k, 0.0, 0.0, 0.0, 0);
        }
        sy += v;
        smy += v * m;
        sm += m;
        n += 1;
    }
    flush(c, sy, smy, sm, n);
    out
}

/// What a request asked for: the rule and the offset in force.
pub(crate) struct Rebin {
    pub rule: Rule,
    pub off: Offset,
}

impl<R: ReadAt> Routes<R> {
    /// `rebin` and `mzoff` of a request; `None` without `rebin`. `err.spectrum.rebin` for a value that is neither `nominal` nor `0.1`.
    pub(crate) fn rebin_of(&mut self, q: &Query) -> Result<Option<Rebin>> {
        let Some(t) = q.some("rebin") else {
            return Ok(None);
        };
        let rule = Rule::parse(t)
            .ok_or_else(|| Error::new("err.spectrum.rebin").with("value", t.to_string()))?;
        let off = match q.some("mzoff") {
            Some(s) => Spec::parse(s)?.resolve(&mut self.idx, self.pol)?,
            None => Offset::default(),
        };
        Ok(Some(Rebin { rule, off }))
    }
}

/// The answer of `/api/spectrum` with `rebin`: the bins in columns (`bin`, `mz`, `y`, `points`), then the rule and the offset (always stated, also
/// when 0), then the window as the plain answers give it.
pub(crate) fn bins_json(b: &Binned, rb: &Rebin) -> String {
    let bins = rebin(&b.mz, &b.y, rb.rule, rb.off.da);
    let col = |o: &mut Out, key: &str, f: &dyn Fn(&Bin) -> f64, nd: usize| {
        let v: Vec<f64> = bins.iter().map(f).collect();
        o.raw(key).rounded(&v, nd);
    };
    let mut o = Out::default();
    o.raw("{\"mode\":\"bins\",\"rule\":")
        .str(rb.rule.name())
        .raw(",\"width_da\":")
        .num(rb.rule.width());
    o.raw(",\"edges\":\"half-open [bin - width/2, bin + width/2) on the corrected m/z (observed - offset)\"");
    o.raw(",\"mz_offset_da\":")
        .num(rb.off.da)
        .raw(",\"mz_offset\":")
        .raw(&rb.off.evidence_json())
        .raw(",");
    col(&mut o, "\"bin\":", &|x| x.centre, 1);
    col(&mut o, ",\"mz\":", &|x| x.mz, 3);
    col(&mut o, ",\"y\":", &|x| x.y, 1);
    o.raw(",\"points\":[");
    for (i, x) in bins.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(x.points as i64);
    }
    o.raw("],\"scans\":").int(b.n as i64);
    crate::profspec::window_tail(&mut o, b);
    o.raw("}");
    o.0
}

#[cfg(test)]
#[path = "nominalbin_tests.rs"]
mod tests;
