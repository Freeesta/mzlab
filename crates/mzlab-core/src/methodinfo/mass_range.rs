//! `MassRangeEx/MassRangeEx` of an Analyst experiment read by its counted grammar (`docs/agenti/wiff_specifica_completa.md` §2.2): a 40-byte head
//! (`+36` u32 = parameters per entry), then exactly `n` entries (n = `ExperimentHeader` +176) of Q1/start, 0/stop, Q3/step, dwell or RT, a reserved u32,
//! a u16 byte length and the UTF-16 name, then the parameter blocks (u16 tag length, tag, start, stop, step). Any other length is an error, never a guess.

use super::{round, Kind, Transition};
use crate::ole::Ole;

/// Offset of the number of entries in `ExperimentHeader`.
pub(crate) const HEADER_COUNT: usize = 176;
const HEAD: usize = 40;
/// More parameters per entry than any method seen (9) by a wide margin: a larger value is not this grammar.
const MAX_PARAMS: u32 = 64;

/// One parameter block of an entry: its tag (`DP`, `CE`, ...) and start value (the stop and step only differ in a ramp).
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub id: String,
    pub start: f32,
    pub stop: f32,
}

/// One entry: an MRM transition (`m0` Q1, `m2` Q3) or a scan range (`m0` start, `m1` stop, `m2` step).
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub m0: f32,
    pub m1: f32,
    pub m2: f32,
    pub dwell: f32,
    pub name: String,
    pub params: Vec<Param>,
}

fn u16_at(d: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn f32_at(d: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn utf16(b: &[u8]) -> String {
    let u: Vec<u16> = b
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&u)
}

/// The number of entries the experiment header announces (`None` when the header is too short to say).
pub fn header_count(header: &[u8]) -> Option<u32> {
    u32_at(header, HEADER_COUNT)
}

/// The entries `ExperimentHeader` +176 announces for experiment `i` (`None` when the header is missing or says 0: no count to read by).
pub(super) fn announced(ole: &Ole, base: &str, i: usize) -> Option<u32> {
    ole.read(&format!(
        "{base}DeviceMethod0/Period0/Experiment{i}/ExperimentHeader"
    ))
    .and_then(|h| header_count(&h))
    .filter(|n| *n > 0)
}

/// A mass the method stores as an f32, at the precision the file holds: the shortest decimal that reads back as the same f32 (364.125 stays
/// 364.125, 364.1 does not become 364.100006).
#[allow(clippy::cast_possible_truncation)]
pub(super) fn mass(x: f64) -> f64 {
    let f = x as f32;
    if f.is_finite() {
        f.to_string().parse().unwrap_or(x)
    } else {
        x
    }
}

/// The experiment read by the counted grammar; a stream that does not hold the announced entries keeps no table (the `.wiff` reader refuses the sample).
pub(super) fn counted(d: &[u8], n: u32, is_mrm: bool) -> Kind {
    let Ok(entries) = parse(d, n) else {
        return if is_mrm {
            Kind::Mrm(Vec::new())
        } else {
            Kind::Scan {
                range: None,
                ce: Vec::new(),
            }
        };
    };
    let value = |e: &Entry, id: &str| {
        e.params
            .iter()
            .find(|p| p.id == id)
            .map(|p| f64::from(p.start))
            .filter(|v| v.is_finite())
    };
    if is_mrm {
        let p = |e: &Entry, id: &str| value(e, id).map(|v| round(v, 2));
        return Kind::Mrm(
            entries
                .iter()
                .map(|e| Transition {
                    q1: mass(f64::from(e.m0)),
                    q3: mass(f64::from(e.m2)),
                    dwell: round(f64::from(e.dwell), 1),
                    name: e.name.clone(),
                    ce: p(e, "CE"),
                    dp: p(e, "DP"),
                    ep: p(e, "EP"),
                    cep: p(e, "CEP"),
                    cxp: p(e, "CXP"),
                })
                .collect(),
        );
    }
    let mut ce: Vec<f64> = Vec::new();
    for v in entries
        .iter()
        .filter_map(|e| value(e, "CE"))
        .map(|v| round(v, 2))
    {
        if !ce.contains(&v) {
            ce.push(v);
        }
    }
    crate::sortc::by(&mut ce, |a, b| a.total_cmp(b));
    // the scan covers the first range's start to the last range's stop
    let (a, b) = match (entries.first(), entries.last()) {
        (Some(f), Some(l)) => (f64::from(f.m0), f64::from(l.m1)),
        _ => (f64::NAN, f64::NAN),
    };
    let range = (10.0 <= a && a < b && b <= 3000.0).then(|| [round(a, 2), round(b, 2)]);
    Kind::Scan { range, ce }
}

/// The `n` entries of `d`, or why the stream does not hold exactly `n` entries of the grammar (the text goes in the `detail` of the error).
pub fn parse(d: &[u8], n: u32) -> Result<Vec<Entry>, String> {
    let n_params =
        u32_at(d, 36).ok_or_else(|| format!("stream of {} bytes has no head", d.len()))?;
    if n_params > MAX_PARAMS {
        return Err(format!("{n_params} parameters per entry"));
    }
    let mut at = HEAD;
    let mut out = Vec::with_capacity(n.min(4096) as usize);
    for k in 0..n {
        let cut = || {
            format!(
                "entry {} of {n} runs past the end ({} bytes)",
                k + 1,
                d.len()
            )
        };
        let len = u16_at(d, at + 20).ok_or_else(cut)?;
        if len % 2 != 0 {
            return Err(format!("entry {} has a name of odd length {len}", k + 1));
        }
        let name = utf16(d.get(at + 22..at + 22 + len).ok_or_else(cut)?);
        let mut e = Entry {
            m0: f32_at(d, at).ok_or_else(cut)?,
            m1: f32_at(d, at + 4).ok_or_else(cut)?,
            m2: f32_at(d, at + 8).ok_or_else(cut)?,
            dwell: f32_at(d, at + 12).ok_or_else(cut)?,
            name,
            params: Vec::with_capacity(n_params as usize),
        };
        at += 22 + len;
        for _ in 0..n_params {
            let plen = u16_at(d, at).ok_or_else(cut)?;
            if plen == 0 || plen % 2 != 0 {
                return Err(format!(
                    "entry {} has a parameter tag of length {plen}",
                    k + 1
                ));
            }
            let id = utf16(d.get(at + 2..at + 2 + plen).ok_or_else(cut)?);
            let v = at + 2 + plen;
            e.params.push(Param {
                id,
                start: f32_at(d, v).ok_or_else(cut)?,
                stop: f32_at(d, v + 4).ok_or_else(cut)?,
            });
            f32_at(d, v + 8).ok_or_else(cut)?;
            at = v + 12;
        }
        out.push(e);
    }
    if at != d.len() {
        return Err(format!(
            "{n} entries end at byte {at}, the stream has {}",
            d.len()
        ));
    }
    Ok(out)
}

#[cfg(test)]
#[path = "mass_range_tests.rs"]
mod tests;
