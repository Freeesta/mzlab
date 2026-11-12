//! `structure.moiety@1` in a plan: the CSV `flow run` writes is the table of the shared fixture `tests/dati/moiety.json` (the bytes the `molkit` `.wasm`
//! gives in `tests/test_molkit_wasm.py`), for each set of parameters, and a wrong moiety stops the job with its key.

use std::path::{Path, PathBuf};

use mzlab_core::json::{self, Value};

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_moiety_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    d
}

fn fixture() -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/dati/moiety.json");
    json::parse(&std::fs::read(p).unwrap()).unwrap()
}

fn plan(params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"list.csv","type":"candidates"}}],"steps":[{{"id":"m","step":"structure.moiety@1","in":{{"candidates":"file:list.csv"}},"params":{params}}}]}}"#
    )
}

#[test]
fn moiety_in_a_plan_gives_the_tables_of_the_fixture() {
    let (d, f) = (dir("cases"), fixture());
    std::fs::write(
        d.join("list.csv"),
        f.get("table").and_then(|t| t.str()).unwrap(),
    )
    .unwrap();
    for c in f.get("cases").unwrap().arr() {
        let params = c.get("params").and_then(|p| p.str()).unwrap();
        let (run, prov) =
            crate::flow::run_plan(plan(params).as_bytes(), &d, &d.join("work"), 1, None).unwrap();
        assert!(
            run.skipped.is_empty() && !prov.contains("\"error\""),
            "{prov}"
        );
        let got = std::fs::read_to_string(d.join("work/m.csv")).unwrap();
        assert_eq!(got, c.get("out").and_then(|o| o.str()).unwrap(), "{params}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_empty_moiety_stops_the_job_with_its_key() {
    let (d, f) = (dir("empty"), fixture());
    std::fs::write(
        d.join("list.csv"),
        f.get("table").and_then(|t| t.str()).unwrap(),
    )
    .unwrap();
    let out = crate::flow::run_plan(plan("{}").as_bytes(), &d, &d.join("work"), 1, None);
    let text = match out {
        Ok((_, prov)) => prov,
        Err(e) => e.key.to_string(),
    };
    assert!(text.contains("error.sub.query"), "{text}");
    let _ = std::fs::remove_dir_all(&d);
}
