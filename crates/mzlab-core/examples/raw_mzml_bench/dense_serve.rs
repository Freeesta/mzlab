//! `Lettore` for the dense case: packets decoded with the real decoders (`opentfraw::pacchetto`), the profile converted with the scan-event coefficients.
use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{ChromatogramEntry, Field, Format, Reader, Sample, ScanEntry};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::SpectrumExtra;
use opentfraw::pacchetto::{decodifica, decodifica_picchi};

use super::Denso;
use crate::dense::COEFF;

impl Reader for Denso {
    fn format(&self) -> Format {
        Format::Altro("denso-sintetico")
    }
    fn samples(&self) -> &[Sample] {
        &self.campioni
    }
    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        (c == 0)
            .then_some(&self.entries[..])
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }
    /// The centroid list (the packet is read without its profile, as `extract_arrays` does).
    fn scan(&mut self, _c: usize, k: usize) -> Result<Spectrum> {
        self.read(k)?;
        let peaks = decodifica_picchi(&self.buf).map_err(|_| Error::new("err.file.raw"))?;
        let mz = peaks.iter().map(|p| p.mz).collect();
        let int = peaks.iter().map(|p| p.abundance).collect();
        Ok(self.spettro(k, false, mz, int))
    }
    /// The profile points with m/z above 0, converted from the bins as `profile_arrays` does.
    fn scan_profile(&mut self, _c: usize, k: usize) -> Result<Option<Spectrum>> {
        self.read(k)?;
        let packet = decodifica(&self.buf).map_err(|_| Error::new("err.file.raw"))?;
        let Some(profilo) = packet.profile else {
            return Ok(None);
        };
        let pairs = profilo.to_mz_intensity(&COEFF);
        let mz = pairs
            .iter()
            .filter(|(m, _)| *m > 0.0)
            .map(|(m, _)| *m)
            .collect();
        let int = pairs
            .iter()
            .filter(|(m, _)| *m > 0.0)
            .map(|(_, i)| *i as f32)
            .collect();
        Ok(Some(self.spettro(k, true, mz, int)))
    }
    fn scan_header(&mut self, _c: usize, _k: usize) -> Result<Vec<Field>> {
        Ok(Vec::new())
    }
    fn chromatograms(&mut self, _c: usize) -> Result<Vec<ChromatogramEntry>> {
        Ok(vec![ChromatogramEntry {
            id: "TIC".into(),
            tipo: ChromKind::Tic,
            unita: "number of detector counts".into(),
        }])
    }
    fn chromatogram(&mut self, _c: usize, id: &str) -> Result<Chromatogram> {
        let n = self.entries.len();
        Ok(Chromatogram {
            index: 0,
            id: id.into(),
            kind: ChromKind::Tic,
            q1: None,
            q3: None,
            unit: "number of detector counts".into(),
            time: self.entries.iter().map(|v| v.rt).collect(),
            intensity: vec![1.0e9; n],
        })
    }
    fn method(&mut self, _c: usize) -> Result<String> {
        Ok(self.name.clone())
    }
    /// The trailer fields of a Fusion scan as `opentfraw.*` userParams, to give the XML of a spectrum its real weight.
    fn scan_extra(&mut self, _c: usize, k: usize, _s: &Spectrum) -> SpectrumExtra {
        SpectrumExtra {
            params: (0..self.campi_trailer)
                .map(|i| {
                    (
                        format!("opentfraw.trailer.field {i}"),
                        format!("{}", (k * 31 + i * 7) % 9973),
                    )
                })
                .collect(),
            ..SpectrumExtra::default()
        }
    }
}
