//! `Lettore` for the dense case: packets decoded with the real decoders (`opentfraw::pacchetto`), the profile converted with the scan-event coefficients.
use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::SpectrumExtra;
use opentfraw::pacchetto::{decodifica, decodifica_picchi};

use super::Denso;
use crate::denso::COEFF;

impl Lettore for Denso {
    fn formato(&self) -> Formato {
        Formato::Altro("denso-sintetico")
    }
    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }
    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        (c == 0)
            .then_some(&self.voci[..])
            .ok_or_else(|| Error::new("error.badIndex").with("c", c))
    }
    /// The centroid list (the packet is read without its profile, as `extract_arrays` does).
    fn scansione(&mut self, _c: usize, k: usize) -> Result<Spectrum> {
        self.leggi(k)?;
        let picchi = decodifica_picchi(&self.buf).map_err(|_| Error::new("err.file.raw"))?;
        let mz = picchi.iter().map(|p| p.mz).collect();
        let int = picchi.iter().map(|p| p.abundance).collect();
        Ok(self.spettro(k, false, mz, int))
    }
    /// The profile points with m/z above 0, converted from the bins as `profile_arrays` does.
    fn scansione_profilo(&mut self, _c: usize, k: usize) -> Result<Option<Spectrum>> {
        self.leggi(k)?;
        let pacchetto = decodifica(&self.buf).map_err(|_| Error::new("err.file.raw"))?;
        let Some(profilo) = pacchetto.profile else {
            return Ok(None);
        };
        let coppie = profilo.to_mz_intensity(&COEFF);
        let mz = coppie
            .iter()
            .filter(|(m, _)| *m > 0.0)
            .map(|(m, _)| *m)
            .collect();
        let int = coppie
            .iter()
            .filter(|(m, _)| *m > 0.0)
            .map(|(_, i)| *i as f32)
            .collect();
        Ok(Some(self.spettro(k, true, mz, int)))
    }
    fn intestazione(&mut self, _c: usize, _k: usize) -> Result<Vec<Campo>> {
        Ok(Vec::new())
    }
    fn cromatogrammi(&mut self, _c: usize) -> Result<Vec<VoceCromatogramma>> {
        Ok(vec![VoceCromatogramma {
            id: "TIC".into(),
            tipo: ChromKind::Tic,
            unita: "number of detector counts".into(),
        }])
    }
    fn cromatogramma(&mut self, _c: usize, id: &str) -> Result<Chromatogram> {
        let n = self.voci.len();
        Ok(Chromatogram {
            index: 0,
            id: id.into(),
            kind: ChromKind::Tic,
            q1: None,
            q3: None,
            unit: "number of detector counts".into(),
            time: self.voci.iter().map(|v| v.rt).collect(),
            intensity: vec![1.0e9; n],
        })
    }
    fn metodo(&mut self, _c: usize) -> Result<String> {
        Ok(self.nome.clone())
    }
    /// The trailer fields of a Fusion scan as `opentfraw.*` userParams, to give the XML of a spectrum its real weight.
    fn extra_scansione(&mut self, _c: usize, k: usize, _s: &Spectrum) -> SpectrumExtra {
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
