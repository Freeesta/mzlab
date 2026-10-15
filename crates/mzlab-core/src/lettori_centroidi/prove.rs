//! `centroidato` leaves alone the scans that the reader gives as centroids.

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
