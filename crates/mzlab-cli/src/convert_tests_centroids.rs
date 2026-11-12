//! `convert --peaks mzlab` on a synthetic profile mzML with a doublet: the `--report` counts the centroids flagged merged and saturated, stderr names the
//! notice and the code stays 0; the second run, taken from the cache, reports the same counts.

use mzlab_core::json::parse;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Options};

use super::super::convert;
use crate::flow_run_tests::p;

/// `n` unit-resolution profile scans in a numpress mzML (0.1 Da grid, FWHM 0.7 Da) with single peaks and a 1:1 doublet 0.35 Da apart at m/z 300.
fn profile(n: usize) -> Vec<u8> {
    let peaks = [
        (290.0, 4000.0),
        (300.0, 3000.0),
        (300.35, 3000.0),
        (310.0, 2500.0),
    ];
    // numpress: a plain mzML is no input of `convert`, a numpress one is (the reader that writes it again as plain)
    let numpress = Options {
        numpress: true,
        ..Options::default()
    };
    let head = Header {
        run: "prof".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::with_options(Vec::new(), &head, numpress).unwrap();
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let mz: Vec<f64> = (0..400).map(|k| 280.0 + 0.1 * k as f64).collect();
        let y: Vec<f32> = mz
            .iter()
            .enumerate()
            .map(|(k, &m)| {
                let s: f64 = peaks
                    .iter()
                    .map(|&(c, h)| h * (-0.5 * ((m - c) / 0.3).powi(2)).exp())
                    .sum();
                let noise = (((i * 400 + k) as u64).wrapping_mul(2_654_435_761) >> 7) % 1000;
                ((s + 5.0 + noise as f64 * 0.006) * 1.37) as f32
            })
            .collect();
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 0.5 + 0.05 * i as f64,
            polarity: 1,
            profile: true,
            filter: "+ p ESI Q1MS [280.000-320.000]".into(),
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

#[test]
fn the_report_counts_the_merged_centroids_of_our_own_peak_picking() {
    let dir = crate::convert_fixtures::folder("merged");
    std::fs::create_dir_all(dir.join("in")).unwrap();
    std::fs::write(dir.join("in/p.mzML"), profile(30)).unwrap();
    let (src, out, report) = (dir.join("in"), dir.join("out"), dir.join("r.json"));
    for round in ["written", "cached"] {
        let o = convert(&[
            p(&src),
            "--out",
            p(&out),
            "--peaks",
            "mzlab",
            "--report",
            p(&report),
        ]);
        assert_eq!(o.code, 0, "{round}: {}", o.stderr);
        assert!(
            o.stderr.contains("30 centroids merged") && o.stderr.contains("warn.centroid.merged"),
            "{round}: {}",
            o.stderr
        );
        let r = parse(&std::fs::read(&report).unwrap()).unwrap();
        let c = r.arr()[0]
            .get("centroidi")
            .expect("the counts of our centroids");
        assert_eq!(c.get("merged").and_then(|v| v.f64()), Some(30.0), "{round}");
        assert_eq!(
            c.get("saturated").and_then(|v| v.f64()),
            Some(0.0),
            "{round}"
        );
    }
    // the file's own peaks: nothing of ours to count
    let o = convert(&[
        p(&src),
        "--out",
        p(&dir.join("file")),
        "--report",
        p(&report),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(!o.stderr.contains("warn.centroid.merged"), "{}", o.stderr);
    assert!(parse(&std::fs::read(&report).unwrap()).unwrap().arr()[0]
        .get("centroidi")
        .is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
