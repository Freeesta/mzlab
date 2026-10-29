//! The native executor of `spectra.ms2@1`: the descriptor of the step and the wrapper of its text (`mzlab_feat::flow_ms2`, the one implementation the browser runs
//! too). Takes names and bytes and returns the text, like `flow_match`; the MGF feeds `spectra.match`, `formula.ms2`.

use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};
use mzlab_feat::flow_ms2::{Ms2Params, Named};

/// `spectra.ms2@1`: one MGF record per MS2 scan of the files that passes the intensity threshold and the isolation window.
pub fn ms2_step() -> Step {
    Step {
        ins: vec![
            Port::new("source", DataType::Mzml, "mzML files with data-dependent MS2").many(),
            Port {
                optional: true,
                ..Port::new("features", DataType::Report, "Feature table: keep the scans of its features")
            },
        ],
        outs: vec![Port::new("spectra", DataType::Spectra, "MS2 spectra as MGF")],
        params: vec![
            Param::number("min_intensity", "Peak threshold")
                .min(0.0)
                .default_num(0.0)
                .unit("counts")
                .doc("Peaks below this intensity are dropped; a scan left with no peak is dropped"),
            Param::number("max_isolation", "Largest isolation window")
                .min(0.0)
                .unit("Da")
                .doc("Scans whose isolation window is wider are dropped; off when absent, and a scan that does not say its window is kept"),
            Param::number("tol_da", "Precursor tolerance")
                .min(0.0)
                .default_num(0.02)
                .unit("Da")
                .doc("With a feature table: largest distance between the precursor and the m/z of the feature"),
            Param::number("rt_tol", "RT tolerance")
                .min(0.0)
                .default_num(0.2)
                .unit("min")
                .doc("With a feature table: largest distance between the scan and the RT of the feature"),
            Param::boolean("dia_pseudo", "DIA pseudo spectra")
                .default_bool(false)
                .doc("For data-independent runs (SWATH, MSE): one pseudo MS2 spectrum per feature of the table, made of the fragments whose trace follows that of the precursor; needs the feature table, and the DDA scans are not written"),
            Param::number("dia_min_corr", "Fragment correlation")
                .min(0.0)
                .max(1.0)
                .default_num(0.9)
                .doc("With `dia_pseudo`: least Pearson correlation between the trace of a fragment and that of the precursor"),
            Param::integer("dia_min_points", "Cycles per peak")
                .min(3.0)
                .default_num(5.0)
                .doc("With `dia_pseudo`: fewest cycles of the RT range of a feature; a feature with fewer has no spectrum"),
            Param::number("dia_frag_ppm", "Fragment tolerance")
                .min(0.0)
                .default_num(10.0)
                .unit("ppm")
                .doc("With `dia_pseudo`: window around an m/z to follow its trace"),
        ],
        runs_in: RunsIn {
            wasm: Some("hr"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.file.numpress"],
        ..Step::core(
            "spectra.ms2",
            "MS2 spectra from mzML",
            "Reads the MS2 scans of data-dependent runs and writes them as spectra with precursor m/z and intensity, charge, RT and scan number; given a feature table it keeps the scans of its features and names each spectrum with the feature id.",
        )
    }
}

/// Runs the step of `job` on the mzML files and returns the MGF text: files in the given order, scans in file order, `TITLE` `<file>.<scan>`.
/// `SPECTRUMID` (the query id of `spectra.match`) is the same, or with a feature table the id of the feature of the scan, and the scans of no feature go.
pub fn ms2_text(job: &Job, files: &[Named], features: Option<&[u8]>) -> Result<String> {
    mzlab_feat::flow_ms2::ms2_text(
        &Ms2Params::from_params(&job.resolved.params),
        files,
        features,
    )
}
