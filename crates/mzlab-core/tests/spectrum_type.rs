//! The spectrum-type term of a converted mzML on a fake reader that flags every scan as profile: sparse ion counts are written as centroid (MS:1000127) by
//! `auto`, a continuous trace stays profile (MS:1000128), `centroid` / `profile` are the user's word; the points never change and the file read back says
//! what was written.

use mzlab_core::error::{Error, Result};
use mzlab_core::mzml::{self, Chromatogram, Item, Spectrum};
use mzlab_core::readers::{ChromatogramEntry, Field, Format, Reader, Sample, ScanEntry};
use mzlab_core::readers_mzml::Conversion;
use mzlab_core::spectrum_type::{Basis, SpectrumType};

struct Fake {
    samples: Vec<Sample>,
    index: Vec<ScanEntry>,
    sparse: bool,
    flag: bool,
}

fn fake(sparse: bool, flag: bool) -> Fake {
    let n = 6;
    Fake {
        samples: vec![Sample {
            id: 0,
            name: "a".into(),
            scansioni: n,
            size: None,
        }],
        index: (0..n)
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
        sparse,
        flag,
    }
}

impl Reader for Fake {
    fn format(&self) -> Format {
        Format::Altro("fake")
    }
    fn samples(&self) -> &[Sample] {
        &self.samples
    }
    fn index(&mut self, _c: usize) -> Result<&[ScanEntry]> {
        Ok(&self.index)
    }
    fn scan(&mut self, _c: usize, k: usize) -> Result<Spectrum> {
        // isolated counts at irregular masses, or a gaussian trace every 0.1 Da
        let (mz, y): (Vec<f64>, Vec<f64>) = if self.sparse {
            (0..9)
                .map(|i| {
                    (
                        100.0
                            + f64::from(i) * 7.31
                            + (f64::from(i) * 1.7).sin() * 2.0
                            + k as f64 * 0.01,
                        1.0 + f64::from(i % 3),
                    )
                })
                .unzip()
        } else {
            (0..300)
                .map(|i| {
                    let m = 100.0 + f64::from(i) * 0.1;
                    (m, 1000.0 * (-((m - 115.0) / 0.35).powi(2)).exp())
                })
                .unzip()
        };
        Ok(Spectrum {
            index: k,
            level: 1,
            rt: k as f64,
            polarity: 1,
            profile: self.flag,
            mz,
            intensity: y.iter().map(|&v| v as f32).collect(),
            ..Spectrum::default()
        })
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

/// The file and the spectra read back from it.
fn convert(l: &mut Fake, spectrum: SpectrumType) -> (String, Vec<Spectrum>) {
    let conv = Conversion {
        spectrum,
        ..Conversion::default()
    };
    let bytes = conv.write(l, 0, Vec::new(), &mut |_, _| {}).unwrap();
    let mut v = Vec::new();
    mzml::parse(bytes.as_slice(), |i| {
        if let Item::Spectrum(s) = i {
            v.push(s);
        }
    })
    .unwrap();
    (String::from_utf8(bytes).unwrap(), v)
}

fn all_profile(v: &[Spectrum], want: bool) -> bool {
    !v.is_empty() && v.iter().all(|s| s.profile == want)
}

#[test]
fn sparse_counts_flagged_profile_are_written_as_centroid_by_auto() {
    let (doc, back) = convert(&mut fake(true, true), SpectrumType::Auto);
    assert!(doc.contains("MS:1000127") && !doc.contains("MS:1000128"));
    assert!(all_profile(&back, false), "the reader reads centroid back");
    // the points are the ones of the reader, and the decision is in the run
    assert!(back.iter().all(|s| s.mz.len() == 9));
    assert!(doc.contains("mzLab spectrum type"));
    assert!(doc.contains("written=centroid") && doc.contains("sparse points"));
}

#[test]
fn a_continuous_trace_stays_profile_and_the_file_is_the_one_written_before() {
    let (doc, back) = convert(&mut fake(false, true), SpectrumType::Auto);
    assert!(doc.contains("MS:1000128") && !doc.contains("MS:1000127"));
    assert!(all_profile(&back, true));
    // nothing changed in this case: no extra line in the run, the bytes are those of a reader that is never asked
    assert!(!doc.contains("mzLab spectrum type"));
}

#[test]
fn centroids_the_reader_reports_stay_centroids() {
    let (doc, back) = convert(&mut fake(false, false), SpectrumType::Auto);
    assert!(all_profile(&back, false) && !doc.contains("mzLab spectrum type"));
}

#[test]
fn the_users_word_wins_in_both_directions_and_is_recorded() {
    let (doc, back) = convert(&mut fake(false, true), SpectrumType::Centroid);
    assert!(all_profile(&back, false) && doc.contains("spectrum=centroid; written=centroid"));
    assert_eq!(back[0].mz.len(), 300, "the points do not change");
    let (doc, back) = convert(&mut fake(true, true), SpectrumType::Profile);
    assert!(all_profile(&back, true) && doc.contains("spectrum=profile; written=profile"));
}

#[test]
fn the_choice_is_taken_once_and_told() {
    let mut l = fake(true, true);
    let conv = Conversion::default();
    let c = conv.type_choice(&mut l, 0).unwrap();
    assert_eq!((c.basis, c.force), (Basis::Sparse, Some(false)));
    assert!(c.share.unwrap() < 0.5 && c.scans == 6);
    assert!(c
        .text(true)
        .starts_with("spectrum=auto; written=centroid; basis=sparse"));
    let c = conv.type_choice(&mut fake(false, true), 0).unwrap();
    assert_eq!(
        (c.basis, c.force, c.recorded()),
        (Basis::Continuous, None, false)
    );
}
