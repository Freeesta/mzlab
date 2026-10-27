//! The 2D optical (DAD / PDA) matrix of an mzML, read into the `DadData` the `.wiff` reader fills, so `/api/map?kind=pda`, `/api/uvspec` and
//! `/api/uvxic` answer the same JSON for both. Two layouts: a `dad_matrix` array in the PDA `<chromatogram>`, or `<spectrum>` elements with
//! `MS:1000804` (electromagnetic radiation spectrum). Used by `itemroutes`.

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::mzmlhead::{arrays, attr, decode, find, text, Mzml};
use crate::wiff::dad::DadData;

/// Bytes read per step when walking the file for UV spectra.
const STEP: u64 = 4 << 20;

impl<R: ReadAt> Mzml<R> {
    /// The optical matrix of the file, or `None` when it has no more than the 1D PDA trace.
    pub fn dad(&mut self) -> Result<Option<DadData>> {
        if let Some(d) = self.dad_array()? {
            return Ok(Some(d));
        }
        self.dad_spectra()
    }

    /// The `dad_matrix` array of a `<chromatogram>`; the chromatograms close the file, so only its tail is read.
    fn dad_array(&mut self) -> Result<Option<DadData>> {
        let mut n: u64 = 1 << 20;
        loop {
            let from = self.size.saturating_sub(n);
            let tail = self.read(from, self.size)?;
            if let Some(p) = find(&tail, b"<chromatogramList", 0) {
                for (head, data) in arrays(&tail[p..]) {
                    if head.contains("value=\"dad_matrix\"") {
                        return Ok(from_array(&head, &data));
                    }
                }
                return Ok(None);
            }
            if from == 0 {
                return Ok(None);
            }
            n *= 4;
        }
    }

    /// The `MS:1000804` spectra, walking the file in steps (a spectrum that does not fit grows the step).
    fn dad_spectra(&mut self) -> Result<Option<DadData>> {
        let (mut pos, mut step) = (0u64, STEP);
        let (mut times, mut wl, mut mat): (Vec<f64>, Vec<f64>, Vec<f32>) = (vec![], vec![], vec![]);
        while pos < self.size {
            let chunk = self.read(pos, (pos + step).min(self.size))?;
            let last = pos + step >= self.size;
            let Some(a) = find(&chunk, b"<spectrum ", 0) else {
                pos += (chunk.len() as u64).saturating_sub(10).max(1);
                continue;
            };
            let Some(b) = find(&chunk, b"</spectrum>", a) else {
                if last {
                    break;
                }
                pos += a as u64;
                step = if a == 0 { step * 2 } else { step };
                continue;
            };
            let block = &chunk[a..b + 11];
            if find(block, b"MS:1000804", 0).is_some() {
                if let Some((t, w, y)) = uv_row(block) {
                    if wl.is_empty() {
                        wl = w;
                    }
                    // a row on another wavelength grid cannot join the matrix
                    if y.len() == wl.len() {
                        times.push(t);
                        mat.extend(y.iter().map(|&v| v as f32));
                    }
                }
            }
            pos += (b + 11) as u64;
        }
        Ok(build(times, wl, mat))
    }
}

/// `<userParam name="key" value="..."/>` of an array head, as a number.
fn user(head: &str, key: &str) -> Option<f64> {
    let k = format!("<userParam name=\"{key}\" value=\"");
    let s = head.find(&k)? + k.len();
    head[s..].split('"').next()?.trim().parse().ok()
}

/// A matrix of `nt` rows by `nl` columns on a uniform grid, or `None` when the shape does not fit the values.
fn build(times: Vec<f64>, wl: Vec<f64>, mat: Vec<f32>) -> Option<DadData> {
    let (nt, nl) = (times.len(), wl.len());
    if nt == 0 || nl == 0 || mat.len() != nt * nl {
        return None;
    }
    let (w0, w1) = (*wl.first()?, *wl.last()?);
    let step = if nl > 1 {
        (w1 - w0) / (nl - 1) as f64
    } else {
        1.0
    };
    Some(DadData {
        duration_min: *times.last()?,
        time_min: times,
        twc_mau: vec![],
        wavelengths_nm: wl,
        wl_start_nm: w0,
        wl_stop_nm: w1,
        wl_step_nm: step,
        matrix_mau: mat,
    })
}

/// The `dad_matrix` array of one `<chromatogram>` block, with the time and wavelength axes its `userParam`s give (minutes, nanometres).
fn from_array(head: &str, data: &[u8]) -> Option<DadData> {
    let (nt, nl) = (user(head, "n_rt")? as usize, user(head, "n_wl")? as usize);
    let (t0, t1) = (user(head, "rt_start")?, user(head, "rt_stop")?);
    let (w0, step) = (user(head, "wl_start")?, user(head, "wl_step")?);
    let vals = decode(head, data, nt.saturating_mul(nl)).ok()?;
    let at = |n: usize, a: f64, d: f64| (0..n).map(|i| a + d * i as f64).collect::<Vec<_>>();
    let dt = if nt > 1 {
        (t1 - t0) / (nt - 1) as f64
    } else {
        0.0
    };
    #[allow(clippy::cast_possible_truncation)]
    let mat = vals.iter().map(|&v| v as f32).collect();
    build(at(nt, t0, dt), at(nl, w0, step), mat)
}

/// One `<spectrum>` with `MS:1000804`: (time in minutes, wavelengths, absorbance).
fn uv_row(block: &[u8]) -> Option<(f64, Vec<f64>, Vec<f64>)> {
    let list = find(block, b"<binaryDataArrayList", 0)?;
    let h = text(&block[..list]);
    let p = h.find("accession=\"MS:1000016\"")?;
    let tag = &h[h[..p].rfind('<')?..p + h[p..].find('>')?];
    let mut rt: f64 = attr(tag, "value")?.parse().ok()?;
    if attr(tag, "unitName").is_some_and(|u| u.starts_with("second")) {
        rt /= 60.0;
    }
    let (mut wl, mut ab) = (None, None);
    for (head, data) in arrays(block) {
        let a = decode(&head, &data, 0).ok()?;
        if head.contains("MS:1000617") {
            wl = Some(a);
        } else if head.contains("MS:1000515") {
            ab = Some(a);
        }
    }
    let (wl, ab) = (wl?, ab?);
    (wl.len() == ab.len() && !wl.is_empty()).then_some((rt, wl, ab))
}

#[cfg(test)]
#[path = "mzml_dad_tests.rs"]
mod tests;
