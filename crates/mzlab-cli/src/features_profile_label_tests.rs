//! Does the label check call a file mislabelled only when it is? Profile files that are genuine (a fine grid, a coarse grid with narrow peaks, a
//! zero-suppressed grid) must still be centroided with no label notice, a centroid file labelled centroid has no notice, and centroids under a profile
//! label have the notice with the numbers the verdict came from.

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Options};
use mzlab_feat::lrscreen::{centroid_wanted, label_check, LabelCheck, Switch};
use mzlab_feat::recipe::Recipe;
use mzlab_ops::run::feat::lr_note::note_of;

/// What a file holds: `step` Da between the points of the grid (`0.0`: one centroid per ion), peaks of FWHM `fwhm` Da, `suppressed` drops the points under
/// 1 % of the scan maximum, `label` is the flag the file carries.
#[derive(Clone, Copy)]
struct Kind {
    step: f64,
    fwhm: f64,
    suppressed: bool,
    label: bool,
}

/// 40 MS1 scans of 12 ions (nominal masses plus a mass defect, 9.7 Da apart) with a chromatographic peak at the middle.
fn mzml(k: Kind) -> Vec<u8> {
    let head = Header {
        run: "q1".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::with_options(Vec::new(), &head, Options::default()).unwrap();
    w.open_spectra(40).unwrap();
    for i in 0..40usize {
        let h = 2e5 * (-0.5 * ((i as f64 - 20.0) / 4.0).powi(2)).exp() + 3e3;
        let ions: Vec<(f64, f64)> = (0..12)
            .map(|j| {
                (
                    101.2 + 9.7 * j as f64 + 0.01 * i as f64,
                    h / (1.0 + j as f64),
                )
            })
            .collect();
        let (mz, y): (Vec<f64>, Vec<f32>) = if k.step == 0.0 {
            ions.iter().map(|&(m, a)| (m, a as f32)).unzip()
        } else {
            let sigma = k.fwhm / 2.3548;
            let n = ((120.0 - 100.0) / k.step) as usize * 6;
            let grid: Vec<f64> = (0..n).map(|g| 100.0 + k.step * g as f64).collect();
            let y: Vec<f64> = grid
                .iter()
                .map(|&m| {
                    ions.iter()
                        .map(|&(c, a)| a * (-0.5 * ((m - c) / sigma).powi(2)).exp())
                        .sum::<f64>()
                        + 20.0
                })
                .collect();
            let top = y.iter().cloned().fold(0.0, f64::max);
            grid.iter()
                .zip(&y)
                .filter(|&(_, &v)| !k.suppressed || v > 0.01 * top)
                .map(|(&m, &v)| (m, v as f32))
                .unzip()
        };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 0.5 + 0.02 * i as f64,
            polarity: 1,
            profile: k.label,
            filter: "+ p ESI Q1MS [100.000-220.000]".into(),
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

/// What the engine decides for a file of this kind with `centroid` `sw`: the check, whether the file is centroided, and the keys of its notices.
struct Verdict {
    check: Option<LabelCheck>,
    centroided: bool,
    notices: Vec<(&'static str, Vec<(&'static str, usize)>)>,
}

fn verdict(k: Kind, sw: Switch) -> Verdict {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mzlab_label_{n}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (f, idx) = (dir.join("q1.mzML"), dir.join("0.mzidx"));
    std::fs::write(&f, mzml(k)).unwrap();
    crate::build_index_of(&f, "", &idx, mzlab_core::budget_auto::native_index_budget()).unwrap();
    let mut index = crate::open(&idx).unwrap();
    let mut recipe = Recipe::for_index(&mut index).unwrap();
    recipe.screen.centroid = sw;
    let v = Verdict {
        check: label_check(&mut index),
        centroided: centroid_wanted(&mut index, &recipe),
        notices: note_of(&recipe, &mut index)
            .notices
            .iter()
            .map(|n| (n.key, n.params.clone()))
            .collect(),
    };
    let _ = std::fs::remove_dir_all(&dir);
    v
}

const FINE: Kind = Kind {
    step: 0.1,
    fwhm: 0.7,
    suppressed: false,
    label: true,
};
const COARSE_NARROW: Kind = Kind {
    step: 0.5,
    fwhm: 0.4,
    suppressed: false,
    label: true,
};
const CENTROIDS: Kind = Kind {
    step: 0.0,
    fwhm: 0.0,
    suppressed: false,
    label: false,
};
const MISLABELLED: Kind = Kind {
    step: 0.0,
    fwhm: 0.0,
    suppressed: false,
    label: true,
};

fn keys(v: &Verdict) -> Vec<&'static str> {
    v.notices.iter().map(|n| n.0).collect()
}

/// Genuine profile files, whatever their grid, are centroided before detection and never get the label notice: the points are in runs of a profile peak.
#[test]
fn genuine_profile_files_are_centroided_with_no_label_notice() {
    for (what, k) in [
        ("0.1 Da grid", FINE),
        ("0.5 Da grid, narrow peaks (FWHM 0.4)", COARSE_NARROW),
        (
            "0.2 Da grid, zero-suppressed",
            Kind {
                step: 0.2,
                suppressed: true,
                ..FINE
            },
        ),
        (
            "0.5 Da grid, FWHM 0.4, zero-suppressed",
            Kind {
                suppressed: true,
                ..COARSE_NARROW
            },
        ),
    ] {
        let v = verdict(k, Switch::Auto);
        let c = v
            .check
            .clone()
            .unwrap_or_else(|| panic!("{what}: labelled profile"));
        assert!(!c.sparse && v.centroided, "{what}: {c:?}");
        assert!(
            c.runs_pct >= 50,
            "{what}: {} % of the points in runs",
            c.runs_pct
        );
        assert_eq!(keys(&v), vec!["note.features.centroided"], "{what}");
    }
}

/// A file labelled centroid is never questioned: no check, no centroiding, no notice.
#[test]
fn a_centroid_file_labelled_centroid_has_no_check_and_no_notice() {
    let v = verdict(CENTROIDS, Switch::Auto);
    assert!(v.check.is_none() && !v.centroided && v.notices.is_empty());
}

/// Centroids under a profile label: the verdict and its numbers (every point is isolated: no run, no fine spacing), the cut-offs of the detector in the
/// notice, and the user's word (`centroid` on) still centroids the file.
#[test]
fn centroids_labelled_profile_are_reported_with_the_numbers_and_the_cut_offs() {
    let v = verdict(MISLABELLED, Switch::Auto);
    let c = v.check.clone().unwrap();
    assert!(c.sparse && !v.centroided);
    assert_eq!((c.profile, c.scans, c.probed), (40, 40, 40));
    assert_eq!(
        (c.points, c.spacings, c.runs_pct, c.fine_pct),
        (40 * 12, 40 * 11, 0, 0)
    );
    assert_eq!(keys(&v), vec!["warn.features.profileLabel"]);
    let p = &v.notices[0].1;
    let get = |k: &str| p.iter().find(|(q, _)| *q == k).map(|(_, v)| *v);
    assert_eq!(
        [
            get("runs_cut"),
            get("fine_cut"),
            get("run_points"),
            get("run_step"),
            get("fine_step")
        ],
        [Some(50), Some(2), Some(4), Some(600), Some(50)]
    );
    assert_eq!(
        (get("points"), get("probed"), get("spacings")),
        (Some(480), Some(40), Some(440))
    );
    let r = mzlab_feat::lrscreen::label_reason(&c);
    for want in [
        "480 points in 40 scans probed",
        "0 % are in runs",
        "sparse below 50 %",
        "under 50 mDa",
        "centroid=on forces it",
    ] {
        assert!(r.contains(want), "{want}: {r}");
    }
    let on = verdict(MISLABELLED, Switch::On);
    assert!(on.centroided && keys(&on).iter().all(|k| *k != "warn.features.profileLabel"));
}

/// The limit of the detector, which is the one of `raw info`: zero-suppressed profile peaks of three points (a 0.5 Da grid, FWHM 0.7 Da) have no run of
/// four points, so they read as sparse. The notice gives the numbers (a third of the points in runs, no fine spacing) and `--centroid on` decides.
#[test]
fn zero_suppressed_three_point_peaks_read_as_sparse_and_the_numbers_say_so() {
    let k = Kind {
        step: 0.5,
        fwhm: 0.7,
        suppressed: true,
        label: true,
    };
    let v = verdict(k, Switch::Auto);
    let c = v.check.clone().unwrap();
    assert!(c.sparse && !v.centroided, "{c:?}");
    assert!((20..50).contains(&c.runs_pct) && c.fine_pct == 0, "{c:?}");
    assert!(verdict(k, Switch::On).centroided);
}
