//! `features` at unit resolution across samples (`--centroid`, `--report`, `--dry-run`) on a synthetic series of mzML files: eight samples whose m/z
//! axis drifts between -0.2 and +0.2 Da, as profile scans (0.1 Da grid, FWHM 0.7 Da) and as centroids, and a high-resolution series.

use std::path::{Path, PathBuf};

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Options};

use crate::{execute, Outcome};

/// The true ions: nominal m/z with the mass defect 0.2 and the scan of their apex.
const IONS: [(f64, usize); 8] = [
    (130.2, 8),
    (151.2, 14),
    (172.2, 20),
    (193.2, 26),
    (214.2, 32),
    (235.2, 38),
    (256.2, 44),
    (277.2, 50),
];
/// The drift of the m/z axis of each sample, Da.
const DRIFT: [f64; 8] = [-0.2, -0.15, -0.1, -0.05, 0.05, 0.1, 0.15, 0.2];
const SCANS: usize = 60;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Profile,
    Centroid,
    HighRes,
}

/// One mzML of `SCANS` MS1 scans in `kind`, its m/z axis moved by `drift`.
fn mzml(kind: Kind, drift: f64) -> Vec<u8> {
    let head = Header {
        run: "s".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::with_options(Vec::new(), &head, Options::default()).unwrap();
    w.open_spectra(SCANS).unwrap();
    for i in 0..SCANS {
        let heights: Vec<(f64, f64)> = IONS
            .iter()
            .map(|&(mz, apex)| {
                let x = (i as f64 - apex as f64) / 2.0;
                (mz + drift, 2e5 * (-0.5 * x * x).exp())
            })
            .collect();
        let (mz, y): (Vec<f64>, Vec<f32>) = match kind {
            Kind::Profile => {
                let grid: Vec<f64> = (0..2000).map(|k| 100.0 + 0.1 * k as f64).collect();
                let y = grid
                    .iter()
                    .map(|&m| {
                        let s: f64 = heights
                            .iter()
                            .map(|&(c, h)| h * (-0.5 * ((m - c) / 0.3).powi(2)).exp())
                            .sum();
                        (s + 20.0) as f32
                    })
                    .collect();
                (grid, y)
            }
            Kind::Centroid | Kind::HighRes => (
                heights.iter().map(|h| h.0).collect(),
                heights.iter().map(|h| (h.1 + 20.0) as f32).collect(),
            ),
        };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 0.5 + 0.02 * i as f64,
            polarity: 1,
            profile: kind == Kind::Profile,
            filter: if kind == Kind::HighRes {
                "FTMS + p ESI Full ms [100.0000-400.0000]".into()
            } else {
                "+ p ESI Q1MS [100.000-400.000]".into()
            },
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

/// A folder with the eight files `s0.mzML`.. of `kind` (the drifts of `DRIFT`, or none).
fn series(tag: &str, kind: Kind, drifting: bool) -> (PathBuf, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("mzlab_flr_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let files = DRIFT
        .iter()
        .enumerate()
        .map(|(k, d)| {
            let f = dir.join(format!("s{k}.mzML"));
            std::fs::write(&f, mzml(kind, if drifting { *d } else { 0.0 })).unwrap();
            f.display().to_string()
        })
        .collect();
    (dir, files)
}

fn run(files: &[String], extra: &[&str]) -> Outcome {
    let mut a = vec!["features".to_string()];
    a.extend(files.iter().cloned());
    a.extend(extra.iter().map(|s| s.to_string()));
    execute(&a)
}

fn first_lines(o: &Outcome, n: usize) -> String {
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .take(n)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The rows of a features CSV: m/z, RT and the state of each sample.
fn table(o: &Outcome) -> Vec<(f64, f64, Vec<String>)> {
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    let states: Vec<usize> = (0..head.len())
        .filter(|&k| head[k].ends_with(":feature_state"))
        .collect();
    lines
        .map(|l| {
            let c: Vec<&str> = l.split(',').collect();
            (
                c[1].parse().unwrap(),
                c[2].parse().unwrap(),
                states.iter().map(|&k| c[k].to_string()).collect(),
            )
        })
        .collect()
}

/// A params file that sets the three switches of the screen off (`recenter`, `dedup`, and `centroid` unless the flag gives it).
fn off(dir: &Path, with_centroid: bool) -> String {
    let f = dir.join(if with_centroid {
        "off-all.json"
    } else {
        "off.json"
    });
    let c = if with_centroid {
        r#","centroid":"off""#
    } else {
        ""
    };
    std::fs::write(&f, format!(r#"{{"recenter":"off","dedup":"off"{c}}}"#)).unwrap();
    f.display().to_string()
}

#[test]
fn a_profile_series_gives_the_rows_of_the_centroid_series_one_per_ion_in_every_sample() {
    let (dp, profile) = series("rows_p", Kind::Profile, true);
    let (dc, centroid) = series("rows_c", Kind::Centroid, true);
    let (p, c) = (table(&run(&profile, &[])), table(&run(&centroid, &[])));
    assert_eq!(p.len(), c.len(), "profile {p:?}\ncentroid {c:?}");
    assert_eq!(p.len(), IONS.len(), "one row per true ion: {p:?}");
    for (mz, rt, states) in &p {
        assert!(
            IONS.iter().any(|&(m, _)| (m - mz).abs() <= 0.1),
            "a row on no true ion: {mz} {rt}"
        );
        assert!(states.iter().all(|s| s == "DETECTED"), "{mz}: {states:?}");
    }
    for (k, a) in p.iter().enumerate() {
        for b in &p[k + 1..] {
            assert!(
                (a.0 - b.0).abs() > 0.6 || (a.1 - b.1).abs() > 0.1,
                "duplicates {a:?} {b:?}"
            );
        }
    }
    let _ = (std::fs::remove_dir_all(dp), std::fs::remove_dir_all(dc));
}

#[test]
fn the_defaults_leave_a_high_resolution_series_and_a_centroid_series_as_they_were() {
    // the three switches off is the path of before: the files that have nothing to centroid, shift or merge give its bytes
    for (tag, kind) in [("hr", Kind::HighRes), ("lrc", Kind::Centroid)] {
        let (dir, files) = series(tag, kind, false);
        let default = run(&files, &[]);
        let before = run(
            &files,
            &["--centroid", "off", "--params", &off(&dir, false)],
        );
        assert_eq!(default.code, 0, "{}", default.stderr);
        assert_eq!(default.stdout, before.stdout, "{tag}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn the_switches_off_reproduce_the_profile_csv_of_before_and_the_flag_is_the_key() {
    let (dir, files) = series("off", Kind::Profile, true);
    let all_off = off(&dir, true);
    let notes = dir.join("notes.json").display().to_string();
    let before = run(&files, &["--params", &all_off, "--notes", &notes]);
    let flag = run(
        &files,
        &["--centroid", "off", "--params", &off(&dir, false)],
    );
    assert_eq!(
        before.stdout, flag.stdout,
        "--centroid off is the key centroid=off"
    );
    // with the screen off the profile warning stays, as before
    let said = std::fs::read_to_string(&notes).unwrap();
    assert!(said.contains("warn.features.profileUnit"), "{said}");
    let (old, new) = (table(&before), table(&run(&files, &[])));
    assert!(
        old.len() >= 2 * new.len(),
        "the profile points of before split the ions: {} rows against {}",
        old.len(),
        new.len()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_report_says_what_each_file_went_through_and_the_table_is_the_one_without_it() {
    let (dir, files) = series("report", Kind::Profile, true);
    let path = dir.join("report.json");
    let with = run(&files, &["--report", &path.display().to_string()]);
    assert_eq!(with.stdout, run(&files, &[]).stdout);
    let doc = mzlab_core::json::parse(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        doc.get("schema").and_then(|v| v.str()),
        Some("features-report/1")
    );
    let list = doc.get("files").unwrap().arr();
    assert_eq!(list.len(), DRIFT.len());
    for (k, f) in list.iter().enumerate() {
        let num = |key: &str| f.get(key).and_then(|v| v.f64()).unwrap();
        assert_eq!(f.get("unit").and_then(|v| v.bool()), Some(true));
        assert_eq!(num("profile_fraction"), 1.0);
        assert_eq!(f.get("centroided").and_then(|v| v.bool()), Some(true));
        assert_eq!(num("rows") as usize, IONS.len(), "{f:?}");
        // the shift of a file is its drift from the median of the series (0 here), within 0.03 Da
        assert!((num("mz_shift_da") - DRIFT[k]).abs() <= 0.03, "{k}: {f:?}");
        let applied = f.get("recentred").and_then(|v| v.bool()).unwrap();
        // subtracted from 0.05 Da, the smallest shift the data can tell from the noise of the centroids
        assert_eq!(applied, num("mz_shift_da").abs() >= 0.05, "{k}: {f:?}");
    }
    let choices = doc.get("choices").unwrap().arr();
    assert!(
        choices
            .iter()
            .any(|c| c.get("key").and_then(|v| v.str()) == Some("centroid")),
        "{choices:?}"
    );
    assert!(doc.get("merged").is_some());
    // `tests/test_features_report_schema.py` checks the two documents against their schemas
    if let Some(to) = std::env::var_os("MZLAB_JSON_SAMPLES") {
        std::fs::copy(&path, Path::new(&to).join("features-report.json")).unwrap();
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_dry_run_prints_the_plan_detects_nothing_and_writes_nothing() {
    let (dir, files) = series("dry", Kind::Profile, true);
    let path = dir.join("never.json");
    let o = run(
        &files,
        &["--dry-run", "--report", &path.display().to_string()],
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(!path.exists());
    if let Some(to) = std::env::var_os("MZLAB_JSON_SAMPLES") {
        std::fs::write(Path::new(&to).join("features-plan.json"), &o.stdout).unwrap();
    }
    let doc = mzlab_core::json::parse(&o.stdout).unwrap();
    assert_eq!(
        doc.get("schema").and_then(|v| v.str()),
        Some("features-plan/1")
    );
    let members = doc.get("members").unwrap().arr();
    assert_eq!(members.len(), DRIFT.len());
    assert_eq!(
        members[0].get("centroid").and_then(|v| v.bool()),
        Some(true)
    );
    // exit codes: 4 a missing file, 2 a usage error
    assert_eq!(run(&["/nonexistent/x.mzML".into()], &["--dry-run"]).code, 4);
    assert_eq!(run(&files, &["--dry-run", "--centroid", "maybe"]).code, 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_table_of_the_command_is_the_table_of_the_flow() {
    let (dir, files) = series("flow", Kind::Profile, true);
    let names: Vec<String> = (0..DRIFT.len()).map(|k| format!("s{k}.mzML")).collect();
    let rows: Vec<_> = names
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    std::fs::write(
        dir.join("samples.json"),
        mzlab_core::samples::to_json(&mzlab_core::samples::propose(&rows)),
    )
    .unwrap();
    let raw: Vec<String> = names
        .iter()
        .map(|n| format!(r#"{{"name":"{n}","type":"raw"}}"#))
        .collect();
    let src: Vec<String> = names.iter().map(|n| format!(r#""file:{n}""#)).collect();
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{},{{"name":"samples.json","type":"samples"}}],"steps":[
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":[{}]}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
        {{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
        {{"id":"gap","step":"features.gapfill@1","in":{{"table":"ali.table","index":["idx.index"]}}}},
        {{"id":"csv","step":"export.table_csv@1","in":{{"table":"gap.table","samples":"file:samples.json"}}}}]}}"#,
        raw.join(","),
        src.join(",")
    );
    let w = dir.join("work");
    std::fs::create_dir_all(&w).unwrap();
    let (done, prov) = crate::flow::run_plan(plan.as_bytes(), &dir, &w, 2, None).unwrap();
    assert!(
        done.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let flow = std::fs::read_to_string(w.join("csv.csv")).unwrap();
    let cli = run(&files, &[]);
    assert_eq!(cli.code, 0, "{}", cli.stderr);
    // the rows (m/z, RT, areas, states) are the same; the header and the origin metadata are of each entry
    let body = |t: &str| t.lines().skip(1).map(str::to_string).collect::<Vec<_>>();
    assert_eq!(body(&flow), body(&String::from_utf8_lossy(&cli.stdout)));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A synthetic `.wiff` of three samples whose scans are profile (every bin of the block, `tools/wiff_grande.py`). python3 is part of the test.
fn wiff(tag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("mzlab_flr_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = std::process::Command::new("python3")
        .arg(tools.join("wiff_grande.py"))
        .arg(dir.join("in"))
        .args(["--samples", "3", "--scans", "40", "--points", "3333"])
        .output()
        .expect("python3 is needed by this test");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    (
        dir.clone(),
        dir.join("in/grande.wiff").display().to_string(),
    )
}

#[test]
fn a_wiff_of_profile_scans_is_centroided_per_sample_and_the_dry_run_checks_the_selection() {
    let (dir, w) = wiff("wiff");
    let report = dir.join("report.json").display().to_string();
    let o = run(&[w.clone()], &["--sample", "1,2,3", "--report", &report]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let doc = mzlab_core::json::parse(&std::fs::read(&report).unwrap()).unwrap();
    let list = doc.get("files").unwrap().arr();
    assert_eq!(list.len(), 3, "one member per chosen sample");
    for (k, f) in list.iter().enumerate() {
        let m = f.get("member").and_then(|v| v.str()).unwrap();
        assert!(m.ends_with(&format!("grande.wiff@{}", k + 1)), "{m}");
        assert_eq!(
            f.get("profile_fraction").and_then(|v| v.f64()),
            Some(1.0),
            "{f:?}"
        );
        assert_eq!(
            f.get("centroided").and_then(|v| v.bool()),
            Some(true),
            "{f:?}"
        );
    }
    // the plan says the same without detecting; an unknown sample is exit 5 in both
    let plan = run(&[w.clone()], &["--sample", "1,2,3", "--dry-run"]);
    assert_eq!(plan.code, 0, "{}", plan.stderr);
    assert_eq!(run(&[w.clone()], &["--sample", "7", "--dry-run"]).code, 5);
    assert_eq!(run(&[w], &["--sample", "7"]).code, 5);
    let _ = std::fs::remove_dir_all(&dir);
}
