//! `tp.filter@2` in a plan after `tp.link` over the synthetic identity series: with the default thresholds the registry `filtri/1` is the one the
//! interpretation stored in the meta of the `.mzfeat`; the decoy thresholds and the tolerance reach the answer; a table that was not interpreted is refused.
//! Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::Path;

use mzlab_core::json::parse;
use mzlab_feat::interp::store;
use mzlab_feat::store::Slice;

use crate::flow_link_tests::{fixture, hypotheses, plan, series, FORMULA};

fn run(dir: &Path, params: &str) -> Result<String, String> {
    let base = plan("{}");
    let step = format!(
        r#",{{"id":"flt","step":"tp.filter@2","in":{{"tp":"lnk.tp","hypotheses":"file:hyp.json"}},"params":{params}}}]}}"#
    );
    let text = format!("{}{step}", base.strip_suffix("]}").unwrap());
    let (done, prov) = crate::flow::run_plan(text.as_bytes(), dir, &dir.join("work"), 2, None)
        .map_err(|e| e.key.to_string())?;
    if !done.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read_to_string(dir.join("work/flt.json")).unwrap())
}

fn decoys(text: &str) -> (f64, f64) {
    let d = parse(text.as_bytes()).unwrap();
    let d = d.get("decoys").unwrap().clone();
    let n = |k: &str| d.get(k).and_then(|v| v.f64()).unwrap();
    (n("rules"), n("survived"))
}

#[test]
fn the_default_thresholds_give_the_registry_the_interpretation_stored() {
    let Some(out) = series("refilter") else {
        return;
    };
    let parent = format!(r#"{{"formula":"{FORMULA}","rt":1.0}}"#);
    let dir = fixture(&out, &parent, &hypotheses());
    let got = run(&dir, "{}").unwrap();
    assert!(got.starts_with("{\"schema\":\"filtri/1\""), "{got}");

    let tp = std::fs::read(dir.join("work/lnk.mzfeat")).unwrap();
    let (meta, _) = store::read_meta(&mut Slice(&tp), tp.len() as u64)
        .unwrap()
        .unwrap();
    let stored = format!("{:?}", meta.get("filtri").unwrap());
    assert_eq!(format!("{:?}", parse(got.as_bytes()).unwrap()), stored);

    // a second run gives the same bytes; the decoy thresholds reach the count of decoys
    assert_eq!(got, run(&dir, "{}").unwrap());
    let (rules, _) = decoys(&got);
    let one = run(&dir, r#"{"decoy_k":1}"#).unwrap();
    assert!(decoys(&one).0 < rules, "one decoy per rule is fewer decoys");
    // a tolerance other than the default is read from the calibration of the files
    assert!(run(&dir, r#"{"tol_min":1,"tol_max":2}"#).is_ok());
    let _ = std::fs::remove_dir_all(&out);
}
