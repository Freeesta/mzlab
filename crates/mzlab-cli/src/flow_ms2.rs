//! The native executor of `spectra.ms2@1`: the MS2 scans of mzML files (DDA) as an MGF `spectra` file, with the precursor, charge, RT and the precursor
//! intensity of every scan. Takes names and bytes and returns the text, like `flow_match`; the MGF feeds `spectra.match`, `formula.ms2`.

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::mzml::{self, Spectrum};
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow_match::Named;

/// `spectra.ms2@1`: one MGF record per MS2 scan of the files that passes the intensity threshold and the isolation window.
pub fn ms2_step() -> Step {
    Step {
        ins: vec![Port::new("source", DataType::Mzml, "mzML files with data-dependent MS2").many()],
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
        ],
        runs_in: RunsIn {
            wasm: Some("wasip1 (mzlab-cli)"),
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.file.numpress"],
        ..Step::core(
            "spectra.ms2",
            "MS2 spectra from mzML",
            "Reads the MS2 scans of data-dependent runs and writes them as spectra with precursor m/z and intensity, charge, RT and scan number.",
        )
    }
}

/// What the text of a spectrum says that `mzml::Spectrum` does not keep.
#[derive(Default, Clone, Copy)]
struct Extra {
    charge: Option<i64>,
    width: Option<f64>,
    prec_int: Option<f64>,
}

/// The value of the cvParam `acc` inside a `<spectrum>` block.
fn cv_value(block: &str, acc: &str) -> Option<f64> {
    let at = block.find(&format!("accession=\"{acc}\""))?;
    let start = block[..at].rfind('<')?;
    let tag = &block[start..start + block[start..].find('>')?];
    let v = tag.split_once("value=\"")?.1;
    v.split('"').next()?.parse().ok()
}

/// Extras of every spectrum in file order (`<spectrum ` opens one; `<spectrumList` does not match).
fn extras(text: &str) -> Vec<Extra> {
    text.split("<spectrum ")
        .skip(1)
        .map(|b| {
            let b = b.split("</spectrumList").next().unwrap_or(b);
            let width = cv_value(b, "MS:1000828").zip(cv_value(b, "MS:1000829"));
            Extra {
                charge: cv_value(b, "MS:1000041").map(|c| c as i64),
                width: width.map(|(lo, hi)| lo + hi),
                prec_int: cv_value(b, "MS:1000042"),
            }
        })
        .collect()
}

fn param(job: &Job, name: &str) -> Option<f64> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |p| p.0)
}

fn record(out: &mut String, title: &str, s: &Spectrum, e: Extra, peaks: &[(f64, f64)]) {
    let (sign, mode) = match s.polarity {
        1 => ('+', "Positive"),
        -1 => ('-', "Negative"),
        _ => ('+', ""),
    };
    out.push_str(&format!(
        "BEGIN IONS\nTITLE={title}\nSPECTRUMID={title}\nPEPMASS={:.5}",
        s.precursor.unwrap_or(0.0)
    ));
    if let Some(i) = e.prec_int {
        out.push_str(&format!(" {i:.1}"));
    }
    out.push_str(&format!(
        "\nRTINSECONDS={:.3}\nSCANS={}\nMSLEVEL=2\n",
        s.rt * 60.0,
        s.index + 1
    ));
    if let Some(c) = e.charge.filter(|c| *c > 0) {
        out.push_str(&format!("CHARGE={c}{sign}\n"));
    }
    if !mode.is_empty() {
        out.push_str(&format!("IONMODE={mode}\n"));
    }
    for (m, i) in peaks {
        out.push_str(&format!("{m:.5} {i:.1}\n"));
    }
    out.push_str("END IONS\n");
}

/// Runs the step of `job` on the mzML files and returns the MGF text: files in the given order, scans in file order, `TITLE` and `SPECTRUMID` (the query id of `spectra.match`) `<file>.<scan>`.
pub fn ms2_text(job: &Job, files: &[Named]) -> Result<String> {
    let floor = param(job, "min_intensity").unwrap_or(0.0);
    let widest = param(job, "max_isolation");
    let mut out = String::new();
    for (name, bytes) in files {
        let run = mzml::read(*bytes)?;
        let text = String::from_utf8_lossy(bytes);
        let extra = extras(&text);
        let extra = if extra.len() == run.spectra.len() {
            extra
        } else {
            vec![Extra::default(); run.spectra.len()]
        };
        for (s, e) in run.spectra.iter().zip(extra) {
            if s.level != 2 || s.precursor.is_none() {
                continue;
            }
            if widest.zip(e.width).is_some_and(|(max, w)| w > max) {
                continue;
            }
            let peaks: Vec<(f64, f64)> = (0..s.mz.len())
                .map(|i| (s.mz[i], s.intensity_f64(i)))
                .filter(|p| p.1 >= floor && p.1 > 0.0)
                .collect();
            if !peaks.is_empty() {
                record(
                    &mut out,
                    &format!("{}.{}", stem(name), s.index + 1),
                    s,
                    e,
                    &peaks,
                );
            }
        }
    }
    if out.is_empty() && files.is_empty() {
        return Err(Error::new("err.flow.port.missing").with("port", "source"));
    }
    Ok(out)
}
