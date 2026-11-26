//! The gap fill of the page opens each file without its scan table (`Index::open_lean`). A file that the recipe centroids gets the same centroids from
//! that index, on the decision kept in `Screen::centroid_files`, as from the whole index: that is what keeps the cells the app fills. A file the decision
//! says no to is read as it is. Synthetic data only: a unit-resolution profile series on a 0.1 Da grid.

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::lrscreen::{centroid_wanted, centroids_of, centroids_of_file};
use mzlab_feat::recipe::Recipe;

/// 40 MS1 scans, each a profile of three Gaussian peaks of width 0.35 Da on a 0.1 Da grid (labelled profile, so the screen centroids it).
fn profile_index(label_profile: bool) -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "lean".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(40).unwrap();
    for i in 0..40 {
        let grid: Vec<f64> = (0..300).map(|b| 200.0 + b as f64 * 0.1).collect();
        let y: Vec<f32> = grid
            .iter()
            .map(|&m| {
                [(210.0, 9.0e4), (221.0, 4.0e4), (228.0, 2.0e4)]
                    .iter()
                    .map(|&(c, a): &(f64, f64)| {
                        a * (-((m - c) * (m - c)) / (2.0 * 0.15 * 0.15)).exp()
                            * (1.0 + 0.01 * (i % 5) as f64)
                    })
                    .sum::<f64>() as f32
            })
            .collect();
        w.spectrum(&Spectrum {
            index: i,
            id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
            level: 1,
            rt: 0.05 * i as f64,
            polarity: 1,
            profile: label_profile,
            filter: String::new(),
            precursor: None,
            mz: grid,
            intensity: y,
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "lean.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

fn peaks(ix: &mut Index<Box<dyn mzlab_core::idxread::ReadAt>>) -> Vec<f64> {
    let mut out = Vec::new();
    for si in 0..ix.sections.len() {
        let n = ix.sections[si].n_peaks;
        let mut mz = Vec::new();
        ix.mz_into(si, 0, n, &mut mz).unwrap();
        out.extend(mz);
    }
    out
}

#[test]
fn a_lean_index_is_centroided_on_the_decision_as_the_whole_index_is() {
    let b = profile_index(true);
    let n = b.len() as u64;
    let mut r = Recipe::lr(0.1, 3e3, 7e3);
    let mut whole = Index::open(b.clone(), n).unwrap();
    assert!(
        centroid_wanted(&mut whole, &r),
        "the series is profile, not sparse counts"
    );
    let mut from_whole = centroids_of(&mut whole, &r).unwrap().expect("centroids");

    let mut lean = Index::open_lean(b.clone(), n).unwrap();
    // without the scan table the lean index cannot decide: it reads as not profile
    assert!(!centroid_wanted(&mut lean, &r));
    r.screen.centroid_files = vec![true];
    let mut from_lean = centroids_of_file(&mut lean, &r, 0)
        .unwrap()
        .expect("centroids from the lean index");
    assert_eq!(from_whole.fwhm, from_lean.fwhm);
    let (pw, pl) = (peaks(&mut from_whole.index), peaks(&mut from_lean.index));
    assert!(
        !pw.is_empty() && pw.len() < 40 * 300 / 10,
        "reduced to centroids: {}",
        pw.len()
    );
    assert_eq!(pw, pl);
}

#[test]
fn a_file_the_decision_says_no_to_is_not_centroided_and_one_without_a_decision_decides_itself() {
    let b = profile_index(true);
    let n = b.len() as u64;
    let mut r = Recipe::lr(0.1, 3e3, 7e3);
    r.screen.centroid_files = vec![false];
    let mut lean = Index::open_lean(b.clone(), n).unwrap();
    assert!(centroids_of_file(&mut lean, &r, 0).unwrap().is_none());
    // a file beyond the decisions (or a run without them) keeps the rule of the whole index
    r.screen.centroid_files.clear();
    let mut whole = Index::open(b, n).unwrap();
    assert!(centroids_of_file(&mut whole, &r, 0).unwrap().is_some());
}
