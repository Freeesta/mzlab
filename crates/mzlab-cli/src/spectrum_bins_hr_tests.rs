//! `spectrum get --bin` on a high-resolution PROFILE file (single-precision index section, points 0.004 Da apart): the bins, not `err.scan.none`.
//! A high-resolution centroid file keeps its own refusal (`err.spectrum.rebin.hr`, in `spectrum_bins_tests.rs`).

use mzlab_core::json::{parse, Value};

use crate::extract::tests::{dir, run};

const STEP: f64 = 0.004;
const POINTS: usize = 2500;

fn grid(i: usize) -> f64 {
    300.0 + STEP * i as f64
}

/// Baseline of 10 and a Gaussian of sigma 0.01 Da at 305.0702 (it straddles the bin edge 305.05), invented numbers.
fn intensity(i: usize) -> f32 {
    let d = (grid(i) - 305.0702) / 0.01;
    (10.0 + 5.0e4 * (-0.5 * d * d).exp()) as f32
}

fn hr_profile(tag: &str) -> (std::path::PathBuf, String) {
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let d = dir(tag);
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(3).unwrap();
    for k in 0..3 {
        let s = mzlab_core::mzml::Spectrum {
            index: k,
            id: format!("scan={}", k + 1),
            level: 1,
            rt: k as f64,
            polarity: 1,
            profile: true,
            filter: "FTMS + p ESI Full ms [300.0000-310.0000]".into(),
            mz: (0..POINTS).map(grid).collect(),
            intensity: (0..POINTS).map(intensity).collect(),
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let f = d.join("hrp.mzML");
    std::fs::write(&f, w.finish().unwrap()).unwrap();
    (d, f.to_string_lossy().into_owned())
}

fn column(v: &Value, key: &str) -> Vec<f64> {
    v.get("bins")
        .unwrap()
        .arr()
        .iter()
        .map(|b| b.get(key).and_then(Value::f64).unwrap())
        .collect()
}

#[test]
fn tenths_of_an_hr_profile_file_sum_the_points_of_each_bin() {
    let (d, f) = hr_profile("bins_hrp");
    let o = run(&["spectrum", "get", &f, "--rt", "1", "--bin", "0.1"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&o.stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("spectrum-bins/1"));
    let (bin, counts, points) = (
        column(&v, "bin"),
        column(&v, "counts"),
        column(&v, "points"),
    );
    // the same sum, made here from the file's own points: [b - 0.05, b + 0.05) (no grid point is on an edge; the two end bins are partial)
    for (k, b) in bin.iter().enumerate() {
        let held: Vec<usize> = (0..POINTS)
            .filter(|&i| (grid(i) * 10.0).round() / 10.0 == *b)
            .collect();
        let want: f64 = held.iter().map(|&i| f64::from(intensity(i))).sum();
        assert_eq!(points[k], held.len() as f64, "bin {b}");
        assert!(
            (counts[k] - want).abs() < 0.5,
            "bin {b}: {} vs {want}",
            counts[k]
        );
    }
    // the peak at 305.0702 is cut by the edge 305.05: both bins carry it
    let at = |m: f64| bin.iter().position(|b| (*b - m).abs() < 1e-9).unwrap();
    assert!(
        counts[at(305.1)] > 1.0e5 && counts[at(305.0)] > 5.0e3,
        "{counts:?}"
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn nominal_bins_of_an_hr_profile_file_are_answered_too() {
    let (d, f) = hr_profile("bins_hrp_nom");
    let o = run(&["spectrum", "get", &f, "--rt", "0,2", "--bin", "nominal"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&o.stdout).unwrap();
    let (bin, counts, points) = (
        column(&v, "bin"),
        column(&v, "counts"),
        column(&v, "points"),
    );
    assert_eq!(bin.first(), Some(&300.0));
    // `points` counts the grid cells of a bin (250 of 0.004 Da in a nominal bin); `counts` is the mean spectrum of the window summed per bin (three equal scans: one scan's sum)
    assert_eq!(points[1], 250.0, "{points:?}");
    let one: f64 = (0..POINTS)
        .filter(|&i| (grid(i)).round() == bin[1])
        .map(|i| f64::from(intensity(i)))
        .sum();
    assert!((counts[1] - one).abs() < 1.0, "{} vs {one}", counts[1]);
    assert!(o.stderr.contains("note: bins nominal"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(d);
}
