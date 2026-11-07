//! `features.detect` `max_gap` counts the missed scans (0 = consecutive), as `features.gapfill`: a plan saved at `features.detect@1` (scan distance N) is
//! migrated to `@2` with N - 1 and gives the same bytes as a plan written at `@2`, in `flow run`, in the desktop route and in the browser runner; the page
//! route `flow/line` migrates it to the plan the command line runs. Synthetic ion traces that miss one or two scans, so every value changes the table.

use std::path::{Path, PathBuf};

use mzlab_core::flow::{line_route, parse, plan_json};
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};

use crate::flow_run_tests::{flow_run, p};
use crate::tests::layered_table;

const IONS: [(f64, f64, f64); 5] = [
    (201.1234, 2.0, 3.0e6),
    (255.0987, 2.6, 1.5e6),
    (301.1456, 3.1, 4.0e6),
    (333.2101, 3.7, 8.0e5),
    (389.1789, 4.3, 2.0e6),
];

/// Full scans every 3 s of five accurate-mass ions; ion `k` misses scan `i` when `(i + 3k) % 6 == 0` (one scan) or `(i + k) % 13` is 0 or 1 (two scans).
fn mzml_gaps(seed: u32) -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: format!("g{seed}"),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 140;
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let mut peaks: Vec<(f64, f32)> = IONS
            .iter()
            .enumerate()
            .filter(|(k, _)| !(i + 3 * k + seed as usize).is_multiple_of(6) && (i + k) % 13 > 1)
            .map(|(_, &(m, c, a))| (m, (a * (-0.5 * ((rt - c) / 0.15).powi(2)).exp()) as f32))
            .filter(|q| q.1 > 100.0)
            .collect();
        peaks.push((150.0371 + (i % 7) as f64 * 1.013, 2e3));
        peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        w.spectrum(&Spectrum {
            index: i,
            id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: "FTMS + p ESI Full ms [100.0000-900.0000]".into(),
            precursor: None,
            mz: peaks.iter().map(|q| q.0).collect(),
            intensity: peaks.iter().map(|q| q.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// The base chain over the three files, the detection at `version` with `params` (a JSON object).
fn plan(version: u32, params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"g0.mzML","type":"raw"}},{{"name":"g1.mzML","type":"raw"}},{{"name":"g2.mzML","type":"raw"}}],"steps":[
{{"id":"idx","step":"index.mzidx@1","in":{{"source":["file:g0.mzML","file:g1.mzML","file:g2.mzML"]}}}},
{{"id":"det","step":"features.detect@{version}","in":{{"index":["idx.index"]}},"params":{params}}},
{{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
{{"id":"gap","step":"features.gapfill@1","in":{{"table":"ali.table","index":["idx.index"]}}}}]}}"#
    )
}

fn fixture() -> (PathBuf, Vec<(String, Vec<u8>)>) {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_max_gap_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let files: Vec<(String, Vec<u8>)> = (0..3)
        .map(|s| (format!("g{s}.mzML"), mzml_gaps(s)))
        .collect();
    for (n, b) in &files {
        std::fs::write(dir.join("data").join(n), b).unwrap();
    }
    (dir, files)
}

/// The tables `flow run` writes for `plan` (per-file detections, aligned and filled), sorted by name.
fn tables(dir: &Path, tag: &str, plan: &str) -> Vec<(String, Vec<u8>)> {
    let (file, out) = (dir.join(format!("{tag}.json")), dir.join(tag));
    std::fs::write(&file, plan).unwrap();
    let o = flow_run(&[p(&file), "--in", p(&dir.join("data")), "--out", p(&out)]);
    assert_eq!(o.code, 0, "{tag}: {}", o.stderr);
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|f| {
            let n = f.to_string_lossy();
            n.ends_with(".mzfeat") || n.ends_with(".mzfeatf") || n.ends_with(".weak")
        })
        .map(|f| {
            (
                f.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&f).unwrap(),
            )
        })
        .collect();
    v.sort();
    v
}

fn get<'a>(t: &'a [(String, Vec<u8>)], name: &str) -> &'a [u8] {
    &t.iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn a_detect_1_plan_with_scan_distance_n_gives_the_bytes_of_detect_2_with_n_minus_1_missed_scans() {
    let (dir, files) = fixture();
    // the defaults: 2 at version 1, 1 at version 2
    let default = tables(&dir, "new_default", &plan(2, "{}"));
    assert_eq!(tables(&dir, "old_default", &plan(1, "{}")), default);
    assert_eq!(default, tables(&dir, "new_1", &plan(2, r#"{"max_gap":1}"#)));
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for n in 1..=4u32 {
        let new = tables(
            &dir,
            &format!("new_{n}"),
            &plan(2, &format!(r#"{{"max_gap":{}}}"#, n - 1)),
        );
        assert_eq!(
            tables(
                &dir,
                &format!("old_{n}"),
                &plan(1, &format!(r#"{{"max_gap":{n}}}"#))
            ),
            new,
            "old {n}"
        );
        // the desktop route runs the same plan
        let desk = dir.join(format!("desk_{n}"));
        std::fs::create_dir_all(&desk).unwrap();
        let text = plan(1, &format!(r#"{{"max_gap":{n}}}"#));
        crate::flow::run_plan(text.as_bytes(), &dir.join("data"), &desk, 1, None).unwrap();
        assert_eq!(
            std::fs::read(desk.join("gap.mzfeat")).unwrap(),
            get(&new, "gap.mzfeat"),
            "desktop {n}"
        );
        // the browser runner: the layers of the migrated plan
        let layers = format!(
            r#"[{{"step":"features.detect","params":{{"max_gap":{}}}}}]"#,
            n - 1
        );
        assert!(
            layered_table(&files, &layers, true) == get(&new, "ali.mzfeat"),
            "browser {n}"
        );
        seen.push(get(&new, "gap.mzfeat").to_vec());
    }
    // the traces miss scans: each value gives its own table
    for a in 0..seen.len() {
        for b in a + 1..seen.len() {
            assert_ne!(seen[a], seen[b], "max_gap {a} and {b}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_page_route_migrates_a_detect_1_plan_to_the_plan_the_command_line_runs() {
    let steps = crate::flow::steps();
    for (old, new) in [(0, 0), (1, 0), (2, 1), (5, 4)] {
        let text = plan(1, &format!(r#"{{"max_gap":{old},"floor":5000}}"#));
        let cli = parse(text.as_bytes()).unwrap();
        let det = &cli.steps[1];
        assert_eq!(det.version, 2);
        assert!(
            det.params
                .iter()
                .any(|(k, v)| k == "max_gap" && v.f64() == Some(f64::from(new))),
            "{old}"
        );
        let body = serde_json::json!({"plan": text, "line": "mzlab-cli flow run"}).to_string();
        let page = line_route(&body, Some(&steps)).unwrap();
        assert_eq!(page, plan_json(&cli), "{old}");
        assert!(page.contains("features.detect@2"));
        // a plan already at version 2 is not converted again
        let v2 = parse(plan(2, &format!(r#"{{"max_gap":{old}}}"#)).as_bytes()).unwrap();
        assert!(v2.steps[1]
            .params
            .iter()
            .any(|(k, v)| k == "max_gap" && v.f64() == Some(f64::from(old))));
    }
}

#[test]
fn the_detect_range_starts_at_0_missed_scans_and_the_defaults_are_1_and_2() {
    let steps = crate::flow::steps();
    let param = |id: &str| {
        let s = steps.iter().find(|s| s.id == id).unwrap();
        let q = s.params.iter().find(|q| q.name == "max_gap").unwrap();
        (s.version, q.min, q.max, q.default.clone())
    };
    let (v, min, max, def) = param("features.detect");
    assert_eq!((v, min, max), (2, Some(0.0), None));
    assert_eq!(def.and_then(|d| d.f64()), Some(1.0));
    let (_, min, _, def) = param("features.gapfill");
    assert_eq!((min, def.and_then(|d| d.f64())), (Some(0.0), Some(2.0)));
    // the checker takes 0..k and refuses a negative count, at the current version
    let check = |v: i32| {
        let plan = parse(plan(2, &format!(r#"{{"max_gap":{v}}}"#)).as_bytes()).unwrap();
        mzlab_core::flow::check(&plan, &steps, mzlab_core::flow::Platform::Native)
            .iter()
            .filter(|q| q.instance == "det")
            .map(|q| q.error_key)
            .collect::<Vec<_>>()
    };
    for ok in [0, 1, 2, 7] {
        assert!(check(ok).is_empty(), "{ok}: {:?}", check(ok));
    }
    assert_eq!(check(-1), ["err.flow.param.range"]);
}
