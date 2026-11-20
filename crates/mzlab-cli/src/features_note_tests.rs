//! Tests of `features_note`: the recipe line for unit-resolution files and the profile warning.

use super::*;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Options};

/// A unit-resolution mzML of `n` MS1 scans: one ion at 152.2 eluting at the middle, as profile points (0.1 Da grid, FWHM 0.7 Da) or as one centroid.
fn mzml(profile: bool, n: usize) -> Vec<u8> {
    let head = Header {
        run: "q1".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::with_options(Vec::new(), &head, Options::default()).unwrap();
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.5 + 0.02 * i as f64;
        let h = 2e5 * (-0.5 * ((i as f64 - n as f64 / 2.0) / 4.0).powi(2)).exp();
        let (mz, y): (Vec<f64>, Vec<f32>) = if profile {
            let mz: Vec<f64> = (0..300).map(|k| 120.0 + 0.1 * k as f64).collect();
            let y = mz
                .iter()
                .map(|&m| (h * (-0.5 * ((m - 152.2) / 0.3).powi(2)).exp() + 20.0) as f32)
                .collect();
            (mz, y)
        } else {
            (vec![152.2], vec![(h + 20.0) as f32])
        };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile,
            filter: "+ p ESI Q1MS [120.000-150.000]".into(),
            precursor: None,
            mz,
            intensity: y,
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn lines_of(profile: bool) -> Vec<String> {
    let d = std::env::temp_dir().join(format!("mzlab_fnote_{profile}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let (f, idx) = (d.join("q1.mzML"), d.join("q1.mzidx"));
    std::fs::write(&f, mzml(profile, 40)).unwrap();
    crate::build_index_of(&f, "", &idx, mzlab_core::budget_auto::native_index_budget()).unwrap();
    let mut index = crate::open(&idx).unwrap();
    let recipe = Recipe::for_index(&mut index).unwrap();
    let out = lines(&recipe, &index, "q1.mzML");
    let _ = std::fs::remove_dir_all(&d);
    out
}

#[test]
fn unit_resolution_profile_data_get_the_recipe_line_and_the_warning() {
    let l = lines_of(true);
    assert_eq!(l.len(), 2, "{l:?}");
    assert!(
        l[0].starts_with("recipe q1.mzML: unit resolution (1 Da channels centred on n"),
        "{}",
        l[0]
    );
    assert!(
        l[0].ends_with("40 of 40 MS1 scans in profile mode"),
        "{}",
        l[0]
    );
    assert!(
        l[1].starts_with("warning: q1.mzML: 40 of 40 MS1 scans are profile mode"),
        "{}",
        l[1]
    );
    assert!(l[1].ends_with("(warn.features.profileUnit)"), "{}", l[1]);
}

#[test]
fn centroided_unit_resolution_data_get_the_recipe_line_only() {
    let l = lines_of(false);
    assert_eq!(l.len(), 1, "{l:?}");
    assert!(
        l[0].ends_with("0 of 40 MS1 scans in profile mode"),
        "{}",
        l[0]
    );
}
