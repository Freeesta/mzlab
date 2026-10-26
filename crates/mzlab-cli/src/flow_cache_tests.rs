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

    // a caller told after each job sees every job once, all of them reused here
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(String, bool)>::new()));
    let log = seen.clone();
    let done: crate::flow::DoneFn = std::sync::Arc::new(move |id, _, _, cached, err| {
        assert!(err.is_none());
        log.lock().unwrap().push((id.to_string(), cached));
    });
    crate::flow::run_plan_notify(&text, &data, &out, 1, None, Some(false), Some(done)).unwrap();
    let seen = seen.lock().unwrap();
    assert!(seen.len() >= 4 && seen.iter().all(|x| x.1), "{seen:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_added_in_the_middle_does_not_run_the_others_again() {
    let (dir, files) = fixture("cache_middle");
    let (data, out) = (dir.join("data"), dir.join("out"));
    std::fs::create_dir_all(&out).unwrap();
    let base = std::fs::read_to_string(dir.join("plan.json")).unwrap();
    let run = |text: &str, log: &std::sync::Arc<std::sync::Mutex<Vec<(String, bool)>>>| {
        let log = log.clone();
        let done: crate::flow::DoneFn = std::sync::Arc::new(move |id, _, _, cached, err| {
            assert!(err.is_none(), "{err:?}");
            log.lock().unwrap().push((id.to_string(), cached));
        });
        crate::flow::run_plan_notify(
            text.as_bytes(),
            &data,
            &out,
            1,
            None,
            Some(false),
            Some(done),
        )
        .unwrap();
    };
    let first = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    run(&base, &first);
    assert!(first.lock().unwrap().iter().all(|x| !x.1));

    // a fourth file between the second and the third: the indexes of the other three come from the cache
    std::fs::write(data.join("s_t020.mzML"), crate::tests::mzml(20.0)).unwrap();
    assert_eq!(files.len(), 3);
    let wider = base
        .replace(
            r#"{"name":"s_t030.mzML","type":"raw"}"#,
            r#"{"name":"s_t020.mzML","type":"raw"},{"name":"s_t030.mzML","type":"raw"}"#,
        )
        .replace(
            r#""file:s_t010.mzML","file:s_t030.mzML""#,
            r#""file:s_t010.mzML","file:s_t020.mzML","file:s_t030.mzML""#,
        );
    assert_ne!(wider, base);
    let second = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    run(&wider, &second);
    let idx: Vec<bool> = second
        .lock()
        .unwrap()
        .iter()
        .filter(|x| x.0 == "idx")
        .map(|x| x.1)
        .collect();
    assert_eq!(idx.len(), 4);
    assert_eq!(idx.iter().filter(|c| **c).count(), 3, "{idx:?}");
    assert!(out.join("idx-s_t020.mzidx").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_probe_counts_the_steps_the_cache_would_give_back() {
    let (dir, _) = fixture("cache_probe");
    let (data, out) = (dir.join("data"), dir.join("out"));
    std::fs::create_dir_all(&out).unwrap();
    let text = std::fs::read(dir.join("plan.json")).unwrap();
    // nothing is cached yet: every step is to compute, and the probe leaves no index behind
    assert_eq!(crate::flow::probe_plan(&text, &data, &out).unwrap(), (0, 4));
    assert!(!out.join("cache").join("index.json").exists());

    crate::flow::run_plan_cached(&text, &data, &out, 1, None, Some(false)).unwrap();
    let index = std::fs::read(out.join("cache").join("index.json")).unwrap();
    assert_eq!(crate::flow::probe_plan(&text, &data, &out).unwrap(), (4, 0));

    // a parameter of align: index and detect are reused, align and gapfill are to compute
    let changed = String::from_utf8(text).unwrap().replace(
        r#""step":"features.align@1","#,
        r#""step":"features.align@1","params":{"drop_artefacts":false},"#,
    );
    assert!(changed.contains("drop_artefacts"));
    assert_eq!(
        crate::flow::probe_plan(changed.as_bytes(), &data, &out).unwrap(),
        (2, 2)
    );
    assert_eq!(
        std::fs::read(out.join("cache").join("index.json")).unwrap(),
        index
    );
    let _ = std::fs::remove_dir_all(&dir);
}
