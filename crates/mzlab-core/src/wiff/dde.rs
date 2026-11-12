//! The precursors of the dependent (IDA) product-ion scans of a `.wiff` sample. `SampleSubtree/SampleN/DDERealTimeData` holds a 32-byte head and one
//! 32-byte row per dependent slot of each cycle (f64 precursor m/z at +0, u16 charge at +24): a layout worked out from files, not yet checked against the
//! vendor software. It is read defensively: a stream that does not fit the scans, or a row that is not a plausible m/z, leaves the precursor absent and
//! counted (`warn.wiff.idaPrecursor`); the header mass of the method is never written in its place.

use super::modes::CODE_PRODUCT;
use super::{header, method_no, samples, Experiment, Record, SAMPLES};
use crate::methodinfo::{Kind, Method};
use crate::ole::Ole;

const STREAM: &str = "DDERealTimeData";
const HEAD: usize = 32;
const ROW: usize = 32;
/// The widest m/z an SCIEX instrument scans (TOF): the bound when the method gives no survey range.
const MZ_MAX: f64 = 40_000.0;
/// A charge above this is not a charge: the row is not what the layout says.
const Z_MAX: u16 = 99;

/// How a sample's product-ion experiments are acquired: by a fixed method (`Fixed`), or by IDA with the sample's own stream (`Stream`) or without it
/// (`Missing`: another sample of the same method has one, so the header mass is the saved template, not the measured precursor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ida {
    Fixed,
    Stream,
    Missing,
}

fn stream_of(sample: u32) -> String {
    format!("{SAMPLES}Sample{sample}/{STREAM}")
}

/// Whether methods `a` and `b` have the same experiment headers, byte for byte (and at least one).
fn same_headers(ole: &Ole, a: u32, b: u32) -> bool {
    let mut i = 0;
    loop {
        match (header(ole, a, i), header(ole, b, i)) {
            (None, None) => return i > 0,
            (x, y) if x != y => return false,
            _ => i += 1,
        }
    }
}

/// Whether `sample` was acquired by IDA; its dependent experiments then lose the header mass (`clear`).
#[inline(never)] // kept out of `open_wiff`: inlined there it costs the size-capped `.wasm` about 1 KB more
pub(super) fn ida_of(ole: &Ole, sample: u32, experiments: &mut [Experiment]) -> Ida {
    let ida = acquisition(ole, sample);
    clear(ida, experiments);
    ida
}

/// IDA with the sample's own stream, else when another sample that has one ran the same method (same `MethodN`, or the same experiment headers byte
/// for byte), else a fixed method.
fn acquisition(ole: &Ole, sample: u32) -> Ida {
    if ole.has(&stream_of(sample)) {
        return Ida::Stream;
    }
    let own = method_no(ole, sample);
    let same = |k: u32| {
        let no = method_no(ole, k);
        no == own || same_headers(ole, own, no)
    };
    let mut others = samples(ole)
        .into_iter()
        .filter(|k| *k != sample && ole.has(&stream_of(*k)));
    if others.any(same) {
        Ida::Missing
    } else {
        Ida::Fixed
    }
}

/// The dependent experiments (product ions of an IDA sample) lose the header mass: it is the value the method was saved with.
fn clear(ida: Ida, experiments: &mut [Experiment]) {
    if ida != Ida::Fixed {
        experiments
            .iter_mut()
            .filter(|e| e.code == Some(CODE_PRODUCT))
            .for_each(|e| e.precursor = None);
    }
}

/// The rows of the stream `b` (its body, `ROW` bytes each) for `n_exp` experiments and `records`, `None` when its length or its row count does not fit:
/// one row per slot of every cycle after the survey (`cycle * (n_exp - 1) + experiment - 1`), for the whole cycles or up to the last slot the records reach.
fn rows<'a>(b: &'a [u8], n_exp: usize, records: &[Record]) -> Option<&'a [u8]> {
    let body = b.get(HEAD..)?;
    if n_exp < 2 || body.is_empty() || body.len() % ROW != 0 {
        return None;
    }
    let n = body.len() / ROW;
    let cycles = records.len().div_ceil(n_exp);
    let reached = records
        .iter()
        .filter(|r| r.experiment >= 1)
        .map(|r| slot(r, n_exp) + 1)
        .max()
        .unwrap_or(0);
    (n == cycles * (n_exp - 1) || n == reached).then_some(body)
}

/// Row `k` of a body `rows` accepted.
fn row(body: &[u8], k: usize) -> Option<&[u8]> {
    body.get(k * ROW..(k + 1) * ROW)
}

fn slot(r: &Record, n_exp: usize) -> usize {
    r.cycle * (n_exp - 1) + r.experiment - 1
}

/// The precursor m/z and charge of a row, `None` when it is not plausible: not finite, not above 0 (an untriggered slot), outside the survey range
/// (or above `MZ_MAX` without one), or a charge no ion has.
fn plausible(row: &[u8], range: Option<[f64; 2]>) -> Option<(f64, Option<u8>)> {
    let mz = f64::from_le_bytes(row.get(..8)?.try_into().ok()?);
    let z = u16::from_le_bytes([*row.get(24)?, *row.get(25)?]);
    let (lo, hi) = range.map_or((0.0, MZ_MAX), |[a, b]| (a.min(b) - 1.0, a.max(b) + 1.0));
    // NaN fails every comparison and an infinity the finite bounds
    let inside = mz > 0.0 && mz >= lo && mz <= hi;
    (inside && z <= Z_MAX).then(|| (mz, u8::try_from(z).ok().filter(|z| *z > 0)))
}

/// Gives each acquired dependent scan of `records` the precursor of its row; returns how many acquired dependent scans have none (a row that does
/// not fit, a stream that does not fit the scans, or no stream: `Ida::Missing`). A row must lie in the m/z range of the survey of `method`, when it says.
#[inline(never)] // as `ida_of`
pub(super) fn assign(
    ole: &Ole,
    sample: u32,
    ida: Ida,
    experiments: &[Experiment],
    records: &mut [Record],
    method: &Option<Method>,
) -> usize {
    if ida == Ida::Fixed {
        return 0;
    }
    let range = survey_range(method);
    let n_exp = experiments.len();
    let dependent = |r: &Record| {
        r.size > 0
            && experiments
                .get(r.experiment)
                .is_some_and(|e| e.code == Some(CODE_PRODUCT))
    };
    let b = (ida == Ida::Stream)
        .then(|| ole.read(&stream_of(sample)))
        .flatten()
        .unwrap_or_default();
    let table = rows(&b, n_exp, records);
    let mut unread = 0;
    for r in records.iter_mut().filter(|r| dependent(r)) {
        let found = table
            .as_ref()
            .filter(|_| r.experiment >= 1)
            .and_then(|t| row(t, slot(r, n_exp)))
            .and_then(|row| plausible(row, range));
        match found {
            Some((mz, z)) => (r.precursor, r.charge) = (Some(mz), z),
            None => unread += 1,
        }
    }
    unread
}

/// The m/z range the method gives the survey experiment (the first): an IDA precursor is picked from it.
fn survey_range(method: &Option<Method>) -> Option<[f64; 2]> {
    match method
        .as_ref()?
        .experiments
        .iter()
        .find(|x| x.index == 0)?
        .kind
    {
        Kind::Scan { range, .. } => range,
        Kind::Mrm(_) => None,
    }
}

#[cfg(test)]
#[path = "dde_tests.rs"]
mod tests;
