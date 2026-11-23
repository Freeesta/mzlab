//! The spectrum-type term of a converted mzML on a fake reader that flags every scan as profile: sparse ion counts are written as centroid (MS:1000127) by
//! `auto`, a continuous trace stays profile (MS:1000128), `centroid` / `profile` are the user's word; the points never change and the file read back says
//! what was written.

use mzlab_core::error::{Error, Result};
use mzlab_core::mzml::{self, Chromatogram, Item, Spectrum};
use mzlab_core::readers::{ChromatogramEntry, Field, Format, Reader, Sample, ScanEntry};
use mzlab_core::readers_mzml::Conversion;
use mzlab_core::spectrum_type::{Basis, SpectrumType};

/// What the scans of a fake sample hold (the reader flags the profile ones as `flag` says).
#[derive(Clone, Copy)]
enum Shape {
    /// Isolated counts at irregular masses.
    Counts,
    /// A gaussian trace every 0.1 Da.
    Trace,
    /// `n` points per peak every 0.003 Da, the peaks 7.3 Da apart (zero-suppressed high-resolution profile).
    Peaks(usize),
    /// A Q1 trace every 0.5 Da.
    Q1,
    /// Three points on a fine grid.
    Few,
    /// No points.
    Empty,
}

struct Fake {
    samples: Vec<Sample>,
    index: Vec<ScanEntry>,
    /// Shape and MS level of each scan.
    shapes: Vec<(Shape, u8)>,
    flag: bool,
}

fn many(shapes: Vec<(Shape, u8)>, flag: bool) -> Fake {
    let n = shapes.len();
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
                livello: shapes[k].1,
                polarita: 1,
                tic: 0.0,
                base_peak: None,
                filtro: String::new(),
                precursore: None,
                position: None,
            })
            .collect(),
        shapes,
        flag,
    }
}

/// Six MS1 scans of one shape.
fn fake_of(shape: Shape, flag: bool) -> Fake {
    many(vec![(shape, 1); 6], flag)
}

fn fake(sparse: bool, flag: bool) -> Fake {
    fake_of(if sparse { Shape::Counts } else { Shape::Trace }, flag)
}

fn points(shape: Shape, k: usize) -> Vec<f64> {
    match shape {
        Shape::Counts => (0..9)
            .map(|i| {
                100.0 + f64::from(i) * 7.31 + (f64::from(i) * 1.7).sin() * 2.0 + k as f64 * 0.01
            })
            .collect(),
        Shape::Trace => (0..300).map(|i| 100.0 + f64::from(i) * 0.1).collect(),
        Shape::Peaks(n) => (0..10)
            .flat_map(|p| (0..n).map(move |i| 500.0 + f64::from(p) * 7.3 + i as f64 * 0.003))
            .collect(),
        Shape::Q1 => (0..200).map(|i| 100.0 + f64::from(i) * 0.5).collect(),
        Shape::Few => vec![300.0, 300.004, 300.008],
        Shape::Empty => Vec::new(),
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
        let (shape, level) = self.shapes[k];
        let mz = points(shape, k);
        let y: Vec<f32> = mz
            .iter()
            .map(|&m| match shape {
                Shape::Trace => (1000.0 * (-((m - 115.0) / 0.35).powi(2)).exp()) as f32,
                _ => 1.0 + (m * 10.0) as usize as f32 % 3.0,
            })
            .collect();
        Ok(Spectrum {
            index: k,
            level,
            rt: k as f64,
            polarity: 1,
            profile: self.flag,
            mz,
            intensity: y,
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
    assert_eq!((c.basis, c.force), (Basis::Sparse, None));
    // only the level that was probed and found sparse changes; a level never seen keeps what its reader says
    assert!(!c.keeps(true, 1) && c.keeps(true, 2));
    assert!(c.share.unwrap() < 0.5 && c.scans == 6);
    assert!(c
        .text()
        .starts_with("spectrum=auto; written=centroid for ms_level 1"));
    let c = conv.type_choice(&mut fake(false, true), 0).unwrap();
    assert_eq!(
        (c.basis, c.force, c.recorded()),
        (Basis::Continuous, None, false)
    );
}

/// The decision of `auto` on a sample of the shapes, all flagged profile.
fn auto(shapes: Vec<(Shape, u8)>) -> mzlab_core::spectrum_type::TypeChoice {
    Conversion::default()
        .type_choice(&mut many(shapes, true), 0)
        .unwrap()
}

#[test]
fn zero_suppressed_high_resolution_peaks_stay_profile_at_any_width() {
    // two to five points per peak on a fine grid: no run of four, but never ion counts
    for n in 2..=5 {
        let c = auto(vec![(Shape::Peaks(n), 1); 6]);
        assert_eq!((c.basis, c.sparse), (Basis::Continuous, 0), "{n} points");
        let (doc, back) = convert(&mut fake_of(Shape::Peaks(n), true), SpectrumType::Auto);
        assert!(
            doc.contains("MS:1000128") && all_profile(&back, true),
            "{n}"
        );
    }
}

#[test]
fn a_profile_scan_with_few_points_stays_profile() {
    let c = auto(vec![(Shape::Few, 1); 6]);
    assert_eq!((c.basis, c.sparse), (Basis::Continuous, 0));
}

#[test]
fn a_q1_trace_every_half_dalton_stays_profile() {
    let c = auto(vec![(Shape::Q1, 1); 6]);
    assert_eq!((c.basis, c.sparse), (Basis::Continuous, 0));
}

#[test]
fn a_profile_ms1_among_sparse_product_scans_stays_profile() {
    // MS1 trace, MS2 isolated counts: the MS2 scans become centroid, the MS1 scans keep their profile, the note names the level
    let mut shapes = Vec::new();
    for _ in 0..3 {
        shapes.push((Shape::Trace, 1));
        shapes.push((Shape::Counts, 2));
        shapes.push((Shape::Counts, 2));
    }
    let c = auto(shapes.clone());
    assert_eq!(c.basis, Basis::Sparse);
    assert!(c.keeps(true, 1) && !c.keeps(true, 2));
    assert!(c.text().contains("centroid for ms_level 2"));
    let (_, back) = convert(&mut many(shapes, true), SpectrumType::Auto);
    let flags: Vec<bool> = back.iter().map(|s| s.profile).collect();
    assert_eq!(
        flags,
        [true, false, false, true, false, false, true, false, false]
    );
}

#[test]
fn nothing_to_look_at_decides_nothing() {
    let c = auto(Vec::new());
    assert_eq!((c.basis, c.sparse, c.recorded()), (Basis::Empty, 0, false));
    let c = auto(vec![(Shape::Empty, 1); 4]);
    assert_eq!((c.basis, c.sparse), (Basis::Empty, 0));
}

#[test]
fn other_peaks_are_ours_and_the_note_says_so() {
    let conv = Conversion {
        peaks: mzlab_core::readers_mzml::Peaks::Mzlab,
        ..Conversion::default()
    };
    let c = conv.type_choice(&mut fake(false, true), 0).unwrap();
    assert_eq!((c.basis, c.recorded()), (Basis::Peaks, false));
    assert_eq!(
        c.text(),
        "spectrum=auto; written=centroid; basis=peaks mzlab"
    );
}

#[test]
fn a_mixed_file_is_not_said_to_be_all_profile() {
    let c = auto(vec![(Shape::Trace, 1); 3]);
    assert!(c.text().contains("written=as the reader"));
}
