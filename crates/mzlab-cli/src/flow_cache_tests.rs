//! The cache of a run folder: running the same plan again in the same `--out` takes every step from the cache (`cached` in `prov/1`, `reused` on
//! stderr) with the same bytes; a changed parameter runs that step and what follows it only; `--fresh` runs everything; the desktop route keeps it too.

use std::path::Path;

use mzlab_core::json::{parse, Value};

use crate::flow_run_tests::{fixture, flow_run, folder, p};

/// `instance -> cached` of the `prov.json` of `out` (`activity` is keyed by instance).
fn cached(out: &Path) -> Vec<(String, bool)> {
    let prov = parse(&std::fs::read(out.join("prov.json")).unwrap()).unwrap();
    let Some(Value::Obj(acts)) = prov.get("activity").cloned() else {
        panic!("no activity in prov.json")
    };
    let flag = |a: &Value| a.get("cached").and_then(Value::bool).unwrap_or(false);
    let list: Vec<(String, bool)> = acts.iter().map(|(k, a)| (k.clone(), flag(a))).collect();
    assert_eq!(list.len(), 4);
    list
}

/// The files the steps made (`plan.json`, `prov.json` and `corsa.json` aside).
fn made(out: &Path) -> Vec<(String, Vec<u8>)> {
    let skip = ["plan.json", "prov.json", "corsa.json"];
    folder(out)
        .into_iter()
        .filter(|(n, _)| !skip.contains(&n.as_str()))
        .collect()
}

#[test]
fn a_second_run_in_the_same_folder_reuses_every_step() {
    let (dir, _) = fixture("cache");
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let run = |extra: &[&str]| {
        let mut a = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
        a.extend(extra);
        let o = flow_run(&a);
        assert_eq!(o.code, 0, "{}", o.stderr);
    };
    run(&[]);
    let first = made(&out);
    assert!(cached(&out).iter().all(|(_, c)| !c));
    assert!(out.join("cache").join("index.json").is_file());

    // the same plan again: every step from the cache, the same bytes
    run(&[]);
    assert!(cached(&out).iter().all(|(_, c)| *c), "{:?}", cached(&out));
    assert_eq!(made(&out), first);

    // a parameter of align: index and detect reused, align and gapfill run again
    run(&["--set", "ali.drop_artefacts=false"]);
    let c = cached(&out);
    let of = |id: &str| c.iter().find(|x| x.0 == id).unwrap().1;
    assert!(of("idx") && of("det") && !of("ali") && !of("gap"), "{c:?}");

    // back to the plan as written: --fresh runs everything and gives the bytes of the first run
    run(&["--fresh"]);
    assert!(cached(&out).iter().all(|(_, c)| !c));
    assert_eq!(made(&out), first);

    // a file of the folder changed by hand is not taken again
    std::fs::write(out.join("gap.mzfeat"), b"changed").unwrap();
    run(&[]);
    let c = cached(&out);
    assert!(
        !c.iter().find(|x| x.0 == "gap").unwrap().1 && c.iter().find(|x| x.0 == "ali").unwrap().1,
        "{c:?}"
    );
    assert_eq!(made(&out), first);

    // the route of the desktop keeps the cache of its folder too
    let text = std::fs::read(out.join("plan.json")).unwrap();
    let (_, prov) = crate::flow::run_plan_cached(&text, &data, &out, 1, None, Some(false)).unwrap();
    assert!(
        !prov.contains("\"error\"") && prov.matches("\"cached\":true").count() == 4,
        "{prov}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
