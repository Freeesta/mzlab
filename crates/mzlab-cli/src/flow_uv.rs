//! The native executor of `chrom.uv_peaks@1`: the peaks of the UV/PDA trace of mzML files at one wavelength, the narrow ones of `detect_peaks` and,
//! when asked, the broad bands (`mzlab_feat::uv_bands`), as one CSV. Takes names and bytes. Called by `flow.rs`.

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};
use mzlab_core::wiff::dad::DadData;
use mzlab_feat::uv_bands::{uv_peaks, UvKind, UvParams};

use crate::flow::{io, read_all, FileRunner};
use crate::flow_match::Named;
use crate::flow_read::named;

/// Header of the CSV, the columns of `UvPeak`.
const HEADER: &str = "file,kind,rt_apex_min,rt_lo_min,rt_hi_min,fwhm_s,height_mau,area_mau_s,snr";

/// `chrom.uv_peaks@1`: one CSV row per peak of the UV trace of each file.
pub fn uv_step() -> Step {
    Step {
        ins: vec![Port::new("source", DataType::Mzml, "mzML files with a UV/PDA matrix").many()],
        outs: vec![Port::new("peaks", DataType::Peaks, "Peaks of the UV trace, one row each (CSV)")],
        params: vec![
            Param::number("wavelength_nm", "Wavelength")
                .min(0.0)
                .default_num(254.0)
                .unit("nm")
                .doc("The trace is the matrix at this wavelength; the nearest column when none is inside the band"),
            Param::number("band_nm", "Bandwidth")
                .min(0.0)
                .default_num(4.0)
                .unit("nm")
                .doc("The columns within half of this around the wavelength are averaged"),
            Param::boolean("broad", "Broad bands")
                .default_bool(false)
                .doc("Also look for bands too wide for the narrow peak finder"),
            Param::number("broad_min_s", "Narrowest band")
                .min(0.0)
                .default_num(30.0)
                .unit("s")
                .doc("Smallest full width at half maximum of a band"),
            Param::number("broad_max_s", "Widest band")
                .min(0.0)
                .default_num(300.0)
                .unit("s")
                .doc("Largest full width at half maximum of a band"),
            Param::number("broad_snr", "Min. S/N of a band")
                .min(0.0)
                .default_num(8.0)
                .doc("Smallest height of a band over its bridge, in noise units"),
        ],
        runs_in: RunsIn {
            wasm: Some("wasip1 (mzlab-cli)"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.pda.none"],
        ..Step::core(
            "chrom.uv_peaks",
            "Peaks of the UV trace",
            "Integrates the peaks of the UV/PDA trace of each file at one wavelength: apex, edges, width, height and area. With broad bands on it also finds the bands that the narrow peak finder loses. It lists peaks; it does not identify.",
        )
    }
}

fn param(job: &Job, name: &str) -> Option<f64> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

fn flag(job: &Job, name: &str) -> bool {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.bool()).unwrap_or(false)
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

/// The CSV of the peaks of every file under the parameters of the job.
pub fn uv_text(job: &Job, files: &[Named]) -> Result<String> {
    let (nm, band) = (
        param(job, "wavelength_nm").unwrap_or(254.0),
        param(job, "band_nm").unwrap_or(4.0),
    );
    let p = UvParams {
        broad: flag(job, "broad"),
        min_s: param(job, "broad_min_s").unwrap_or(30.0),
        max_s: param(job, "broad_max_s").unwrap_or(300.0),
        snr: param(job, "broad_snr").unwrap_or(8.0),
    };
    let mut out = format!("{HEADER}\n");
    for (name, bytes) in files {
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

/// `chrom.uv_peaks`: one CSV for all the files of `source`.
pub(crate) fn uv_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let files = read_all(r, job, "source")?;
    let text = uv_text(job, &named(&files))?;
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Peaks)
}

#[cfg(test)]
#[path = "flow_uv_tests.rs"]
mod tests;
