//! The TIC and BPC of a timsTOF run (docs/agenti/bruker.md): the TIC of a survey scan from `Frames`, of the other spectra from their points;
//! the BPC from the maxima the reader keeps of the spectra already decoded, decoding only the frames not read yet.
use super::{bad_index, BrukerReader, UNITS};
use mzlab_core::error::{Error, Result};
use mzlab_core::mzml::{ChromKind, Chromatogram};
use mzlab_core::readers::damages_one;

impl BrukerReader {
    pub(super) fn trace(&mut self, id: &str) -> Result<Chromatogram> {
        let (tic, kind, index) = match id {
            "TIC" => (true, ChromKind::Tic, 0),
            "BPC" => (false, ChromKind::Bpc, 1),
            _ => return Err(Error::new("error.badIndex").with("id", id)),
        };
        // the trace follows the survey scans; a run with none (MRM, PRM) is traced over all
        let ms1 = self.desc.iter().any(|d| d.livello == 1);
        let ks: Vec<usize> = (0..self.desc.len())
            .filter(|&k| !ms1 || self.desc[k].livello == 1)
            .collect();
        // the TIC of a survey scan is in `Frames`: only a run without survey scans needs the points
        if tic && !ms1 {
            self.complete()?;
        }
        let mut intensity = Vec::with_capacity(ks.len());
        for &k in &ks {
            intensity.push(if tic {
                self.indice[k].tic
            } else {
                match self.maxima[k] {
                    Some(m) => m,
                    // the peaks of the spectrum, without the m/z axis the maximum does not need; a frame that does not decode is 0, as its spectrum
                    // is written with no points and counted (`readers::Skips`), and it is not decoded again
                    None => {
                        let d = self.desc.get(k).cloned().ok_or_else(|| bad_index(k))?;
                        let m = match self.points(&d) {
                            Ok(p) => maximum(&p.1),
                            Err(e) if damages_one(&e) => 0.0,
                            Err(e) => return Err(e),
                        };
                        self.maxima[k] = Some(m);
                        m
                    }
                }
            });
        }
        Ok(Chromatogram {
            index,
            id: id.into(),
            kind,
            q1: None,
            q3: None,
            unit: UNITS.into(),
            time: ks.iter().map(|&k| self.indice[k].rt).collect(),
            intensity,
        })
    }
}

/// The base peak intensity of a spectrum, over its intensities as 32-bit numbers (those of `Spectrum::intensity`).
pub(super) fn maximum(y: &[f64]) -> f64 {
    y.iter().fold(0.0f64, |m, &x| m.max(f64::from(x as f32)))
}
