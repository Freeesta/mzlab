//! `features --params '{"preset": "nominal"}'` runs the profile for nominal-resolution data on synthetic unit-resolution files (a parent and two low-mass products): the products the
//! default drops are in the table, the report's `choices` name the profile with the numbers it runs with, and a high-resolution recipe is left alone. Synthetic data only.

use mzlab_core::json::{parse, Value};

use crate::flow_nominal_tests::mzml;
use crate::{execute, Outcome};

const EXIT: u8 = 2;

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
    assert_eq!(o.code, EXIT, "{}", o.stderr);
    assert!(o.stderr.contains("error.badRecipe"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&d);
}

/// A high-resolution file: fine fractional masses, strong ions.
fn hr_mzml(seed: u64) -> Vec<u8> {
    use mzlab_core::mzml::Spectrum;
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let mut st = seed;
    let mut next = move || {
        st = st
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (st >> 11) as f64 / (1u64 << 53) as f64
    };
    let hdr = Header {
        run: "hr".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::new(Vec::new(), &hdr).unwrap();
    let n = 120;
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let mut peaks: Vec<(f64, f32)> = (0..300)
            .map(|_| (100.0 + 400.0 * next(), (2e4 * next()) as f32))
            .collect();
        let v = 5.0e6 * (-0.5 * ((rt - 3.0) / 0.07).powi(2)).exp();
        peaks.push((301.14120, v as f32));
        peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        w.spectrum(&Spectrum {
            index: i,
            id: format!("sample=1 period=1 cycle={} experiment=1", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: "FTMS + p ESI Full ms [100.0000-500.0000]".into(),
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

#[test]
fn the_profile_asked_for_on_a_high_resolution_file_is_reported_as_ignored() {
    let d = std::env::temp_dir().join(format!("mzlab_cli_fnom_hr_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f: Vec<String> = (0..2)
        .map(|k| {
            let p = d.join(format!("h{k}.mzML"));
            std::fs::write(&p, hr_mzml(k + 1)).unwrap();
            p.to_string_lossy().into_owned()
        })
        .collect();
    let params = d.join("p.json");
    std::fs::write(&params, r#"{"preset":"nominal"}"#).unwrap();
    let rep = d.join("r.json");
    let o = run(
        &f,
        &[
            "--params",
            &params.to_string_lossy(),
            "--report",
            &rep.to_string_lossy(),
        ],
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&std::fs::read(&rep).unwrap()).unwrap();
    let choices = v.get("choices").map(Value::arr).unwrap();
    let get = |k: &str| {
        choices
            .iter()
            .find(|c| c.get("key").and_then(Value::str) == Some(k))
    };
    assert_eq!(
        get("axis")
            .and_then(|c| c.get("value"))
            .and_then(Value::str),
        Some("ppm")
    );
    let c = get("preset").expect("the ignored preset is a choice");
    assert_eq!(c.get("value").and_then(Value::str), Some("ignored"));
    assert!(c
        .get("reason")
        .and_then(Value::str)
        .unwrap()
        .contains("high resolution"));
    assert!(
        get("profile").is_none(),
        "no unit profile is named for a high-resolution file"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// The task an agent would say finds the step that carries the profile among the first three.
#[test]
fn the_nominal_profile_is_found_from_the_task() {
    let o = execute(
        &[
            "commands",
            "--find",
            "products",
            "are",
            "lost",
            "at",
            "nominal",
            "resolution",
            "on",
            "a",
            "triple",
            "quadrupole",
            "--json",
        ]
        .map(String::from),
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&o.stdout).unwrap();
    let names: Vec<String> = v
        .get("matches")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .take(3)
        .filter_map(|r| r.get("name").and_then(Value::str).map(String::from))
        .collect();
    assert!(names.iter().any(|n| n == "features.detect"), "{names:?}");
}
