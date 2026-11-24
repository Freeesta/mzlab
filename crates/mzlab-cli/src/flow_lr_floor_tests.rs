//! Rule R1 (one-ion noise floor) in `tp.dig_lr` on the demo full-scan files: the plan with `one_ion` set gives the page's bytes, the floor is
//! evidence only (no state, score, label or criterion moves with it) and the floor of a cell is `mzlab-cli peak noise` on the XIC of that cell.

use super::*;
use mzlab_core::ionstat::floor_json;
use serde_json::Value;

const DIG: &str = r#",{"id":"dig","step":"tp.dig_lr@1","in":{"table":"gap.table","samples":"file:samples.json","parent":"file:parent.json","index":["idx.index"]},"params":{"one_ion":13333.3336,"noise_sd_weight":1}}"#;

fn json(dg: &lr::LrDig) -> Value {
    serde_json::from_str(&summary(dg, "{}")).unwrap()
}

/// The Dig without the keys of the floor: what must not move when `one_ion` changes.
fn without_floor(mut v: Value) -> Value {
    let o = v.as_object_mut().unwrap();
    o.remove("noise_floor");
    o.remove("settings");
    for r in o["rows"].as_array_mut().unwrap() {
        r.as_object_mut().unwrap().remove("checks");
        for p in r["points"].as_array_mut().unwrap() {
            p.as_object_mut().unwrap().remove("sn_floor");
        }
    }
    v
}

#[test]
fn a_plan_that_sets_one_ion_gives_the_bytes_of_the_page() {
    let d = fixture_with("dig_floor", None, &[]);
    run_flow(&d, DIG);
    let (_, dg, _) = page_dig(&d, r#"{"one_ion": 13333.3336, "noise_sd_weight": 1}"#);
    let page = summary(&dg, "{}") + "\n";
    let got = std::fs::read_to_string(d.join("work/dig.json")).unwrap();
    assert!(got == page, "dig JSON differs: flow {} bytes, page {}", got.len(), page.len());
    let v: Value = serde_json::from_str(&got).unwrap();
    assert_eq!(v["settings"]["one_ion"], 13333.3336);
    assert_eq!(v["noise_floor"]["one_ion"], 13333.3336);
    assert_eq!(v["noise_floor"]["sd_weight"], 1.0);
    let parent = &v["rows"][0]["checks"][0];
    assert_eq!(parent["id"], "noise_floor");
    assert_eq!(parent["evidence"]["terms"]["one_ion_source"], "set", "{parent}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_noise_floor_is_evidence_only_and_auto_finds_the_ion_of_the_demo_files() {
    let d = fixture_with("dig_floor_only", None, &[]);
    let (_, auto, _) = page_dig(&d, "{}");
    let auto = json(&auto);
    let nf = &auto["noise_floor"];
    assert_eq!(nf["one_ion"], "auto");
    assert_eq!(nf["evidence_only"], true);
    let (cells, accepted) = (nf["cells"].as_i64().unwrap(), nf["quantum_accepted"].as_i64().unwrap());
    assert!(accepted * 10 >= cells * 9, "the demo files count ions: {accepted} of {cells}");
    assert!(nf["log"][0].as_str().unwrap().starts_with("noise floor R1 (evidence only"), "{nf}");
    let parent = &auto["rows"][0];
    assert_eq!(parent["checks"][0]["status"], "pass", "{parent}");
    let q = parent["checks"][0]["evidence"]["quantum"]["q"].as_f64().unwrap();
    assert!((q - 13333.33).abs() < 1.0, "one ion of the demo instrument: {q}");
    for p in parent["points"].as_array().unwrap().iter().filter(|p| p["state"] == "detected") {
        assert!(p["sn_floor"].as_f64().unwrap() > 0.0, "{p}");
    }
    for settings in [r#"{"one_ion": 0}"#, r#"{"one_ion": 1e9}"#, r#"{"noise_sd_weight": 3}"#] {
        let (_, other, _) = page_dig(&d, settings);
        let other = json(&other);
        assert!(without_floor(other.clone()) == without_floor(auto.clone()), "{settings} moved a result");
        if settings.contains("\"one_ion\": 0") {
            let st: Vec<&str> = other["rows"].as_array().unwrap().iter().map(|r| r["checks"][0]["status"].as_str().unwrap()).collect();
            assert!(st.iter().all(|s| *s == "warn" || *s == "n/a"), "off: no ion term, never pass: {st:?}");
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_floor_of_a_cell_is_peak_noise_on_its_xic() {
    let d = fixture_with("dig_floor_cli", None, &[]);
    let (_, dg, idxs) = page_dig(&d, "{}");
    let r = dg.entries[0].row.unwrap();
    let mut n = 0;
    for (k, idx) in idxs.iter().enumerate() {
        let Some(f) = dg.checks.floor[dg.table.cell(r, k)].as_ref() else {
            continue;
        };
        let mut ix = Index::open(idx.clone(), idx.len() as u64).unwrap();
        let tol = dg.settings.tol_da;
        let (rt, y) = lr::detail::xic(&mut ix, dg.table.mz[r] - tol, dg.table.mz[r] + tol, 0.0).unwrap();
        let csv = d.join(format!("xic{k}.csv"));
        let body: Vec<String> = rt.iter().zip(&y).map(|(a, b)| format!("{a},{b}")).collect();
        std::fs::write(&csv, format!("rt,y\n{}\n", body.join("\n"))).unwrap();
        let (a, b) = f.ask.peak.unwrap();
        let o = crate::peak_noise_cmd::run(vec![csv.to_string_lossy().into(), "--rt".into(), format!("{a},{b}")]);
        assert_eq!(o.code, 0, "{}", o.stderr);
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{}\n", floor_json(f)), "file {k}");
        n += 1;
    }
    assert!(n >= 4, "the parent is checked in the samples: {n}");
    let _ = std::fs::remove_dir_all(&d);
}
