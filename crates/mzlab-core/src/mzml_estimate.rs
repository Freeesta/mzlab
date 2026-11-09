//! The memory a step asks for when it decodes a whole mzML (`mzml::read`), from a pass over its XML tags before anything is decoded: spectra and points
//! (`defaultArrayLength`), 64-bit intensities and the arrays beyond m/z and intensity. Steps that read whole files (`spectra.ms2`, `formula.ms1`) refuse
//! with `error.memoryBudget` (phase `read`) when it does not fit, on the page, in the app and in `flow run`. Coefficients: `docs/agenti/memoria.md`.

use crate::budget::refusal;
use crate::error::Result;

/// Bytes of a decode whatever the file (parser, buffers of one spectrum, the text of the step).
pub const FLOOR_BYTES: f64 = 16.0 * 1048576.0;
/// Bytes per spectrum or chromatogram (the struct, its id and filter, the per-scan extras of the step).
pub const PER_SPECTRUM: f64 = 640.0;
/// Bytes per decoded point (m/z `f64` and intensity `f32`, with the slack of the arrays).
pub const PER_POINT: f64 = 14.0;
/// Bytes per point of each further `f64` array: 64-bit intensities, ion mobility, the `f64` copy of the intensities of a DIA pseudo-spectrum.
pub const PER_WIDE: f64 = 9.0;
/// Bytes per MS2 point of the MGF text of `spectra.ms2` (`{mz:.5} {i:.1}` and a newline, with the growth of the text).
pub const PER_MGF_POINT: f64 = 24.0;
/// Bytes per MS2 spectrum of the MGF text (`BEGIN IONS`, title, precursor, RT, charge, id, `END IONS`).
pub const PER_MGF_SPECTRUM: f64 = 256.0;

/// What the estimate reads from the tags of one mzML.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Inputs {
    /// Spectra and chromatograms.
    pub spectra: u64,
    /// Their points (`defaultArrayLength`).
    pub points: u64,
    /// Points of further `f64` arrays: the 64-bit intensities and each array beyond the first two.
    pub wide: u64,
    /// MS2 spectra (`MS:1000511` with value 2) and their points.
    pub ms2: u64,
    pub ms2_points: u64,
}

/// One spectrum or chromatogram being read: its points, its `binaryDataArray`s, its 64-bit intensity arrays, whether it takes cvParams from a group,
/// and the array being read (intensity, 64-bit).
#[derive(Default)]
struct Seg {
    len: u64,
    arrays: u64,
    wide: u64,
    by_ref: bool,
    array: (bool, bool),
    ms2: bool,
}

const F64: &[u8] = b"MS:1000523";
const INTENSITY: &[u8] = b"MS:1000515";
const LEVEL: &[u8] = b"MS:1000511";

fn has(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// The value of `name="..."` in the tag `tag`, as a number.
fn attr_u64(tag: &[u8], name: &[u8]) -> u64 {
    let Some(at) = tag.windows(name.len()).position(|w| w == name) else {
        return 0;
    };
    let rest = &tag[at + name.len()..];
    rest.iter()
        .take_while(|b| b.is_ascii_digit())
        .fold(0u64, |n, b| {
            n.saturating_mul(10).saturating_add(u64::from(b - b'0'))
        })
}

/// The inputs of the mzML `b`. Base64 holds no `<`, so the pass visits the tags only; a group that declares 64-bit arrays makes every spectrum that
/// refers to a group count as 64-bit (an over-estimate, never an under-estimate).
pub fn inputs(b: &[u8]) -> Inputs {
    let (mut x, mut seg, mut group64, mut in_group) =
        (Inputs::default(), None::<Seg>, false, false);
    let close = |x: &mut Inputs, s: Seg, group64: bool| {
        let wide = (s.wide + s.arrays.saturating_sub(2)).max(u64::from(s.by_ref && group64));
        x.spectra += 1;
        x.points = x.points.saturating_add(s.len);
        x.wide = x.wide.saturating_add(s.len.saturating_mul(wide));
        if s.ms2 {
            x.ms2 += 1;
            x.ms2_points = x.ms2_points.saturating_add(s.len);
        }
    };
    let mut i = 0;
    while let Some(off) = b[i..].iter().position(|&c| c == b'<') {
        let start = i + off + 1;
        let end = b[start..]
            .iter()
            .position(|&c| c == b'>')
            .map_or(b.len(), |e| start + e);
        let tag = &b[start..end];
        i = end.min(b.len());
        let named = |n: &[u8]| {
            tag.starts_with(n) && tag.get(n.len()).is_none_or(|c| !c.is_ascii_alphanumeric())
        };
        if named(b"spectrum") || named(b"chromatogram") {
            if let Some(s) = seg.take() {
                close(&mut x, s, group64);
            }
            seg = Some(Seg {
                len: attr_u64(tag, b"defaultArrayLength=\""),
                ..Seg::default()
            });
        } else if named(b"/spectrum") || named(b"/chromatogram") {
            if let Some(s) = seg.take() {
                close(&mut x, s, group64);
            }
        } else if named(b"referenceableParamGroup") {
            in_group = !tag.ends_with(b"/");
        } else if named(b"/referenceableParamGroup") {
            in_group = false;
        } else if named(b"cvParam") {
            match seg.as_mut() {
                Some(s) => {
                    s.array.0 |= has(tag, INTENSITY);
                    s.array.1 |= has(tag, F64);
                    s.ms2 |= has(tag, LEVEL) && has(tag, b"value=\"2\"");
                }
                None => group64 |= in_group && has(tag, F64),
            }
        } else if let Some(s) = seg.as_mut() {
            if named(b"binaryDataArray") {
                s.arrays += 1;
                s.array = (false, false);
            } else if named(b"/binaryDataArray") {
                s.wide += u64::from(s.array.0 && s.array.1);
            } else if named(b"referenceableParamGroupRef") {
                s.by_ref = true;
            }
        }
    }
    if let Some(s) = seg {
        close(&mut x, s, group64);
    }
    x
}

/// Bytes of decoding a file with `x` while `held` bytes of files are in memory (the file itself among them).
pub fn bytes(x: &Inputs, held: usize) -> usize {
    (FLOOR_BYTES
        + PER_SPECTRUM * x.spectra as f64
        + PER_POINT * x.points as f64
        + PER_WIDE * x.wide as f64) as usize
        + held
}

/// Bytes of the MGF text of the MS2 spectra of `x` (an upper bound: the floor of the step drops peaks).
pub fn mgf_bytes(x: &Inputs) -> usize {
    (PER_MGF_POINT * x.ms2_points as f64 + PER_MGF_SPECTRUM * x.ms2 as f64) as usize
}

/// Bytes of the `f64` copy of every intensity (the DIA pseudo-spectra).
pub fn f64_copy_bytes(x: &Inputs) -> usize {
    (PER_WIDE * x.points as f64) as usize
}

/// `Ok` when `need` bytes fit in `cap`, else `error.memoryBudget` with the phase `read` and its numbers.
pub fn check(need: usize, cap: usize) -> Result<()> {
    if need <= cap {
        return Ok(());
    }
    Err(refusal("read", need, cap))
}

#[cfg(test)]
#[path = "mzml_estimate_tests.rs"]
mod tests;
