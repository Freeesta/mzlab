//! A profile mzML already in the browser -> a copy with one point per peak (`MS:1000035`): the original is read twice (parameters first, then the spectra one at a time) and never held.
//! Spectra that are already centroids and every chromatogram are copied as they are.

use std::io::{Read, Write};

use super::{centroids, estimate, Buf, Params};
use crate::error::Result;
use crate::mzml::{self, Item};
use crate::mzmlwrite::{Header, MzmlWriter};

/// The f32 copy of a value written as f64 (`Spectrum::intensity` next to `wide`).
#[allow(clippy::cast_possible_truncation)]
fn narrow(x: f64) -> f32 {
    x as f32
}

/// Kept for the estimate: at most 128 profile scans, spread over the run (the list is halved whenever it is full, so it holds every 2^k-th scan).
struct Sample {
    scans: Vec<(Vec<f64>, Vec<f64>)>,
    every: usize,
    seen: usize,
}

impl Sample {
    fn push(&mut self, s: &mzml::Spectrum) {
        if self.seen.is_multiple_of(self.every) && !s.mz.is_empty() {
            self.scans.push((
                s.mz.clone(),
                (0..s.mz.len()).map(|i| s.intensity_f64(i)).collect(),
            ));
            if self.scans.len() >= 128 {
                self.scans = self.scans.drain(..).step_by(2).collect();
                self.every *= 2;
            }
        }
        self.seen += 1;
    }
}

/// The parameters of the centroids from the profile scans of a stream (`None`: there is no profile scan) and the number of spectra.
fn params_of<R: Read>(reader: R) -> Result<(Option<Params>, usize)> {
    let mut sample = Sample {
        scans: Vec::new(),
        every: 1,
        seen: 0,
    };
    let mut count = 0;
    mzml::parse(reader, |item| {
        if let Item::Spectrum(s) = item {
            count += 1;
            if s.profile {
                sample.push(&s);
            }
        }
    })?;
    if sample.scans.is_empty() {
        return Ok((None, count));
    }
    let n = sample.scans.len().min(64);
    let refs: Vec<(&[f64], &[f64])> = (0..n)
        .map(|i| &sample.scans[i * sample.scans.len() / n])
        .map(|(a, b)| (a.as_slice(), b.as_slice()))
        .collect();
    Ok((Some(estimate(&refs, 0.7).params), count))
}

/// Writes `name` as a centroid mzML. `open()` gives the file from its start (called twice: parameters and count, then the spectra); `progress(done, total)` after every few spectra, `total` = spectra of the file.
/// Returns false when the file has no profile spectrum (nothing was written).
pub fn mzml_to_centroids<R: Read, W: Write>(
    mut open: impl FnMut() -> Result<R>,
    out: W,
    name: &str,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<bool> {
    let (p, count) = params_of(open()?)?;
    let Some(p) = p else {
        return Ok(false);
    };
    let header = Header {
        run: name
            .trim_end_matches(".mzML")
            .trim_end_matches(".mzml")
            .to_string(),
        sources: vec![name.to_string()],
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        centroided: true,
        ..Default::default()
    };
    let mut w = MzmlWriter::new(out, &header)?;
    let mut buf = Buf::default();
    let mut chroms = Vec::new();
    let mut done = 0;
    let mut failed = None;
    progress(0, count);
    if count > 0 {
        w.open_spectra(count)?;
    }
    mzml::parse(open()?, |item| {
        if failed.is_some() {
            return;
        }
        match item {
            Item::Spectrum(mut s) => {
                if s.profile {
                    let y: Vec<f64> = (0..s.mz.len()).map(|i| s.intensity_f64(i)).collect();
                    let c = centroids(&s.mz, &y, &p, &mut buf);
                    s.mz = c.iter().map(|c| c.mz).collect();
                    let v: Vec<f64> = c.iter().map(|c| c.value).collect();
                    s.intensity = v.iter().map(|&x| narrow(x)).collect();
                    s.wide = Some(v);
                    s.profile = false;
                }
                if let Err(e) = w.spectrum(&s) {
                    failed = Some(e);
                }
                done += 1;
                if done % 32 == 0 {
                    progress(done, count);
                }
            }
            Item::Chromatogram(c) => chroms.push(c),
            Item::Analyzer(_) => {}
        }
    })?;
    if let Some(e) = failed {
        return Err(e);
    }
    progress(count, count);
    if count > 0 {
        w.close_spectra()?;
    }
    if !chroms.is_empty() {
        w.open_chromatograms(chroms.len())?;
        for c in &chroms {
            w.chromatogram(c)?;
        }
        w.close_chromatograms()?;
    }
    w.finish()?;
    Ok(true)
}
