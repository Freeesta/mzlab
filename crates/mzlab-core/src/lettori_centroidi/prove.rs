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

fn alternato(n: usize) -> Alternato {
    Alternato {
        campioni: vec![Campione {
            id: 0,
            nome: "a".into(),
            scansioni: n,
        }],
        indice: (0..n).map(voce).collect(),
    }
}

/// The two indexes of one decoding are the two built apart, whatever the budget (rounds, scratch file shared by the two builders).
#[test]
fn double_index_equals_the_two_apart() {
    use crate::upload::indicizza_lettore_con;
    use std::io::Cursor;
    let n = 301;
    let apart = |centroid: bool, budget: usize, spill: bool| {
        let mut l: Box<dyn Lettore> = Box::new(alternato(n));
        if centroid {
            l = centroidato(l);
        }
        let mut out = Cursor::new(Vec::new());
        let mut sp = Cursor::new(Vec::new());
        let (st, sha) = indicizza_lettore_con(
            &mut *l,
            0,
            &mut || Ok(vec![]),
            "a.raw",
            &mut out,
            budget,
            spill.then_some(&mut sp as &mut dyn crate::mzidx_spill::Spill),
            &mut |_| {},
        )
        .unwrap();
        (out.into_inner(), st.peaks, sha)
    };
    for (budget, spill) in [(1usize << 30, false), (1 << 21, true), (1 << 20, true)] {
        let mut l = Centroidato::nuovo(Box::new(alternato(n)));
        let (mut a, mut b) = (Cursor::new(Vec::new()), Cursor::new(Vec::new()));
        let mut sp = Cursor::new(Vec::new());
        let ((sa, sha_a), (sb, sha_b)) = crate::upload::indicizza_lettore_doppio(
            &mut l,
            0,
            &mut || Ok(vec![]),
            "a.raw",
            &mut a,
            &mut b,
            budget,
            spill.then_some(&mut sp as &mut dyn crate::mzidx_spill::Spill),
            &mut |_| {},
        )
        .unwrap();
        let (pa, peaks_a, sha) = apart(false, 1 << 30, false);
        let (pb, peaks_b, _) = apart(true, 1 << 30, false);
        assert!(
            a.into_inner() == pa,
            "plain index differs (budget {budget}, spill {spill})"
        );
        assert!(
            b.into_inner() == pb,
            "centroid index differs (budget {budget}, spill {spill})"
        );
        assert_eq!((sa.peaks, sb.peaks), (peaks_a, peaks_b));
        assert!(peaks_b < peaks_a, "the profile scans lose points");
        assert_eq!((sha_a, sha_b), (sha.clone(), sha));
    }
}
