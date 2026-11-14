//! The native executor of `chrom.uv_peaks@1`: the peaks of the UV/PDA trace of mzML files at one wavelength, as one CSV. The text is
//! `mzlab_feat::uv_text::uv_text`, shared with the browser (`flowtab`); this file only reads and writes the files. Called by `flow.rs`.

use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};
use mzlab_feat::uv_text::uv_text;

use crate::flow::{io, read_all, FileRunner};
use crate::flow_read::named;

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
            wasm: Some("flowtab"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.pda.none", "err.uv.manifest"],
        ..Step::core(
            "chrom.uv_peaks",
            "Peaks of the UV trace",
            "Integrates the peaks of the UV/PDA trace of each file at one wavelength: apex, edges, width, height and area. With broad bands on it also finds the bands that the narrow peak finder loses. It lists peaks; it does not identify.",
        )
    }
}

/// `chrom.uv_peaks`: one CSV for all the files of `source`.
pub(crate) fn uv_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let files = read_all(r, job, "source")?;
    let text = uv_text(&job.resolved.params, &named(&files), r.cap())?;
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Peaks)
}

#[cfg(test)]
#[path = "flow_uv_tests.rs"]
mod tests;
