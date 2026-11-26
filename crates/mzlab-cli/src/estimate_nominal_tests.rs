//! The nominal-resolution hint of `estimate` and of `features --report` on synthetic unit-resolution series: a parent compound with companions and, with
//! products, three low-mass products a few thousand counts high; no random peaks, so the rows counted are the compounds.

use super::{run, Value};
use crate::execute;
use mzlab_core::json;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};

const BIG: [(f64, f64); 3] = [(301.10, 3.0e6), (323.08, 4.0e5), (283.09, 3.0e5)];
const LOW: [(f64, f64); 3] = [(105.03, 9.0e3), (91.05, 5.0e3), (77.04, 3.5e3)];

fn mzml(products: bool, jit: f64, seed: u64, hr: bool) -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "n".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 270;
    w.open_spectra(n).unwrap();
    let mut state = seed;
    let mut noise = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 3.4
    };
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let comps: Vec<(f64, f64)> = if products {
            BIG.iter().chain(&LOW).copied().collect()
        } else {
            BIG.to_vec()
        };
        let mut peaks: Vec<(f64, f32)> = comps
            .iter()
            .map(|&(m, a)| {
                let v = a * (-0.5 * ((rt - 6.0) / 0.07).powi(2)).exp();
                (if hr { m } else { m + 0.3 + jit * noise() }, v as f32)
            })
            .filter(|p| p.1 > 50.0)
            .collect();
        peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        w.spectrum(&Spectrum {
            index: i,
            id: format!("sample=1 period=1 cycle={} experiment=1", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: if hr {
                "FTMS + p ESI Full ms [100.0000-400.0000]".into()
            } else {
                "+ p ESI Q1MS [50.000-400.000]".into()
            },
            precursor: None,
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn series(tag: &str, products: bool, jit: f64, hr: bool) -> (std::path::PathBuf, Vec<String>) {
    let dir = std::env::temp_dir().join(format!("mzlab_nh_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let files = (0..3u64)
        .map(|k| {
            let f = dir.join(format!("n{k}.mzML"));
            std::fs::write(&f, mzml(products, jit, 5 + k, hr)).unwrap();
            f.display().to_string()
        })
        .collect();
    (dir, files)
}

fn estimate(files: &[String], extra: &[&str]) -> crate::Outcome {
    let mut a: Vec<&str> = files.iter().map(String::as_str).collect();
    a.extend(extra);
    run(&a)
}

fn keys(d: &Value, list: &str) -> Vec<String> {
    d.get(list)
        .map_or(&[][..], Value::arr)
        .iter()
        .filter_map(|n| n.get("key").or(n.get("set")).and_then(Value::str))
        .map(String::from)
        .collect()
}

#[test]
fn a_series_with_the_signature_gets_the_hint_with_its_numbers_and_nothing_is_applied() {
    let (dir, files) = series("sig", true, 0.06, false);
    let d = json::parse(&estimate(&files, &["--json"]).stdout).unwrap();
    let hints = d.get("hints").unwrap().arr();
    assert_eq!(hints.len(), 1, "{hints:?}");
    let h = &hints[0];
    assert_eq!(
        h.get("set").and_then(Value::str),
        Some("detect.preset=nominal")
    );
    assert_eq!(h.get("files").and_then(Value::f64), Some(3.0));
    assert_eq!(h.get("of").and_then(Value::f64), Some(3.0));
    assert!(h.get("between_share").and_then(Value::f64).unwrap() >= 0.1);
    assert!(keys(&d, "notes").contains(&"note.estimate.nominal".to_string()));
    // the hint is never one of the proposals that `--emit-set` turns into settings
    assert!(!keys(&d, "params").iter().any(|s| s.contains("preset")));
    for f in d.get("files").unwrap().arr() {
        let n = f
            .get("nominal")
            .expect("the unit-axis file carries its numbers");
        assert_eq!(n.get("hint"), Some(&Value::Bool(true)), "{n:?}");
        assert_eq!(n.get("height_nominal").and_then(Value::f64), Some(2500.0));
    }
    let table = estimate(&files, &[]);
    assert!(String::from_utf8_lossy(&table.stdout)
        .contains("--set detect.preset=nominal   # hint, not applied"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_series_without_it_gets_the_numbers_and_no_hint() {
    let (dir, files) = series("tidy", false, 0.02, false);
    let d = json::parse(&estimate(&files, &["--json"]).stdout).unwrap();
    assert!(d.get("hints").unwrap().arr().is_empty());
    assert!(!keys(&d, "notes").contains(&"note.estimate.nominal".to_string()));
    for f in d.get("files").unwrap().arr() {
        let n = f.get("nominal").unwrap();
        assert_eq!(n.get("hint"), Some(&Value::Bool(false)), "{n:?}");
        assert_eq!(n.get("between").and_then(Value::f64), Some(0.0));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_high_resolution_file_has_no_nominal_member() {
    let (dir, files) = series("hr", true, 0.0, true);
    let d = json::parse(&estimate(&files, &["--json"]).stdout).unwrap();
    assert!(d.get("hints").unwrap().arr().is_empty());
    for f in d.get("files").unwrap().arr() {
        assert_eq!(f.get("axis").and_then(Value::str), Some("ppm"));
        assert!(f.get("nominal").is_none());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ions_split_across_the_grids_give_the_hint_without_any_low_mass_product() {
    let (dir, files) = series("split", false, 0.4, false);
    let d = json::parse(&estimate(&files, &["--json"]).stdout).unwrap();
    let hints = d.get("hints").unwrap().arr();
    assert_eq!(hints.len(), 1, "{hints:?}");
    assert!(hints[0].get("split_share").and_then(Value::f64).unwrap() > 0.0);
    assert_eq!(
        hints[0].get("between_share").and_then(Value::f64),
        Some(0.0)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_report_names_the_profile_with_the_numbers_unless_it_is_in_force() {
    let (dir, files) = series("rep", true, 0.06, false);
    let report = |extra: &[&str]| {
        let path = dir.join("r.json");
        let mut a = vec!["features".to_string()];
        a.extend(files.iter().cloned());
        a.extend(["--report", &path.display().to_string()].map(String::from));
        a.extend(extra.iter().map(|s| s.to_string()));
        let o = execute(&a);
        assert_eq!(o.code, 0, "{}", o.stderr);
        json::parse(std::fs::read_to_string(&path).unwrap().as_bytes()).unwrap()
    };
    let r = report(&[]);
    let hint = r
        .get("choices")
        .unwrap()
        .arr()
        .iter()
        .find(|c| c.get("key").and_then(Value::str) == Some("nominal_hint"))
        .expect("the choice is in the report")
        .clone();
    let why = hint.get("reason").and_then(Value::str).unwrap();
    assert!(
        why.contains("candidate rows") && why.contains("{\"preset\": \"nominal\"}"),
        "{why}"
    );
    let params = dir.join("p.json");
    std::fs::write(&params, r#"{"preset":"nominal"}"#).unwrap();
    let r = report(&["--params", &params.display().to_string()]);
    assert!(r
        .get("choices")
        .unwrap()
        .arr()
        .iter()
        .all(|c| c.get("key").and_then(Value::str) != Some("nominal_hint")));
    let _ = std::fs::remove_dir_all(&dir);
}
