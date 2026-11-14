//! The CSV of `chrom.uv_peaks@1`: the peaks of the UV/PDA trace of each mzML at one wavelength (`uv_bands::uv_peaks`). Bytes in, text out: `flow run`
//! (`mzlab-cli`) and the browser module `flowtab` (`mzlab-feat-wasm`) both call it, so they write the same bytes.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;
use mzlab_core::mzml_estimate;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::wiff::dad::DadData;

use crate::uv_bands::{uv_peaks, UvKind, UvParams};

/// Header of the CSV, the columns of `UvPeak`.
const HEADER: &str = "file,kind,rt_apex_min,rt_lo_min,rt_hi_min,fwhm_s,height_mau,area_mau_s,snr";

fn param(params: &[(String, Value)], name: &str) -> Option<f64> {
    params
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| v.f64())
}

fn flag(params: &[(String, Value)], name: &str) -> bool {
    params
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| v.bool())
        .unwrap_or(false)
}

/// The trace of `d` at `nm`: the mean of the columns within `band / 2`, or the nearest column when there is none.
fn trace_at(d: &DadData, nm: f64, band: f64) -> Vec<f64> {
    let wl = &d.wavelengths_nm;
    let mut cols: Vec<usize> = (0..wl.len())
        .filter(|&c| (wl[c] - nm).abs() <= band / 2.0)
        .collect();
    if cols.is_empty() {
        let near = (0..wl.len()).min_by(|&a, &b| (wl[a] - nm).abs().total_cmp(&(wl[b] - nm).abs()));
        cols.extend(near);
    }
    (0..d.time_min.len())
        .map(|r| {
            let sum: f64 = cols
                .iter()
                .map(|&c| f64::from(d.matrix_mau[r * wl.len() + c]))
                .sum();
            sum / cols.len() as f64
        })
        .collect()
}

/// A CSV field: quoted when it holds a comma, a quote or a line break.
fn field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The CSV of the peaks of every file (`name`, bytes) under the parameters of the step. Each file is read whole only when its decoding (`mzml_estimate`,
/// with every file and the copy the reader takes in memory) fits in `cap` bytes, else `error.memoryBudget` (phase `read`).
pub fn uv_text(params: &[(String, Value)], files: &[(&str, &[u8])], cap: usize) -> Result<String> {
    let (nm, band) = (
        param(params, "wavelength_nm").unwrap_or(254.0),
        param(params, "band_nm").unwrap_or(4.0),
    );
    let p = UvParams {
        broad: flag(params, "broad"),
        min_s: param(params, "broad_min_s").unwrap_or(30.0),
        max_s: param(params, "broad_max_s").unwrap_or(300.0),
        snr: param(params, "broad_snr").unwrap_or(8.0),
    };
    let mut out = format!("{HEADER}\n");
    let held: usize = files.iter().map(|(_, b)| b.len()).sum();
    for (name, bytes) in files {
        let x = mzml_estimate::inputs(bytes);
        mzml_estimate::check(mzml_estimate::bytes(&x, held + bytes.len()), cap)?;
        let mut m = Mzml::new(bytes.to_vec(), bytes.len() as u64);
        let d = m.dad()?.ok_or_else(|| Error::new("err.pda.none"))?;
        let y = trace_at(&d, nm, band);
        for k in uv_peaks(&d.time_min, &y, &p)? {
            let kind = if k.kind == UvKind::Narrow {
                "narrow"
            } else {
                "broad"
            };
            out += &format!(
                "{},{kind},{:.4},{:.4},{:.4},{:.2},{:.4},{:.3},{:.1}\n",
                field(name),
                k.rt_apex,
                k.rt_lo,
                k.rt_hi,
                k.fwhm_s,
                k.height,
                k.area,
                k.snr
            );
        }
    }
    Ok(out)
}

/// `uv_text` for the browser (`flowtab`): `params_json` is the step's parameters merged with its defaults, the files are packed as `manifest`
/// (JSON `[{"name","len"}]`, in the order of the plan) over their concatenated bytes `blob`.
pub fn uv_packed(params_json: &str, manifest: &str, blob: &[u8], cap: usize) -> Result<String> {
    let params = crate::flow_ms2::json_params(params_json)?;
    let bad = || Error::new("err.uv.manifest");
    let list = mzlab_core::json::parse(manifest.as_bytes())?;
    let (mut at, mut files) = (0usize, Vec::new());
    for f in list.arr() {
        let len = f.get("len").and_then(Value::f64).ok_or_else(bad)? as usize;
        let name = f.get("name").and_then(Value::str).ok_or_else(bad)?;
        files.push((name, blob.get(at..at + len).ok_or_else(bad)?));
        at += len;
    }
    uv_text(&params, &files, cap)
}
