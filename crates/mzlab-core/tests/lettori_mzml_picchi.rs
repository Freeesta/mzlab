//! `Picchi` of the mzML conversion on a fake reader with a gaussian profile at known centres: `Mzlab` gives the centres, `Profilo` the profile points,
//! `File` the centroids of the file.

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{ChromatogramEntry, Field, Format, Reader, Sample, ScanEntry};
use mzlab_core::lettori_mzml::{Conversion, Peaks};
use mzlab_core::mzml::{self, Chromatogram, Item, Spectrum};

const CENTRI: [f64; 3] = [101.0, 102.5, 104.0];

struct Finto {
    campioni: Vec<Sample>,
    indice: Vec<ScanEntry>,
    profilo_ok: bool,
}

fn finto(profilo_ok: bool) -> Finto {
    let n = 5;
    Finto {
        campioni: vec![Sample {
            id: 0,
            nome: "a".into(),
            scansioni: n,
            size: None,
        }],
        indice: (0..n)
            .map(|k| ScanEntry {
                id: k.to_string(),
                rt: k as f64,
                livello: 1,
                polarita: 1,
                tic: 0.0,
                base_peak: None,
                filtro: String::new(),
                precursore: None,
                position: None,
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

impl Reader for Finto {
    fn format(&self) -> Format {
        Format::Altro("finto")
    }
    fn samples(&self) -> &[Sample] {
        &self.campioni
    }
    fn index(&mut self, _c: usize) -> Result<&[ScanEntry]> {
        Ok(&self.indice)
    }
    // The scans as the reader gives them: profile, or the centroids of the maker when `profilo_ok` is false.
    fn scan(&mut self, _c: usize, k: usize) -> Result<Spectrum> {
        Ok(spettro(k, self.profilo_ok))
    }
    fn scan_profile(&mut self, _c: usize, k: usize) -> Result<Option<Spectrum>> {
        Ok(self.profilo_ok.then(|| spettro(k, true)))
    }
    fn scan_header(&mut self, _c: usize, _k: usize) -> Result<Vec<Field>> {
        Ok(Vec::new())
    }
    fn chromatograms(&mut self, _c: usize) -> Result<Vec<ChromatogramEntry>> {
        Ok(Vec::new())
    }
    fn chromatogram(&mut self, _c: usize, _id: &str) -> Result<Chromatogram> {
        Err(Error::new("err.test"))
    }
    fn method(&mut self, _c: usize) -> Result<String> {
        Ok(String::new())
    }
}

fn converti(l: &mut Finto, peaks: Peaks) -> Result<(String, Vec<Spectrum>)> {
    let conv = Conversion {
        peaks,
        ..Conversion::default()
    };
    let bytes = conv.write(l, 0, Vec::new(), &mut |_, _| {})?;
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
    let (doc, spettri) = converti(&mut finto(true), Peaks::Mzlab).unwrap();
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
    let (doc, spettri) = converti(&mut finto(true), Peaks::Profilo).unwrap();
    assert!(doc.contains("MS:1000128") && !doc.contains("MS:1000035"));
    assert!(spettri.iter().all(|s| s.profile && s.mz.len() == 400));
    let e = converti(&mut finto(false), Peaks::Profilo).unwrap_err();
    assert_eq!(e.key, "err.conv.noProfile");
}

#[test]
fn lettori_mzml_picchi_file_keeps_the_scans_of_the_reader() {
    let (doc, spettri) = converti(&mut finto(true), Peaks::File).unwrap();
    assert!(doc.contains("MS:1000128") && !doc.contains("MS:1000035"));
    assert!(spettri.iter().all(|s| s.mz.len() == 400));
}
