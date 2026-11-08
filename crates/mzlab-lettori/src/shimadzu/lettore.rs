//! `impl Lettore` of the Shimadzu reader (the struct is in `mod.rs`).

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

use super::carica::Fam;
use super::{meta, rec, rifiuto, ShimadzuReader, F32_EXACT};

impl Lettore for ShimadzuReader {
    fn format(&self) -> Formato {
        Formato::Altro("shimadzu")
    }

    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        use mzlab_core::conv::{IDENTITA, NON_DECODIFICATO};
        &[
            ("oven and injector programs, GCMS Configuration, embedded .qgm method, Status curves, MS Status Data, Mass Parameters", NON_DECODIFICATO),
            ("UV (LSS Raw Data, PDA 3D Raw Data)", NON_DECODIFICATO),
            ("operator, computer, paths and file names", IDENTITA),
        ]
    }

    fn samples(&self) -> &[Campione] {
        &self.campioni
    }

    fn index(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.one(c)?;
        Ok(&self.d.indice)
    }

    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.one(c)?;
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
                let (mz, _, y) = rec::point(r, &f, j);
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

    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.one(c)?;
        let (v, &(a, _, f)) = self
            .d
            .indice
            .get(k)
            .zip(self.d.pos.get(k))
            .ok_or_else(|| Error::new("error.badIndex").with("k", k))?;
        let field = |nome: &str, valore: String| Campo {
            nome: nome.into(),
            valore,
            unita: None,
            cv: None,
        };
        let mut out = vec![
            field("record", (k + 1).to_string()),
            field("offset", a.to_string()),
            field("TIC", v.tic.to_string()),
        ];
        if self.fam == Fam::Lc {
            out.push(field("event", (usize::from(f.ev) + 1).to_string()));
        }
        Ok(out)
    }

    fn chromatograms(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.one(c)?;
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

    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.one(c)?;
        self.d
            .cromat
            .iter()
            .find(|h| h.id == id)
            .cloned()
            .ok_or_else(|| Error::new("error.badIndex").with("id", id))
    }

    fn method(&mut self, c: usize) -> Result<String> {
        self.one(c)?;
        Ok(meta::document(
            self.id(),
            self.fam,
            &self.d,
            &self.meta,
            true,
        ))
    }

    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        self.one(c)?;
        Ok(Some(meta::header(self.id(), self.fam, &self.d, &self.meta)))
    }

    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        SpectrumExtra {
            sim: s.filter.contains("SIM"),
            ..SpectrumExtra::default()
        }
    }
}
