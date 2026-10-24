//! The wavelet peak model (`features.detect`, algorithm `centwave`) on a synthetic centroided run: an ion with two overlapping Gaussian peaks 7 s apart is
//! one run of scans for the default model and two peaks for CentWave; an isolated peak keeps its apex and its area; flat noise gives nothing; two runs give
//! the same table. The default model stays as it was. Synthetic data only.

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::centwave::{CentWave, Peak};
use mzlab_feat::table::FileFeatures;
use mzlab_feat::trace::{detect, Params};

/// Seconds between two scans.
const DT: f64 = 1.2;
const SCANS: usize = 300;
const MZ: f64 = 300.1234;
const SIGMA: f64 = 2.4;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The `.mzidx` of a run with one ion made of Gaussians `(centre in seconds, amplitude)` of width `SIGMA`, over noise peaks far below the floor.
fn index_of(peaks: &[(f64, f64)]) -> Vec<u8> {
    index_with(peaks, SIGMA, 0.0)
}

/// As `index_of`, with the width of the Gaussians and a multiplicative jitter of the intensities (a fraction of them).
fn index_with(peaks: &[(f64, f64)], sigma: f64, jitter: f64) -> Vec<u8> {
    let mut rng = Lcg(5);
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "cw".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(SCANS).unwrap();
    for i in 0..SCANS {
        let rt = 60.0 + DT * i as f64;
        let mut p: Vec<(f64, f32)> = (0..30)
            .map(|_| {
                (
                    100.0 + 400.0 * rng.next(),
                    (2e3 * (1.0 + rng.next())) as f32,
                )
            })
            .collect();
        let v: f64 = peaks
            .iter()
            .map(|&(c, a)| a * (-0.5 * ((rt - c) / sigma).powi(2)).exp())
            .sum::<f64>()
            * (1.0 + jitter * (2.0 * rng.next() - 1.0));
        if v > 100.0 {
            p.push((MZ * (1.0 + 1e-6 * (rng.next() - 0.5)), v as f32));
        }
        p.sort_by(|a, b| a.0.total_cmp(&b.0));
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: rt / 60.0,
            polarity: 1,
            profile: false,
            filter: "FTMS + p ESI Full ms [100.0000-600.0000]".into(),
            precursor: None,
            mz: p.iter().map(|x| x.0).collect(),
            intensity: p.iter().map(|x| x.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "cw.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

fn run(peaks: &[(f64, f64)], peak: Peak) -> FileFeatures {
    let idx = index_of(peaks);
    let mut index = Index::open(idx.clone(), idx.len() as u64).unwrap();
    let p = Params {
        peak,
        ..Params::default()
    };
    detect(&mut index, &p).unwrap()
}

fn wavelet() -> Peak {
    Peak::CentWave(CentWave::default())
}

/// RT (minutes) of the rows near `MZ`, ascending.
fn apexes(f: &FileFeatures) -> Vec<f64> {
    let mut v: Vec<f64> = (0..f.len())
        .filter(|&i| (f.mz[i] - MZ).abs() < 0.01)
        .map(|i| f64::from(f.rt[i]))
        .collect();
    v.sort_by(f64::total_cmp);
    v
}

#[test]
fn default_centwave_width_is_five_to_thirty_seconds_and_configurable() {
    let cw_default = CentWave::default();
    assert_eq!(cw_default.width_s, (5.0, 30.0));
    let cw_custom = CentWave::default().with_width_s((5.0, 60.0));
    assert_eq!(cw_custom.width_s, (5.0, 60.0));

    // with default parameters, explicit and default give identical output
    let two = [(150.0, 1.0e6), (157.0, 8.0e5)];
    let def = run(&two, wavelet());
    let explicit = run(
        &two,
        Peak::CentWave(CentWave {
            width_s: (5.0, 30.0),
            ..CentWave::default()
        }),
    );
    assert_eq!(def.mz, explicit.mz);
    assert_eq!(def.rt, explicit.rt);
    assert_eq!(def.area, explicit.area);
    assert_eq!(def.height, explicit.height);
}

#[test]
fn two_overlapping_peaks_are_two_for_centwave_and_one_run_for_the_default_model() {
    let two = [(150.0, 1.0e6), (157.0, 8.0e5)];
    let plain = apexes(&run(&two, Peak::Minima));
    assert_eq!(plain.len(), 1, "{plain:?}");
    let cw = apexes(&run(&two, wavelet()));
    assert_eq!(cw.len(), 2, "{cw:?}");
    assert!((cw[0] * 60.0 - 150.0).abs() < 2.0 * DT, "{cw:?}");
    assert!((cw[1] * 60.0 - 157.0).abs() < 2.0 * DT, "{cw:?}");
}

#[test]
fn an_isolated_peak_keeps_its_apex_and_its_area() {
    let f = run(&[(150.0, 1.0e6)], wavelet());
    let rows: Vec<usize> = (0..f.len())
        .filter(|&i| (f.mz[i] - MZ).abs() < 0.01)
        .collect();
    assert_eq!(rows.len(), 1, "{:?}", apexes(&f));
    let i = rows[0];
    assert!((f64::from(f.rt[i]) * 60.0 - 150.0).abs() < DT);
    // a Gaussian of height H and width SIGMA holds H * SIGMA * sqrt(2 pi) of intensity times seconds, the area is in those units
    let whole = 1.0e6 * SIGMA * (2.0 * std::f64::consts::PI).sqrt();
    let seen = f64::from(f.area[i] as f32);
    assert!(
        seen > 0.5 * whole && seen < 1.5 * whole,
        "{seen} vs {whole}"
    );
}

/// A noisy peak wider than the narrowest wavelet in a series that holds little else: the flanks that the narrow wavelet leaves out are not noise.
#[test]
fn a_noisy_wide_peak_in_a_short_series_is_one_peak_with_its_height() {
    let idx = index_with(&[(150.0, 1.0e6)], 6.0, 0.08);
    let mut index = Index::open(idx.clone(), idx.len() as u64).unwrap();
    let p = Params {
        peak: wavelet(),
        ..Params::default()
    };
    let f = detect(&mut index, &p).unwrap();
    let rows: Vec<usize> = (0..f.len())
        .filter(|&i| (f.mz[i] - MZ).abs() < 0.01)
        .collect();
    assert_eq!(rows.len(), 1, "{:?}", apexes(&f));
    let h = f64::from(f.height[rows[0]]);
    assert!(h > 0.9e6 && h < 1.1e6, "{h}");
}

#[test]
fn flat_noise_and_a_peak_below_the_noise_floor_give_nothing() {
    assert!(apexes(&run(&[], wavelet())).is_empty());
    assert!(apexes(&run(&[(150.0, 1.5e4)], wavelet())).is_empty());
}

#[test]
fn two_runs_give_the_same_table() {
    let two = [(150.0, 1.0e6), (157.0, 8.0e5)];
    let (a, b) = (run(&two, wavelet()), run(&two, wavelet()));
    assert_eq!(a.mz, b.mz);
    assert_eq!(a.rt, b.rt);
    assert_eq!(a.area, b.area);
}
