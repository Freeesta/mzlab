//! `--mz-offset` of `trace xic` and `features` on synthetic unit-resolution centroid files: five samples whose m/z axis is moved by +0.24 +- 0.05 Da
//! (the offset of the 3200 QTRAP). The estimate recovers each shift, `auto` gives the numbers of the fixed offset it measured, a blank has no ion to
//! measure on, a split peak is "wide", and `features` puts the same ions in one row across the samples.

use std::path::PathBuf;

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Options};

use crate::{execute, Outcome};

/// The shifts of the five samples, Da.
const SHIFTS: [f64; 5] = [0.19, 0.22, 0.24, 0.26, 0.29];
const SCANS: usize = 60;

/// What the ions of one file do.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    /// Every ion at its true m/z plus the shift.
    Plain,
    /// No ion at all, only the baseline.
    Blank,
    /// Every ion is two peaks 0.7 Da apart whose chromatographic apexes are 4 scans apart: the centroid walks from one to the other.
    Split,
}

/// The true m/z of the ions and the scan of their apex.
fn ions() -> Vec<(f64, usize)> {
    (0..8).map(|k| (130.2 + 9.7 * k as f64, 8 + 6 * k)).collect()
}

fn mzml(shift: f64, shape: Shape) -> Vec<u8> {
    let head = Header {
        run: "s".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::with_options(Vec::new(), &head, Options::default()).unwrap();
    w.open_spectra(SCANS).unwrap();
    for i in 0..SCANS {
        let (mz, y): (Vec<f64>, Vec<f32>) = if shape == Shape::Blank {
            (vec![111.0, 222.0, 333.0], vec![20.0, 30.0, 25.0])
        } else {
            // (m/z, apex scan) of each peak of each ion
            let peaks = ions().into_iter().flat_map(|(mz, apex)| {
                if shape == Shape::Split {
                    vec![(mz - 0.35, apex), (mz + 0.35, apex + 4)]
                } else {
                    vec![(mz, apex)]
                }
            });
            peaks
                .map(|(mz, apex)| {
                    let x = (i as f64 - apex as f64) / 2.0;
                    (mz + shift, (2e5 * (-0.5 * x * x).exp() + 20.0) as f32)
                })
                .unzip()
        };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 0.5 + 0.02 * i as f64,
            polarity: 1,
            profile: false,
            filter: "+ c ESI Q1MS [100.000-400.000]".into(),
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

/// A folder with one file per shift (`s0.mzML`..) in `shape`.
fn series(tag: &str, shape: Shape) -> (PathBuf, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("mzlab_mzoff_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let files = SHIFTS
        .iter()
        .enumerate()
        .map(|(k, d)| {
            let f = dir.join(format!("s{k}.mzML"));
            std::fs::write(&f, mzml(*d, shape)).unwrap();
            f.display().to_string()
        })
        .collect();
    (dir, files)
}

fn cli(args: &[&str]) -> Outcome {
    execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn text(o: &Outcome) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// The number after `"key":` in `json` (the first one).
fn number(json: &str, key: &str) -> f64 {
    let at = json.find(&format!("\"{key}\":")).unwrap() + key.len() + 3;
    json[at..]
        .split(|c| c == ',' || c == '}')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

/// `json` without its `"mz_offset":{..}` evidence object.
fn without_evidence(json: &str) -> String {
    let at = json.find(",\"mz_offset\":{").unwrap();
    let end = at + json[at..].find('}').unwrap() + 1;
    format!("{}{}", &json[..at], &json[end..])
}

/// The `"points":[..]` of a trace.
fn points(json: &str) -> &str {
    let at = json.find("\"points\":").unwrap();
    &json[at..at + json[at..].find("]]").unwrap() + 2]
}

#[test]
fn the_estimate_recovers_the_shift_of_each_sample() {
    let (dir, files) = series("rec", Shape::Plain);
    for (f, d) in files.iter().zip(SHIFTS) {
        let o = cli(&[
            "trace", "xic", f, "--mz", "130.2", "--da", "0.3", "--mz-offset", "auto:130.2", "--dry-run",
        ]);
        assert_eq!(o.code, 0, "{}", o.stderr);
        let j = text(&o);
        assert!(j.contains("\"status\":\"ok\""), "{j}");
        let got = number(&j, "mz_offset_da");
        assert!((got - d).abs() <= 0.02, "{f}: {got} against {d}");
        assert!(o.stderr.contains("mz offset"), "{}", o.stderr);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_blank_gives_not_found_offset_zero_and_a_warning_with_exit_0() {
    let (dir, files) = series("blank", Shape::Blank);
    let o = cli(&[
        "trace", "xic", &files[0], "--mz", "130.2", "--da", "0.3", "--mz-offset", "auto:130.2",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stderr.contains("warn.mz.offset.notFound"), "{}", o.stderr);
    let plain = cli(&["trace", "xic", &files[0], "--mz", "130.2", "--da", "0.3"]);
    assert_eq!(points(&text(&o)), points(&text(&plain)));
    let j = text(&cli(&[
        "trace", "xic", &files[0], "--mz", "130.2", "--da", "0.3", "--mz-offset", "auto:130.2", "--dry-run",
    ]));
    assert!(j.contains("\"status\":\"not_found\""), "{j}");
    assert_eq!(number(&j, "mz_offset_da"), 0.0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_split_peak_is_wide_and_still_applied() {
    let (dir, files) = series("wide", Shape::Split);
    let o = cli(&[
        "trace", "xic", &files[2], "--mz", "130.2", "--da", "0.3", "--mz-offset", "auto:130.2", "--dry-run",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let j = text(&o);
    assert!(j.contains("\"status\":\"wide\""), "{j}");
    assert!(number(&j, "spread_da") > 0.15, "{j}");
    assert!(o.stderr.contains("warn.mz.offset.wide"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_fixed_offset_is_the_shifted_window_and_auto_is_the_fixed_number_it_measured() {
    let (dir, files) = series("fix", Shape::Plain);
    let f = &files[2];
    let fixed = cli(&["trace", "xic", f, "--mz", "130.2", "--da", "0.3", "--mz-offset", "0.24"]);
    let moved = cli(&["trace", "xic", f, "--mz", "130.44", "--da", "0.3"]);
    assert_eq!(fixed.code, 0, "{}", fixed.stderr);
    assert_eq!(points(&text(&fixed)), points(&text(&moved)));
    let j = text(&fixed);
    assert!(j.contains("\"id\":\"xic:130.2\"") && j.contains("\"mode\":\"fixed\""), "{j}");
    assert_eq!(number(&j, "mz_offset_da"), 0.24);
    let auto = cli(&["trace", "xic", f, "--mz", "130.2", "--da", "0.3", "--mz-offset", "auto:130.2"]);
    assert_eq!(
        without_evidence(&text(&auto)),
        without_evidence(&j),
        "auto is the number it measured"
    );
    // the default writes nothing about the offset
    let none = text(&cli(&["trace", "xic", f, "--mz", "130.2", "--da", "0.3"]));
    assert!(!none.contains("mz_offset"), "{none}");
    // a list and a sweep carry it too
    let list = text(&cli(&["trace", "xic", f, "--mz", "130.2,139.9", "--da", "0.3", "--mz-offset", "0.24"]));
    assert!(list.contains("\"mz_offset_da\":0.24"), "{list}");
    let sweep = text(&cli(&["trace", "xic", f, "--sweep", "130:140:5", "--mz-offset", "0.24"]));
    assert!(sweep.contains("\"mz_offset_da\":0.24"), "{sweep}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_bad_spec_is_exit_2_and_a_missing_file_exit_4() {
    let (dir, files) = series("err", Shape::Plain);
    for bad in ["abc", "0.7", "-0.61", "auto:", "auto:0", "auto:130.2@x"] {
        let o = cli(&["trace", "xic", &files[0], "--mz", "130.2", "--da", "0.3", "--mz-offset", bad]);
        assert_eq!(o.code, 2, "{bad}: {}", o.stderr);
        assert!(o.stderr.contains("err.mz.offset."), "{bad}: {}", o.stderr);
        let o = cli(&["features", &files[0], "--mz-offset", bad, "--dry-run"]);
        assert_eq!(o.code, 2, "features {bad}: {}", o.stderr);
    }
    let o = cli(&["trace", "xic", "/nonexistent/x.mzML", "--mz", "130.2", "--da", "0.3", "--mz-offset", "0.1"]);
    assert_eq!(o.code, 4, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn features_with_auto_puts_each_ion_in_one_row_at_its_true_mz() {
    let (dir, files) = series("feat", Shape::Plain);
    let report = dir.join("report.json").display().to_string();
    let mut a = vec!["features"];
    a.extend(files.iter().map(String::as_str));
    a.extend(["--mz-offset", "auto:130.2", "--report", &report]);
    let o = cli(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let out = text(&o);
    let mut rows = Vec::new();
    for l in out.lines().skip(1) {
        let c: Vec<&str> = l.split(',').collect();
        let mz: f64 = c[1].parse().unwrap();
        assert!(
            ions().iter().any(|&(m, _)| (m - mz).abs() <= 0.1),
            "a row off every true ion: {mz}"
        );
        assert!(
            c.iter().filter(|x| **x == "DETECTED").count() == SHIFTS.len(),
            "{l}"
        );
        rows.push(mz);
    }
    assert_eq!(rows.len(), ions().len(), "one row per ion, no split: {rows:?}");
    let rep = std::fs::read_to_string(&report).unwrap();
    assert!(rep.contains("\"mz_offset_da\":0.19") && rep.contains("\"mz_offset_da\":0.29"), "{rep}");
    // a run without the flag keeps a report without the members
    let mut b = vec!["features"];
    b.extend(files.iter().map(String::as_str));
    b.extend(["--report", &report]);
    assert_eq!(cli(&b).code, 0);
    assert!(!std::fs::read_to_string(&report).unwrap().contains("mz_offset"));
    let _ = std::fs::remove_dir_all(&dir);
}
