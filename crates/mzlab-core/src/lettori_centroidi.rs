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
}

impl Centroidato {
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
        let mut s = self.l.scansione(c, k)?;
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
    let n = l.campioni().len();
    Box::new(Centroidato {
        l,
        params: vec![None; n],
        buf: Buf::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader that gives centroid and profile scans alternately (even scans are the file's own centroid list).
    struct Alternato {
        campioni: Vec<Campione>,
        indice: Vec<VoceScansione>,
    }

    fn voce(k: usize) -> VoceScansione {
        VoceScansione {
            id: k.to_string(),
            rt: k as f64,
            livello: 1,
            polarita: 1,
            tic: 0.0,
            picco_base: None,
            filtro: String::new(),
            precursore: None,
            posizione: None,
        }
    }

    impl Lettore for Alternato {
        fn formato(&self) -> Formato {
            Formato::Altro("finto")
        }
        fn campioni(&self) -> &[Campione] {
            &self.campioni
        }
        fn indice(&mut self, _c: usize) -> Result<&[VoceScansione]> {
            Ok(&self.indice)
        }
        fn scansione(&mut self, _c: usize, k: usize) -> Result<Spectrum> {
            let profile = k % 2 == 1;
            let (mz, y): (Vec<f64>, Vec<f64>) = if profile {
                (0..400)
                    .map(|i| {
                        let m = 100.0 + f64::from(i) * 0.01;
                        let g = |c: f64| 1000.0 * (-((m - c) / 0.05).powi(2)).exp();
                        (m, g(101.0) + g(102.5) + g(103.0))
                    })
                    .unzip()
            } else {
                (vec![101.0, 102.5, 103.0], vec![1000.0, 900.0, 800.0])
            };
            Ok(Spectrum {
                index: k,
                profile,
                mz,
                intensity: y.iter().map(|&v| v as f32).collect(),
                ..Spectrum::default()
            })
        }
        fn intestazione(&mut self, _c: usize, _k: usize) -> Result<Vec<Campo>> {
            Ok(Vec::new())
        }
        fn cromatogrammi(&mut self, _c: usize) -> Result<Vec<VoceCromatogramma>> {
            Ok(Vec::new())
        }
        fn cromatogramma(&mut self, _c: usize, _id: &str) -> Result<Chromatogram> {
            Err(crate::error::Error::new("err.test"))
        }
        fn metodo(&mut self, _c: usize) -> Result<String> {
            Ok(String::new())
        }
    }

    #[test]
    fn centroids_of_the_reader_are_not_touched() {
        let n = 6;
        let l = Alternato {
            campioni: vec![Campione {
                id: 0,
                nome: "a".into(),
                scansioni: n,
            }],
            indice: (0..n).map(voce).collect(),
        };
        let mut c = centroidato(Box::new(l));
        for k in 0..n {
            let s = c.scansione(0, k).unwrap();
            assert!(!s.profile, "scan {k}");
            if k % 2 == 0 {
                assert_eq!(s.mz, vec![101.0, 102.5, 103.0], "scan {k} kept as is");
                assert_eq!(s.intensity, vec![1000.0, 900.0, 800.0]);
            } else {
                assert!(s.mz.len() < 400, "scan {k} reduced to peaks");
            }
        }
    }
}
