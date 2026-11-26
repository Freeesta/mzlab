//! `features --params '{"preset": "nominal"}'` runs the profile for nominal-resolution data on synthetic unit-resolution files (a parent and two low-mass products): the products the
//! default drops are in the table, the report's `choices` name the profile with the numbers it runs with, and a high-resolution recipe is left alone. Synthetic data only.

use mzlab_core::json::{parse, Value};

use crate::flow_nominal_tests::mzml;
use crate::{execute, Outcome};

fn files(tag: &str) -> (std::path::PathBuf, Vec<String>) {
    let d = std::env::temp_dir().join(format!("mzlab_cli_fnom_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let names = (0..3)
        .map(|k| {
            let p = d.join(format!("n{k}.mzML"));
            std::fs::write(&p, mzml(5)).unwrap();
            p.to_string_lossy().into_owned()
        })
        .collect();
    (d, names)
}

fn run(files: &[String], extra: &[&str]) -> Outcome {
    let mut a = vec!["features".to_string()];
    a.extend(files.iter().cloned());
    a.extend(extra.iter().map(|s| s.to_string()));
    execute(&a)
}

/// Rows of the CSV within 0.5 Da of the nominal `mz` (calibration offset 0.3 Da).
fn rows_near(o: &Outcome, mz: f64) -> usize {
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let mut lines = text.lines();
    let col = lines
        .next()
        .and_then(|h| h.split(',').position(|c| c == "mz"))
        .expect("an mz column");
    lines
        .filter_map(|l| l.split(',').nth(col)?.parse::<f64>().ok())
        .filter(|m| (m - (mz + 0.3)).abs() <= 0.5)
        .count()
}

fn profile_choice(path: &std::path::Path) -> (String, String) {
    let v = parse(&std::fs::read(path).unwrap()).unwrap();
    let c = v
        .get("choices")
        .map(Value::arr)
        .unwrap()
        .iter()
        .find(|c| c.get("key").and_then(Value::str) == Some("profile"))
        .expect("a profile choice");
    let s = |k: &str| c.get(k).and_then(Value::str).unwrap().to_string();
    (s("value"), s("reason"))
}

#[test]
fn the_nominal_profile_keeps_the_low_mass_products_and_the_report_names_the_profile() {
    let (d, f) = files("run");
    let params = d.join("p.json");
    std::fs::write(&params, r#"{"preset":"nominal"}"#).unwrap();
    let (r_def, r_nom) = (d.join("def.json"), d.join("nom.json"));
    let p = |x: &std::path::Path| x.to_string_lossy().into_owned();
    let def = run(&f, &["--report", &p(&r_def)]);
    let nom = run(&f, &["--params", &p(&params), "--report", &p(&r_nom)]);
    for mz in [91.05, 77.04] {
        assert_eq!(rows_near(&def, mz), 0, "the default drops {mz}");
        assert!(rows_near(&nom, mz) >= 1, "the profile keeps {mz}");
    }
    let (value, why) = profile_choice(&r_def);
    assert_eq!(value, "lr");
    assert!(
        why.contains("floor 3000") && why.contains("\"preset\": \"nominal\""),
        "{why}"
    );
    let (value, why) = profile_choice(&r_nom);
    assert_eq!(value, "nominal");
    assert!(
        why.contains("floor 1000") && why.contains("grid_touch 0.25"),
        "{why}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_unknown_profile_is_refused() {
    let (d, f) = files("bad");
    let params = d.join("p.json");
    std::fs::write(&params, r#"{"preset":"fast"}"#).unwrap();
    let o = run(&f, &["--params", &params.to_string_lossy()]);
    assert_ne!(o.code, 0, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&d);
}
