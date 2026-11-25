//! `dedup` of `features` on a synthetic replicate series (`tools/lr_synthetic.py --replicates 4`, run into a temp dir; skipped without python3 and
//! numpy; `MZLAB_GOLDEN_STRICT`, `CI` or `GITHUB_ACTIONS` set fails instead): one peak cut in two rows is merged by the overlap rule with every number in `--report`, the
//! threshold of `--params` undoes it, and the hint to that threshold is in the notes and the report.

use std::path::PathBuf;
use std::process::Command;

use mzlab_core::json::{parse, Value};

use crate::{execute, Outcome};

fn tools() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools")
}

/// The four replicates in a new folder; `None` without python3 and numpy.
fn replicates(tag: &str) -> Option<(PathBuf, Vec<String>)> {
    let d = std::env::temp_dir().join(format!("mzlab_dedup_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let ok = Command::new("python3")
        .arg(tools().join("lr_synthetic.py"))
        .arg(&d)
        .args(["--seed", "1", "--replicates", "4"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        // CI must run this test, never skip it
        assert!(
            ["MZLAB_GOLDEN_STRICT", "CI", "GITHUB_ACTIONS"]
                .iter()
                .all(|k| std::env::var_os(k).is_none()),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return None;
    }
    let files = (0..4)
        .map(|k| d.join(format!("rep-s{k}.mzML")).display().to_string())
        .collect();
    Some((d, files))
}

fn run(files: &[String], extra: &[&str]) -> Outcome {
    let mut a = vec!["features".to_string()];
    a.extend(files.iter().cloned());
    a.extend(extra.iter().map(|s| s.to_string()));
    execute(&a)
}

fn rows(o: &Outcome) -> usize {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8_lossy(&o.stdout).lines().count() - 1
}

fn report(path: &std::path::Path) -> Value {
    parse(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn a_split_peak_is_merged_with_its_numbers_and_the_threshold_undoes_it() {
    let Some((d, files)) = replicates("merge") else {
        return;
    };
    let (on, off) = (d.join("on.json"), d.join("off.json"));
    let none = d.join("none.json");
    std::fs::write(&none, r#"{"dedup_overlap":0}"#).unwrap();
    let a = run(&files, &["--report", &on.display().to_string()]);
    let b = run(
        &files,
        &[
            "--params",
            &none.display().to_string(),
            "--report",
            &off.display().to_string(),
        ],
    );
    let (r_on, r_off) = (report(&on), report(&off));
    let merged = r_on.get("merged").unwrap().arr();
    assert!(!merged.is_empty(), "the replicates hold split peaks");
    assert_eq!(rows(&a) + merged.len(), rows(&b), "one row less per merge");
    assert!(r_off
        .get("merged")
        .unwrap()
        .arr()
        .iter()
        .all(|m| m.get("rule").and_then(Value::str) == Some("correlation")));
    for m in merged
        .iter()
        .filter(|m| m.get("rule").and_then(Value::str) == Some("overlap"))
    {
        for key in [
            "kept",
            "dropped",
            "dmz_da",
            "drt_min",
            "r",
            "overlap",
            "shared_files",
            "threshold",
            "height_kept",
            "height_dropped",
        ] {
            assert!(m.get(key).is_some(), "{key}");
        }
        assert!(m.get("dmz_da").and_then(Value::f64).unwrap() <= 0.6);
        assert!(m.get("overlap").and_then(Value::f64).unwrap() >= 0.5);
        assert!(m.get("shared_files").and_then(Value::f64).unwrap() >= 1.0);
        assert!(
            m.get("height_kept").and_then(Value::f64).unwrap()
                >= m.get("height_dropped").and_then(Value::f64).unwrap()
        );
    }
    assert!(merged
        .iter()
        .any(|m| m.get("rule").and_then(Value::str) == Some("overlap")));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn rows_left_close_are_in_the_report_and_the_hint_names_the_threshold() {
    let Some((d, files)) = replicates("near") else {
        return;
    };
    let rep = d.join("r.json");
    let o = run(&files, &["--report", &rep.display().to_string()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let near = report(&rep);
    let near = near.get("near").unwrap().arr();
    assert!(
        !near.is_empty(),
        "the replicates keep close rows of two ions"
    );
    for p in near {
        assert!(p.get("dmz_da").and_then(Value::f64).unwrap() <= 0.6);
        assert!(p.get("shared_files").is_some() && p.get("overlap").is_some());
        assert!(p.get("overlap_wide").is_some());
    }
    let total = report(&rep).get("near_total").and_then(Value::f64).unwrap();
    assert!(total as usize >= near.len());
    assert!(near.len() <= 20);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_hint_is_the_catalogue_text_that_names_dedup_overlap() {
    let mut log = mzlab_feat::lrscreen::Log::default();
    assert!(crate::par::features_report::near_notice(&log).is_none());
    log.near.push(mzlab_feat::lrscreen::Near {
        a: (250.2, 5.0),
        b: (250.7, 5.05),
        dmz_da: 0.5,
        drt_min: 0.05,
        r: f64::NAN,
        overlap: 0.4,
        overlap_wide: 0.2,
        shared: 2,
    });
    let n = crate::par::features_report::near_notice(&log).unwrap();
    assert_eq!(n.key, "note.features.nearDup");
    let text = crate::error_text::warning(&n);
    assert!(
        text.contains("dedup_overlap")
            && text.contains("250.200")
            && text.contains("1 pair")
            && text.contains("different ions"),
        "{text}"
    );
}
