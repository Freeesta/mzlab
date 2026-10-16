//! The Dig on a series that compares conditions (`docs/agenti/gruppi.md`): the 38 synthetic mzML of `tests/data/gruppi/` (6 conditions × 3 times × 2
//! replicates, a blank and a standard of the parent; the truth is `attesi.json` and its `LEGGIMI.md`), the roles and conditions from
//! `campioni::proponi`. The pipeline runs natively as the pool of `feat-worker.js` runs it, then the Dig and the block `gruppi`.
//! Criteria 8.1-8.4 of `docs/studi/TABUI_GRUPPI_ESPERIMENTI.md`.

use mzlab_core::campioni;
use mzlab_feat::legacy::lr::{self, summary::summary};
use serde_json::Value;
use std::path::PathBuf;

mod comune;
use comune::*;

const BENTAZONE: &str = r#"{"name": "Bentazone", "neutral": "C10H12N2O3S"}"#;
const CONDS: [&str; 6] = ["Air", "Ar", "He", "N2", "O2", "Reference"];

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/gruppi")
}

/// The names of the mzML, sorted.
fn names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".mzML"))
        .collect();
    v.sort();
    v
}

/// `{"files": [...], "campioni": campioni/2}` as the page passes it (`with_cond`), the names in the order given; without the conditions
/// a plain list, as the page passes a series of today.
fn files_json(names: &[String], with_cond: bool) -> String {
    let list: Vec<String> = names
        .iter()
        .map(|n| format!(r#"{{"name":"{n}","kind":"full"}}"#))
        .collect();
    if !with_cond {
        return format!("[{}]", list.join(","));
    }
    // the roles, times and injection order are those of the folder (sorted), whatever the order the page lists the files in
    let mut sorted = names.to_vec();
    sorted.sort();
    let req: Vec<String> = sorted
        .iter()
        .map(|n| format!(r#"{{"name":"{n}","sha256":"","kind":"mzML"}}"#))
        .collect();
    let doc = campioni::route("proponi", &format!(r#"{{"files":[{}]}}"#, req.join(","))).unwrap();
    format!(r#"{{"files":[{}],"campioni":{doc}}}"#, list.join(","))
}

fn run(names: &[String], with_cond: bool, settings: &str) -> lr::LrDig {
    let idxs: Vec<Vec<u8>> = names
        .iter()
        .map(|n| index_of_file(&dir().join(n)))
        .collect();
    dig_with(&files_json(names, with_cond), BENTAZONE, settings, &idxs)
}

fn summary_of(d: &lr::LrDig) -> Value {
    serde_json::from_str(&summary(d, "{}")).unwrap()
}

/// The row of the block `gruppi` whose nominal m/z is `mz`.
fn row_of(g: &Value, mz: u32) -> &Value {
    g["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["mz"].as_f64().unwrap().round() as u32 == mz)
        .unwrap_or_else(|| panic!("no row of m/z {mz}"))
}

fn cond_of<'a>(row: &'a Value, c: &str) -> &'a Value {
    row["cond"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == c)
        .unwrap()
}

/// Mean over the replicates of a condition at a time of the expected area of a compound (`attesi.json`).
fn truth(c: &str, t: &str, compound: &str) -> f64 {
    let v: Value =
        serde_json::from_slice(&std::fs::read(dir().join("attesi.json")).unwrap()).unwrap();
    let a: Vec<f64> = ["r1", "r2"]
        .iter()
        .map(|r| {
            v["file"][format!("{c}_{t}_{r}.mzML")]["composti"][compound]["area_attesa"]
                .as_f64()
                .unwrap()
        })
        .collect();
    (a[0] + a[1]) / 2.0
}

fn present_truth(compound: &str) -> Vec<&'static str> {
    CONDS
        .iter()
        .copied()
        .filter(|c| truth(c, "t5", compound) > 0.0 || truth(c, "t10", compound) > 0.0)
        .collect()
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn presence_sets_shapes_and_ratios_follow_the_truth() {
    let d = run(&names(), true, r#"{"gruppi":{"reference":"O2"}}"#);
    let s = summary_of(&d);
    let g = &s["gruppi"];
    assert_eq!(
        strs(
            &g["conditions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["id"].clone())
                .collect()
        ),
        CONDS
    );
    let comp = [(241, "Parent"), (257, "TP1"), (273, "TP2"), (225, "TP3")];
    for (mz, name) in comp {
        let r = row_of(g, mz);
        assert_eq!(
            strs(&r["present_in"]),
            present_truth(name),
            "{name} present_in"
        );
        let n = present_truth(name).len();
        assert_eq!(r["in_all"], n == CONDS.len(), "{name} in_all");
        assert_eq!(!r["only_in"].is_null(), n == 1, "{name} only_in");
        if n >= 2 && n < CONDS.len() {
            let set = g["sets"]
                .as_array()
                .unwrap()
                .iter()
                .find(|x| x["rows"].as_array().unwrap().contains(&r["row"]));
            assert_eq!(
                strs(&set.unwrap()["conditions"]),
                present_truth(name),
                "{name} set"
            );
        }
        // ratio to the reference (O2) at the last time they share: within 10 % of the ratio of the expected areas
        for c in present_truth(name) {
            let (a, o) = (truth(c, "t10", name), truth("O2", "t10", name));
            // the peaks below the floor of the pipeline (the dark control at t10) have no area to compare
            if o > 1.5e5 && a > 1.5e5 && c != "O2" {
                let got = cond_of(r, c)["ratio_ref"].as_f64().unwrap();
                assert!(
                    (got / (a / o) - 1.0).abs() <= 0.10,
                    "{name} {c}: ratio {got} vs {}",
                    a / o
                );
            }
        }
    }
    // the four products drawn: decay (TP1, parent), rising (TP2, TP3 where formed), absent (TP2 in N2, TP3 in O2)
    assert_eq!(cond_of(row_of(g, 257), "Air")["shape"], "decay");
    assert_eq!(cond_of(row_of(g, 241), "Air")["shape"], "decay");
    // two treated times: «rising» needs more than ×1.5 (TP2 in O2 grows ×1.3 between t5 and t10, TP3 ×1.7)
    assert_eq!(cond_of(row_of(g, 273), "O2")["shape"], "flat");
    assert_eq!(cond_of(row_of(g, 225), "He")["shape"], "rising");
    assert_eq!(cond_of(row_of(g, 273), "N2")["present"], false);
    assert_eq!(cond_of(row_of(g, 225), "O2")["present"], false);
    assert!(cond_of(row_of(g, 273), "N2")["shape"].is_null());
    // the controls: the blank and time zero do not have the products, the standard has the parent only
    let p = cond_of(row_of(g, 273), "O2");
    assert_eq!(p["prev"]["absent_blank"], "pass");
    assert_eq!(p["prev"]["absent_t0"], "pass");
    assert_eq!(p["prev"]["in_parent_std"], "no");
    assert_eq!(cond_of(row_of(g, 241), "Air")["prev"], Value::Null);
}

#[test]
fn the_parent_decays_in_the_active_conditions_only() {
    let s = summary_of(&run(&names(), true, "{}"));
    let conds = s["gruppi"]["conditions"].as_array().unwrap();
    let rem = |c: &str| {
        conds.iter().find(|x| x["id"] == c).unwrap()["parent"]["remaining"]
            .as_f64()
            .unwrap()
    };
    assert!(
        rem("Air") < 0.3 && rem("N2") < 0.3,
        "active conditions: {} {}",
        rem("Air"),
        rem("N2")
    );
    assert!(rem("Reference") > 0.9, "dark control: {}", rem("Reference"));
}

#[test]
fn one_condition_leaves_the_output_as_it_was() {
    // the series of one condition: the same bytes with the condition written or not, and no block `gruppi`
    let n: Vec<String> = names()
        .into_iter()
        .filter(|n| n.starts_with("Air_") || n == "bianco.mzML")
        .collect();
    let (a, b) = (
        summary_of(&run(&n, true, "{}")),
        summary_of(&run(&n, false, "{}")),
    );
    assert!(a.get("gruppi").is_none() && b.get("gruppi").is_none());
    assert_eq!(a["rows"], b["rows"]);
    assert_eq!(a["files"].as_array().unwrap().len(), n.len());
}

#[test]
fn a_time_per_condition_never_grows_over_time() {
    // endpoints only (t0 and t10 of each condition): there is nothing to grow along, per condition
    let n: Vec<String> = names()
        .into_iter()
        .filter(|n| !n.contains("_t5_"))
        .collect();
    let s = summary_of(&run(&n, true, "{}"));
    let d = run(&n, true, "{}");
    for e in &d.entries {
        for c in e.criteria.iter().filter(|c| c.name == "grows over time") {
            assert_eq!(c.status, "n/a", "{}: {}", e.name, c.text);
        }
    }
    assert!(s["gruppi"]["rows"].as_array().unwrap().len() > 3);
}

#[test]
fn one_name_per_product_in_every_condition() {
    // the drift of RT between the runs does not split a product: no two rows within 0.5 Da and 0.3 min
    let s = summary_of(&run(&names(), true, "{}"));
    let rows = s["gruppi"]["rows"].as_array().unwrap();
    for (i, a) in rows.iter().enumerate() {
        for b in &rows[i + 1..] {
            let (dm, dt) = (
                (a["mz"].as_f64().unwrap() - b["mz"].as_f64().unwrap()).abs(),
                (a["rt"].as_f64().unwrap() - b["rt"].as_f64().unwrap()).abs(),
            );
            assert!(dm > 0.5 || dt > 0.3, "double row: {a} {b}");
        }
    }
}

/// A fixed shuffle of the names (a generator of the tests).
fn shuffled(mut v: Vec<String>, seed: u64) -> Vec<String> {
    let mut rng = Lcg(seed);
    for i in (1..v.len()).rev() {
        v.swap(i, (rng.next() * (i + 1) as f64) as usize);
    }
    v
}

/// The block without what depends on the order of the columns (the indices of the files).
fn stable(mut g: Value) -> Value {
    for c in g["conditions"].as_array_mut().unwrap() {
        for k in ["files", "controls"] {
            c.as_object_mut().unwrap().remove(k);
        }
    }
    g
}

/// The path of the first difference of two JSON values ("" when equal).
fn first_diff(a: &Value, b: &Value, path: &str) -> String {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                let d = first_diff(v, y.get(k).unwrap_or(&Value::Null), &format!("{path}.{k}"));
                if !d.is_empty() {
                    return d;
                }
            }
            String::new()
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => x
            .iter()
            .zip(y)
            .enumerate()
            .map(|(i, (u, v))| first_diff(u, v, &format!("{path}[{i}]")))
            .find(|d| !d.is_empty())
            .unwrap_or_default(),
        _ if a == b => String::new(),
        _ => format!("{path}: {a} vs {b}"),
    }
}

#[test]
fn the_order_of_the_files_does_not_change_the_block() {
    let first = stable(summary_of(&run(&names(), true, "{}"))["gruppi"].clone());
    for seed in 1..=5 {
        let g = stable(summary_of(&run(&shuffled(names(), seed), true, "{}"))["gruppi"].clone());
        assert_eq!(first_diff(&g, &first, ""), "", "permutation {seed}");
    }
}

#[test]
fn the_other_polarity_pairs_equal_neutral_masses() {
    use lr::gruppi_out::other_polarity;
    use mzlab_feat::gruppi::RowMeta;
    let row = |id: u32, mz: f64, rt: f64, p: &str| RowMeta {
        id,
        mz,
        rt,
        polarity: p.into(),
        ..RowMeta::default()
    };
    // [M+H]+ 257.06 and [M-H]- 255.04 are the same neutral (256.05); a third row is not
    let mut rows = vec![
        row(1, 257.06, 2.2, "+"),
        row(2, 255.045, 2.25, "-"),
        row(3, 273.05, 1.8, "+"),
    ];
    other_polarity(&mut rows, 0.35, 0.25);
    assert_eq!(
        (
            rows[0].other_polarity,
            rows[1].other_polarity,
            rows[2].other_polarity
        ),
        (Some(2), Some(1), None)
    );
}

/// `cargo test --release -p mzlab-finder-lr-wasm --test lr_gruppi -- --ignored --nocapture scale` with `N=20` or `N=40` under `/usr/bin/time -v`:
/// the whole native pipeline (detection, alignment, Dig, block `gruppi`) of N full-scan files of the folder (`docs/agenti/misure.md`).
#[test]
#[ignore]
fn scale() {
    let n: usize = std::env::var("N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let all = names();
    // 20 = three conditions, the blank and the standard; 40 = the folder and two more replicates of Air (the data of Air_t5_r1 and Air_t10_r1)
    let mut files: Vec<(String, String)> = all
        .iter()
        .filter(|f| {
            n > 20
                || f.starts_with("Air_")
                || f.starts_with("Ar_")
                || f.starts_with("He_")
                || !f.contains('_')
        })
        .map(|f| (f.clone(), f.clone()))
        .collect();
    if n > 38 {
        files.push(("Air_t5_r3.mzML".into(), "Air_t5_r1.mzML".into()));
        files.push(("Air_t10_r3.mzML".into(), "Air_t10_r1.mzML".into()));
    }
    assert_eq!(files.len(), n);
    let names: Vec<String> = files.iter().map(|f| f.0.clone()).collect();
    let t = std::time::Instant::now();
    let idxs: Vec<Vec<u8>> = files
        .iter()
        .map(|f| index_of_file(&dir().join(&f.1)))
        .collect();
    let t_idx = t.elapsed();
    let t = std::time::Instant::now();
    let d = dig_with(&files_json(&names, true), BENTAZONE, "{}", &idxs);
    let t_dig = t.elapsed();
    let t = std::time::Instant::now();
    let s = summary(&d, "{}");
    let hwm = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|t| {
            t.lines()
                .find(|l| l.starts_with("VmHWM"))
                .map(str::to_string)
        })
        .unwrap_or_default();
    println!(
        "N={n} {hwm} index {t_idx:?} pipeline+dig {t_dig:?} summary+gruppi {:?} ({} bytes)",
        t.elapsed(),
        s.len()
    );
}
