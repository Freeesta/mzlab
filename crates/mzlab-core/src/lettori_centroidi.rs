//! A reader whose profile scans are reduced to one point per peak (`centroidi`, MS:1000035), as the conversion of a SCIEX `.wiff` with `centroid`
//! does (`wiff::Wiff::to_mzml_progress`): the parameters of each sample come from at most 64 equally spaced scans of its index (`centroidi::estimate`).
//! Everything else is the reader's.

use crate::centroidi::{self, Buf, Params};
use crate::error::Result;
use crate::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use crate::mzml::{Chromatogram, Spectrum};
use crate::mzmlwrite::{Header, SpectrumExtra};

pub struct Centroidato {
    l: Box<dyn Lettore>,
    params: Vec<Option<Params>>,
    buf: Buf,
    /// The scan last given by `grezza`, kept for the `scansione` that asks for the same one (the two versions of an index from one decoding).
    ultima: Option<(usize, usize, Spectrum)>,
}

impl Centroidato {
    pub fn nuovo(l: Box<dyn Lettore>) -> Centroidato {
        let n = l.campioni().len();
        Centroidato {
            l,
            params: vec![None; n],
            buf: Buf::default(),
            ultima: None,
        }
    }

    /// The reader underneath: the one of the version with the scans as the file has them (profile stays profile).
    pub fn semplice(&mut self) -> &mut dyn Lettore {
        &mut *self.l
    }

    /// Scan `k` of sample `c` as the reader gives it, decoded now and kept: the next `scansione(c, k)` centroids this one instead of decoding it again.
    pub fn grezza(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let s = self.l.scansione(c, k)?;
        self.ultima = Some((c, k, s.clone()));
        Ok(s)
    }

    fn params(&mut self, c: usize) -> Result<Params> {
        if let Some(Some(p)) = self.params.get(c) {
            return Ok(p.clone());
        }
        let len = self.l.indice(c)?.len();
        let n = len.min(64);
        let mut scans = Vec::new();
        for i in 0..n {
            let s = self.l.scansione(c, i * len / n)?;
            let y = s
                .wide
                .unwrap_or_else(|| s.intensity.iter().map(|&v| f64::from(v)).collect());
            scans.push((s.mz, y));
        }
        let refs: Vec<(&[f64], &[f64])> = scans
            .iter()
            .map(|(a, b)| (a.as_slice(), b.as_slice()))
            .collect();
        let p = centroidi::estimate(&refs, 0.7).params;
        if let Some(slot) = self.params.get_mut(c) {
            *slot = Some(p.clone());
        }
        Ok(p)
    }
}

/// The intensity kept in 32 bits, as the mzML of the conversion writes it.
#[allow(clippy::cast_possible_truncation)]
fn narrow(x: f64) -> f32 {
    x as f32
}

impl Lettore for Centroidato {
    fn formato(&self) -> Formato {
        self.l.formato()
    }
    fn campioni(&self) -> &[Campione] {
        self.l.campioni()
    }
    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        self.l.indice(c)
    }
    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let mut s = match self.ultima.take() {
            Some((cc, kk, s)) if cc == c && kk == k => s,
            _ => self.l.scansione(c, k)?,
        };
        if !s.profile {
            return Ok(s);
        }
        let p = self.params(c)?;
        let y = s
            .wide
            .take()
            .unwrap_or_else(|| s.intensity.iter().map(|&v| f64::from(v)).collect());
        let cs = centroidi::centroids(&s.mz, &y, &p, &mut self.buf);
        s.mz = cs.iter().map(|c| c.mz).collect();
        let v: Vec<f64> = cs.iter().map(|c| c.value).collect();
        s.intensity = v.iter().map(|&x| narrow(x)).collect();
        s.wide = Some(v);
        s.profile = false;
        Ok(s)
    }
    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        self.l.intestazione(c, k)
    }
    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        self.l.cromatogrammi(c)
    }
    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        self.l.cromatogramma(c, id)
    }
    fn metodo(&mut self, c: usize) -> Result<String> {
        self.l.metodo(c)
    }
    /// The header of the reader, marked as centroided (the step of the processing the mzML names).
    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        let h = self.l.testata_mzml(c)?;
        Ok(h.map(|h| Header {
            centroided: true,
            ..h
        }))
    }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra {
        self.l.extra_mzml(s)
    }
}

/// `l` with its profile scans as centroids.
pub fn centroidato(l: Box<dyn Lettore>) -> Box<dyn Lettore> {
    Box::new(Centroidato::nuovo(l))
}

#[cfg(test)]
mod prove;
