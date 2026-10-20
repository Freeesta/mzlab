//! `corsa/1` on a hand-written `prov/1`: files and lists apart, estimated against fixed parameters, level counts and the included `report/1`.

use super::*;

const PROV: &str = r#"{"schema":"prov/1","agent":{"name":"mzLab","version":"1.2.3","platform":"native"},"plan_sha256":"aa",
"entity":{"file:a.mzML":{"sha256":"11","bytes":10,"type":"mzml"},"file:lib.msp":{"sha256":"22","bytes":5,"type":"library"},"d.table":{"sha256":"33","type":"table"}},
"activity":{"d":{"step":"features.detect@1","algorithm":"masstrace@1","preset":"hr","params":{"floor":20000,"min_height":200000},"params_sha256":"p1","estimates":{"floor":{"hist":[1,2]}},"used":[],"generated":[],"ms":12.5},
"m":{"step":"spectra.match@1","params":{},"params_sha256":"p2","used":[],"generated":[],"ms":3,"error":{"error_key":"err.x","params":{}}}}}"#;

const ANNOT: &str = r#"{"format":"annot/1","rows":[{"id":"q0","level":"2a"},{"id":"q1","level":"3a"},{"id":"q2","level":"2a"},{"id":"q3","level":"1","auto":false}]}"#;
const REPORT: &str = r#"{"schema":"report/1","rows":4}"#;

fn corsa() -> Corsa {
    let prov = parse(PROV.as_bytes()).unwrap();
    let u = [
        Uscita {
            name: "annot.json",
            bytes: ANNOT.as_bytes(),
        },
        Uscita {
            name: "report.json",
            bytes: REPORT.as_bytes(),
        },
        Uscita {
            name: "notes.json",
            bytes: b"not json",
        },
    ];
    build(&prov, &u).unwrap()
}

#[test]
fn reads_inputs_lists_steps_and_parameters() {
    let c = corsa();
    assert_eq!(c.inputs.len(), 1);
    assert_eq!(c.inputs[0].name, "a.mzML");
    assert_eq!(c.lists.len(), 1);
    assert_eq!(c.lists[0].sha256, "22");
    assert_eq!(c.steps[0].status, "ok");
    assert_eq!(c.steps[1].status, "error");
    assert_eq!(c.steps[1].error_key.as_deref(), Some("err.x"));
    let floor = &c.steps[0].params[0];
    assert_eq!(
        (floor.name.as_str(), floor.proof.is_some()),
        ("floor", true)
    );
    assert!(c.steps[0].params[1].proof.is_none());
}

#[test]
fn counts_levels_auto_apart() {
    let l = corsa().levels;
    assert_eq!((l.files, l.rows), (1, 4));
    assert_eq!(l.auto, vec![("2a".into(), 2), ("3a".into(), 1)]);
    assert_eq!(l.other, vec![("1".into(), 1)]);
}

#[test]
fn json_is_stable_and_valid() {
    let a = corsa().json();
    assert_eq!(a, corsa().json());
    let v = parse(a.as_bytes()).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("corsa/1"));
    assert_eq!(v.get("total_ms").unwrap().f64(), Some(15.5));
    assert_eq!(
        v.get("report").unwrap().get("rows").unwrap().f64(),
        Some(4.0)
    );
    let p = &v.get("steps").unwrap().arr()[0]
        .get("params")
        .unwrap()
        .arr()[0];
    assert_eq!(p.get("state").unwrap().str(), Some("estimated"));
}

#[test]
fn refuses_what_is_not_prov() {
    let v = parse(b"{\"schema\":\"x\"}").unwrap();
    assert!(build(&v, &[])
        .unwrap_err()
        .to_string()
        .contains("err.corsa.prov"));
}
