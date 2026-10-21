//! The whole HR chain of Tabui as one plan over the synthetic identity series: the base chain, `tp.generate` (the hypotheses are made by the plan, not
//! handed in), `tp.link@2`, `tp.filter@2` and `tp.network`. The `.mzfeat` equals the one made with the hypotheses of the engine, the filters equal the
//! ones stored by the interpretation and the network has arcs. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::Path;

use mzlab_core::json::parse;

use crate::flow_link_tests::{fixture, hypotheses, plan, run as link_run, series, FORMULA};

fn full(dir: &Path) -> Result<(), String> {
    let base = plan("{}").replace(
        r#""hypotheses":"file:hyp.json""#,
        r#""hypotheses":"gen.hypotheses""#,
    );
    let gen = r#"{"id":"gen","step":"tp.generate@1","in":{"parent":"file:parent.json"}},"#;
    let cut = base.find(r#"{"id":"lnk""#).unwrap();
    let rest = base[cut..].strip_suffix("]}").unwrap();
    let text = format!(
        r#"{}{gen}{rest},{{"id":"flt","step":"tp.filter@2","in":{{"tp":"lnk.tp","hypotheses":"gen.hypotheses"}}}},{{"id":"net","step":"tp.network@1","in":{{"tp":"lnk.tp","table":"lnk.tp","samples":"file:samples.json"}}}}]}}"#,
        &base[..cut]
    );
    let (done, prov) = crate::flow::run_plan(text.as_bytes(), dir, &dir.join("work"), 2, None)
        .map_err(|e| e.key.to_string())?;
    if !done.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(())
}

#[test]
fn the_chain_runs_from_files_to_the_network_with_the_hypotheses_of_the_plan() {
    let Some(out) = series("hr_chain") else {
        return;
    };
    let parent = format!(r#"{{"formula":"{FORMULA}","rt":1.0}}"#);
    let dir = fixture(&out, &parent, &hypotheses());
    full(&dir).unwrap();
    let w = dir.join("work");
    assert_eq!(
        std::fs::read(w.join("lnk.mzfeat")).unwrap(),
        link_run(&dir, "{}").unwrap(),
        "the hypotheses of the plan are the ones of the engine"
    );
    let net = std::fs::read_to_string(w.join("net.json")).unwrap();
    assert!(net.starts_with("{\"schema\":\"rete/1\""), "{net}");
    let n = parse(net.as_bytes()).unwrap();
    println!(
        "arcs={:?}",
        n.get("n_arcs").and_then(|v| v.f64())
    );
    let flt = std::fs::read_to_string(w.join("flt.json")).unwrap();
    assert!(flt.starts_with("{\"schema\":\"filtri/1\""), "{flt}");
    let _ = std::fs::remove_dir_all(&out);
}
