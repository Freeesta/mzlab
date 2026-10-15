//! SRM/MRM stored as scans: a level-2 scan with an SRM filter whose points are the products of one precursor becomes, per transition, a chromatogram
//! with the id `SRM SIC Q1=<q1> Q3=<q3>` that `wiff/run.rs` writes and `mzmlhead::srm_id` reads.
use std::collections::HashMap;

use mzlab_core::error::Result;
use mzlab_core::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

/// Products closer than this (m/z) to the Q3 of a transition belong to it.
pub const TOLLERANZA_Q3: f64 = 0.005;
/// Precursors closer than this (m/z) to the Q1 of a transition belong to it.
pub const TOLLERANZA_Q1: f64 = 0.005;
const UNITA: &str = "number of detector counts";

fn filtro_srm(f: &str) -> bool {
    let f = f.to_ascii_lowercase();
    f.contains("srm") || f.contains("mrm")
}

struct Transizione {
    q1: f64,
    q3: f64,
    time: Vec<f64>,
    intensity: Vec<f64>,
}

/// A reader that also gives the SRM traces of the scans of the reader it wraps (unless that one has SRM chromatograms of its own).
pub struct SrmDaScansioni {
    inner: Box<dyn Lettore>,
    cache: HashMap<usize, Vec<Chromatogram>>,
}

impl SrmDaScansioni {
    pub fn new(inner: Box<dyn Lettore>) -> Self {
        SrmDaScansioni {
            inner,
            cache: HashMap::new(),
        }
    }

    fn tracce(&mut self, c: usize) -> Result<&Vec<Chromatogram>> {
        if !self.cache.contains_key(&c) {
            let t = self.costruisci(c)?;
            self.cache.insert(c, t);
        }
        Ok(&self.cache[&c])
    }

    fn costruisci(&mut self, c: usize) -> Result<Vec<Chromatogram>> {
        if self
            .inner
            .cromatogrammi(c)?
            .iter()
            .any(|v| v.tipo == ChromKind::Srm)
        {
            return Ok(Vec::new());
        }
        let righe: Vec<VoceScansione> = self.inner.indice(c)?.to_vec();
        let mut tr: Vec<Transizione> = Vec::new();
        for (k, r) in righe.iter().enumerate().filter(|(_, r)| scansione_srm(r)) {
            let q1 = r.precursore.unwrap_or(0.0);
            let s = self.inner.scansione(c, k)?;
            for (i, &q3) in s.mz.iter().enumerate() {
                let y = s.intensity_f64(i);
                let j = match tr.iter().position(|t| {
                    (t.q1 - q1).abs() <= TOLLERANZA_Q1 && (t.q3 - q3).abs() <= TOLLERANZA_Q3
                }) {
                    Some(j) => j,
                    None => {
                        tr.push(Transizione {
                            q1,
                            q3,
                            time: Vec::new(),
                            intensity: Vec::new(),
                        });
                        tr.len() - 1
                    }
                };
                tr[j].time.push(r.rt);
                tr[j].intensity.push(y);
            }
        }
        Ok(tr
            .into_iter()
            .enumerate()
            .map(|(index, t)| Chromatogram {
                index,
                id: format!("SRM SIC Q1={} Q3={}", t.q1, t.q3),
                kind: ChromKind::Srm,
                q1: Some(t.q1),
                q3: Some(t.q3),
                unit: UNITA.into(),
                time: t.time,
                intensity: t.intensity,
            })
            .collect())
    }
}

fn scansione_srm(v: &VoceScansione) -> bool {
    v.livello == 2 && v.precursore.is_some() && filtro_srm(&v.filtro)
}

impl Lettore for SrmDaScansioni {
    fn formato(&self) -> Formato {
        self.inner.formato()
    }
    fn campioni(&self) -> &[Campione] {
        self.inner.campioni()
    }
    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.inner.indice(c)
    }
    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        self.inner.scansione(c, k)
    }
    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.inner.intestazione(c, k)
    }
    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        let mut v = self.inner.cromatogrammi(c)?;
        let extra = self.tracce(c)?;
        v.extend(extra.iter().map(|t| VoceCromatogramma {
            id: t.id.clone(),
            tipo: t.kind,
            unita: t.unit.clone(),
        }));
        Ok(v)
    }
    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        if let Some(t) = self.tracce(c)?.iter().find(|t| t.id == id) {
            return Ok(t.clone());
        }
        self.inner.cromatogramma(c, id)
    }
    fn metodo(&mut self, c: usize) -> Result<String> {
        self.inner.metodo(c)
    }
    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        self.inner.testata_mzml(c)
    }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        self.inner.extra_mzml(s)
    }
    fn variante(&mut self, c: usize) -> bool {
        self.inner.variante(c)
    }
}

/// True when sample `c` of `l` has scans that are SRM transitions.
pub fn ha_scansioni_srm(l: &mut dyn Lettore, c: usize) -> bool {
    l.indice(c)
        .map(|i| i.iter().any(scansione_srm))
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "srm_prove.rs"]
mod tests;
