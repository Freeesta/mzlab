//! `impl Lettore` of the Shimadzu reader (the struct is in `mod.rs`).

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

use super::carica::Fam;
use super::{meta, rec, rifiuto, LettoreShimadzu, F32_EXACT};

impl Lettore for LettoreShimadzu {
    fn formato(&self) -> Formato {
        Formato::Altro("shimadzu")
    }

    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }

    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.uno(c)?;
        Ok(&self.d.indice)
    }

    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.uno(c)?;
        let (v, &(a, n, f)) = self
            .d
            .indice
            .get(k)
            .zip(self.d.pos.get(k))
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        let r = self
            .d
            .raw
            .get(a..a + n)
            .ok_or_else(|| rifiuto("scan outside the raw stream"))?;
        let pts: Vec<(f64, u64)> = (0..f.npk)
            .map(|j| {
                let (mz, _, y) = rec::punto(r, &f, j);
                (f64::from(mz) / 20.0, y)
            })
            .collect();
        let wide = pts
            .iter()
            .any(|p| p.1 > F32_EXACT)
            .then(|| pts.iter().map(|p| p.1 as f64).collect());
        Ok(Spectrum {
            index: k,
            id: v.id.clone(),
            level: v.livello,
            rt: v.rt,
            polarity: v.polarita,
            profile: false,
            filter: v.filtro.clone(),
            precursor: None,
            mz: pts.iter().map(|p| p.0).collect(),
            intensity: pts.iter().map(|p| p.1 as f32).collect(),
            wide,
        })
    }

    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.uno(c)?;
        let (v, &(a, _, f)) = self
            .d
            .indice
            .get(k)
            .zip(self.d.pos.get(k))
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        let campo = |nome: &str, valore: String| Campo {
            nome: nome.into(),
            valore,
            unita: None,
            cv: None,
        };
        let mut out = vec![
            campo("record", (k + 1).to_string()),
            campo("offset", a.to_string()),
            campo("TIC", v.tic.to_string()),
        ];
        if self.fam == Fam::Lc {
            out.push(campo("event", (usize::from(f.ev) + 1).to_string()));
        }
        Ok(out)
    }

    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.uno(c)?;
        Ok(self
            .d
            .cromat
            .iter()
            .map(|h| VoceCromatogramma {
                id: h.id.clone(),
                tipo: h.kind,
                unita: h.unit.clone(),
            })
            .collect())
    }

    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.uno(c)?;
        self.d
            .cromat
            .iter()
            .find(|h| h.id == id)
            .cloned()
            .ok_or_else(|| Error::new("error.badIndex").with("id", id))
    }

    fn metodo(&mut self, c: usize) -> Result<String> {
        self.uno(c)?;
        Ok(meta::documento(
            self.id(),
            self.fam,
            &self.d,
            &self.meta,
            true,
        ))
    }

    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.uno(c)?;
        Ok(Some(meta::header(self.id(), self.fam, &self.d, &self.meta)))
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        SpectrumExtra {
            sim: s.filter.contains("SIM"),
            ..SpectrumExtra::default()
        }
    }
}
