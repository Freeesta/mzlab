//! `Picchi` of the mzML conversion on a fake reader with a gaussian profile at known centres: `Mzlab` gives the centres, `Profilo` the profile points,
//! `File` the centroids of the file.

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use mzlab_core::lettori_mzml::{Conversione, Picchi};
use mzlab_core::mzml::{self, Chromatogram, Item, Spectrum};

const CENTRI: [f64; 3] = [101.0, 102.5, 104.0];

struct Finto {
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
    profilo_ok: bool,
}

fn finto(profilo_ok: bool) -> Finto {
    let n = 5;
    Finto {
        campioni: vec![Campione {
            id: 0,
            nome: "a".into(),
            scansioni: n,
        }],
        indice: (0..n)
            .map(|k| VoceScansione {
                id: k.to_string(),
                rt: k as f64,
                livello: 1,
                polarita: 1,
                tic: 0.0,
                picco_base: None,
                filtro: String::new(),
                precursore: None,
                posizione: None,
            })
            .collect(),
        profilo_ok,
    }
}

fn spettro(k: usize, profile: bool) -> Spectrum {
    let (mz, y): (Vec<f64>, Vec<f64>) = if profile {
        (0..400)
            .map(|i| {
                let m = 100.0 + f64::from(i) * 0.01;
                let g = |c: f64| 1000.0 * (-((m - c) / 0.2).powi(2)).exp();
                (m, CENTRI.iter().map(|&c| g(c)).sum::<f64>())
            })
            .unzip()
    } else {
        (CENTRI.to_vec(), vec![1000.0, 900.0, 800.0])
    };
    Spectrum {
        index: k,
        level: 1,
        rt: k as f64,
        polarity: 1,
        profile,
        mz,
        intensity: y.iter().map(|&v| v as f32).collect(),
        ..Spectrum::default()
    }
}

impl Lettore for Finto {
    fn formato(&self) -> Formato {
        Formato::Altro("finto")
    }
    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }
    fn indice(&mut self, _c: usize) -> Result<&[VoceScansione]> {
        Ok(&self.indice)
    }
    // The scans as the reader gives them: profile, or the centroids of the maker when `profilo_ok` is false.
    fn scansione(&mut self, _c: usize, k: usize) -> Result<Spectrum> {
        Ok(spettro(k, self.profilo_ok))
    }
    fn scansione_profilo(&mut self, _c: usize, k: usize) -> Result<Option<Spectrum>> {
        Ok(self.profilo_ok.then(|| spettro(k, true)))
    }
    fn intestazione(&mut self, _c: usize, _k: usize) -> Result<Vec<Campo>> {
        Ok(Vec::new())
    }
    fn cromatogrammi(&mut self, _c: usize) -> Result<Vec<VoceCromatogramma>> {
        Ok(Vec::new())
    }
    fn cromatogramma(&mut self, _c: usize, _id: &str) -> Result<Chromatogram> {
        Err(Error::new("err.test"))
    }
    fn metodo(&mut self, _c: usize) -> Result<String> {
        Ok(String::new())
    }
}

fn converti(l: &mut Finto, peaks: Picchi) -> Result<(String, Vec<Spectrum>)> {
    let conv = Conversione {
        peaks,
        ..Conversione::default()
    };
    let bytes = conv.scrivi(l, 0, Vec::new(), &mut |_, _| {})?;
    let mut v = Vec::new();
    mzml::parse(bytes.as_slice(), |i| {
        if let Item::Spectrum(s) = i {
            v.push(s);
        }
    })?;
    Ok((String::from_utf8(bytes).unwrap(), v))
}

#[test]
fn lettori_mzml_picchi_mzlab_gives_the_centres_within_half_a_ppm() {
    let (doc, spettri) = converti(&mut finto(true), Picchi::Mzlab).unwrap();
    assert!(doc.contains("MS:1000035") && !doc.contains("MS:1000128"));
    assert!(doc.contains("mzLab centroids"));
    assert_eq!(spettri.len(), 5);
    for s in &spettri {
        assert_eq!(s.mz.len(), CENTRI.len(), "scan {}", s.index);
        for (m, c) in s.mz.iter().zip(CENTRI) {
            assert!((m - c).abs() / c * 1e6 < 0.5, "{m} vs {c}");
        }
    }
}

#[test]
fn lettori_mzml_picchi_profilo_gives_the_profile_points() {
    let (doc, spettri) = converti(&mut finto(true), Picchi::Profilo).unwrap();
    assert!(doc.contains("MS:1000128") && !doc.contains("MS:1000035"));
    assert!(spettri.iter().all(|s| s.profile && s.mz.len() == 400));
    let e = converti(&mut finto(false), Picchi::Profilo).unwrap_err();
    assert_eq!(e.key, "err.conv.noProfile");
}

#[test]
fn lettori_mzml_picchi_file_keeps_the_scans_of_the_reader() {
    let (doc, spettri) = converti(&mut finto(true), Picchi::File).unwrap();
    assert!(doc.contains("MS:1000128") && !doc.contains("MS:1000035"));
    assert!(spettri.iter().all(|s| s.mz.len() == 400));
}
