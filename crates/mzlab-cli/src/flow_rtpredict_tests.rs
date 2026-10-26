//! Plan of `rt.predict` on synthetic alcohols (RT grows by 0.5 min per carbon) and a three-hit candidate list (no lab data).

use std::path::PathBuf;

fn fixture(tag: &str, n_standards: usize) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("mzlab_cli_rtpredict_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let mut std_csv = String::from("smiles,rt\n");
    for n in 1..=n_standards {
        let wobble = if n % 2 == 0 { 0.02 } else { -0.02 };
        std_csv += &format!("{}O,{}\n", "C".repeat(n), 1.0 + 0.5 * n as f64 + wobble);
    }
    std::fs::write(dir.join("standards.csv"), std_csv).unwrap();
    // the right one first in the file is the far one: ethanol cannot elute at 6 min on this series
    let hits = r#"{"format":"candidates/1","hits":[
        {"query":"q0","id":"far","name":"Ethanol","smiles":"CCO","score":0.9},
        {"query":"q0","id":"near","name":"Decanol","smiles":"CCCCCCCCCCO","score":0.5},
        {"query":"q0","id":"bad","name":"Unreadable","smiles":"C((","score":0.1}]}"#;
    std::fs::write(dir.join("hits.json"), hits).unwrap();
    std::fs::write(dir.join("q.csv"), "query,rt\nq0,6.0\n").unwrap();
    dir
}

fn run_plan(
    dir: &std::path::Path,
    params: &str,
    algorithm: &str,
) -> Result<mzlab_core::json::Value, String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"hits.json","type":"candidates"}},{{"name":"standards.csv","type":"table"}},{{"name":"q.csv","type":"table"}}],"steps":[
        {{"id":"r","step":"rt.predict@1","algorithm":"{algorithm}","in":{{"candidates":"file:hits.json","standards":"file:standards.csv","queries":"file:q.csv"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), dir, &dir.join("work"), 1, None)
        .map_err(|e| format!("{e:?}"))?;
    if !run.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(mzlab_core::json::parse(&std::fs::read(dir.join("work/r.json")).unwrap()).unwrap())
}

fn ids(v: &mzlab_core::json::Value) -> Vec<String> {
    v.get("hits")
        .unwrap()
        .arr()
        .iter()
        .map(|h| h.get("id").unwrap().str().unwrap().to_string())
        .collect()
}

/// The window keeps the decanol, flags the ethanol, leaves the unreadable one without a verdict; `order` puts the best residual first, `drop_outside` removes the flagged one.
#[test]
fn logp_window_checks_orders_and_drops() {
    let dir = fixture("window", 20);
    let v = run_plan(&dir, "{}", "logp_window").unwrap();
    let hit = |id: &str| {
        v.get("hits")
            .unwrap()
            .arr()
            .iter()
            .find(|h| h.get("id").unwrap().str() == Some(id))
            .unwrap()
            .clone()
    };
    assert_eq!(hit("near").get("rt_ok").unwrap().bool(), Some(true));
    assert_eq!(hit("far").get("rt_ok").unwrap().bool(), Some(false));
    assert!(hit("far").get("rt_resid").unwrap().f64().unwrap() > 3.0);
    assert!(
        hit("bad").get("rt_ok").unwrap().is_null() && hit("bad").get("rt_pred").unwrap().is_null()
    );
    assert_eq!(
        hit("far").get("name").unwrap().str(),
        Some("Ethanol"),
        "the fields of the hit stay"
    );
    assert_eq!(ids(&v), ["far", "near", "bad"]);
    assert_eq!(
        ids(&run_plan(&dir, r#"{"order":"residual"}"#, "logp_window").unwrap()),
        ["near", "far", "bad"]
    );
    assert_eq!(
        ids(&run_plan(&dir, r#"{"drop_outside":true}"#, "logp_window").unwrap()),
        ["near", "bad"]
    );
    let again = std::fs::read(dir.join("work/r.json")).unwrap();
    run_plan(&dir, r#"{"drop_outside":true}"#, "logp_window").unwrap();
    assert_eq!(again, std::fs::read(dir.join("work/r.json")).unwrap());
    let t = run_plan(&dir, r#"{"rt_logp_terms":true}"#, "logp_window").unwrap();
    assert!(hit_terms(&t));
    let _ = std::fs::remove_dir_all(&dir);
}

fn hit_terms(v: &mzlab_core::json::Value) -> bool {
    v.get("hits")
        .unwrap()
        .arr()
        .iter()
        .any(|h| h.get("score_terms").is_some())
}

/// With a parent the hits get the change of logP and the direction of the RT change; the parent's RT is needed for the direction.
#[test]
fn parent_gives_delta_logp_and_direction() {
    let dir = fixture("parent", 20);
    let v = run_plan(
        &dir,
        r#"{"parent":"CCCCCO","parent_rt":3.5}"#,
        "logp_window",
    )
    .unwrap();
    let near = v
        .get("hits")
        .unwrap()
        .arr()
        .iter()
        .find(|h| h.get("id").unwrap().str() == Some("near"))
        .unwrap();
    assert!(near.get("delta_logp").unwrap().f64().unwrap() > 1.0);
    assert_eq!(near.get("rt_dir").unwrap().str(), Some("less_polar"));
    assert_eq!(near.get("rt_dir_agrees").unwrap().bool(), Some(true));
    let no_rt = run_plan(&dir, r#"{"parent":"CCCCCO"}"#, "logp_window").unwrap();
    let h = &no_rt.get("hits").unwrap().arr()[1];
    assert!(h.get("delta_logp").is_some() && h.get("rt_dir").is_none());
    assert!(run_plan(&dir, r#"{"parent":"C(("}"#, "logp_window").is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The forest predicts and gives the residual but has no window; too few standards stop either algorithm.
#[test]
fn forest_and_the_standards_it_needs() {
    let dir = fixture("forest", 20);
    let v = run_plan(&dir, "{}", "forest").unwrap();
    let near = &v.get("hits").unwrap().arr()[1];
    assert!(near.get("rt_pred").unwrap().f64().is_some() && near.get("rt_ok").is_none());
    assert!(run_plan(&dir, r#"{"drop_outside":true}"#, "forest").is_err());
    let few = fixture("few", 5);
    assert!(run_plan(&few, "{}", "forest").is_err());
    assert!(run_plan(&few, "{}", "logp_window").is_err());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&few);
}
