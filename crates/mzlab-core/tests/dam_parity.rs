//! The Rust reader of the methods (`.dam`: OLE2, binary streams, Analyst XML), their checks against a file, the gradient parsers and the analog helpers give what
//! `mzlab.reader.methodinfo`, `mzlab.analog` and `App.method` give (`tools/dam_riferimento.py`): same texts and counts, numbers within 1e-9. Skipped without
//! python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::analog::{detector_delay, pressure_to_bar, reduce_minmax};
use mzlab_core::gradient::{from_dam, model_from_json, Gradient};
use mzlab_core::json::{self, Out};
use mzlab_core::labmethods::{method_json, warnings_json, Lab};
use mzlab_core::methodcheck::{check_against, method_kind, write_rows};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

/// Equal JSON: same keys (any order), same texts, numbers within 1e-9. `null` and a missing key are the same thing.
fn same(what: &str, got: &Value, want: &Value) {
    match (got, want) {
        (Value::Object(a), Value::Object(b)) => {
            for k in a.keys().chain(b.keys()) {
                same(&format!("{what}.{k}"), a.get(k).unwrap_or(&Value::Null), b.get(k).unwrap_or(&Value::Null));
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{what}: length {got} against {want}");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(&format!("{what}[{i}]"), x, y);
            }
        }
        (Value::Number(a), Value::Number(b)) => assert!(close(a.as_f64().unwrap(), b.as_f64().unwrap()), "{what}: {a} against {b}"),
        _ => assert_eq!(got, want, "{what}"),
    }
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("not JSON ({e}): {text}"))
}

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab-dam-{}", std::process::id()));
    match Command::new("python3").current_dir(root()).arg("tools/dam_riferimento.py").arg(&out).output() {
        Ok(o) if o.status.success() => {}
        other => {
            if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
                panic!("the reference did not run: {other:?}");
            }
            eprintln!("python3/numpy not available: test skipped");
            return None;
        }
    }
    let v = parsed(&std::fs::read_to_string(out.join("riferimento.json")).unwrap());
    Some((out, v))
}

fn lab(dir: &Path, name: &str) -> Lab {
    Lab::read(name, std::fs::read(dir.join(name)).unwrap())
}

fn text(g: &Gradient) -> Value {
    let mut o = Out::default();
    g.write(&mut o);
    parsed(&o.0)
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect()
}

/// A gradient result of the reference (`{"ok": model}` or `{"error_key", "params"}`) against ours.
fn gradient_case(what: &str, got: mzlab_core::Result<Option<Gradient>>, want: &Value) {
    match (got, want.get("ok")) {
        (Ok(Some(g)), Some(ok)) => same(what, &text(&g), ok),
        (Ok(None), Some(ok)) => assert!(ok.is_null(), "{what}: no gradient against {ok}"),
        (Err(e), None) => {
            assert_eq!(e.key, want["error_key"].as_str().unwrap(), "{what}");
            for (k, v) in &e.params {
                let w = &want["params"][*k];
                let eq = v == w.as_str().unwrap_or("") || v.parse::<f64>().ok().zip(w.as_str().and_then(|s| s.parse::<f64>().ok())).is_some_and(|(a, b)| a == b);
                assert!(eq, "{what}: parameter {k}: {v} against {w}");
            }
        }
        (g, _) => panic!("{what}: {:?} against {want}", g.map(|x| x.is_some())),
    }
}

#[test]
fn methods_match_python() {
    let Some((dir, r)) = reference() else { return };
    for f in r["files"].as_array().unwrap() {
        let name = f["file"].as_str().unwrap();
        let l = lab(&dir, name);
        let mut o = Out::default();
        l.write_listed(std::fs::metadata(dir.join(name)).unwrap().len(), &mut o);
        same(&format!("{name} summary"), &parsed(&o.0)["info"], &f["summary"]);
        let Ok(m) = &l.res else {
            assert!(f.get("method").is_none(), "{name}: Python reads it, Rust does not: {:?}", l.res.as_ref().err());
            continue;
        };
        let mut o = Out::default();
        m.write_json(&mut o);
        same(&format!("{name} method"), &parsed(&o.0), &f["method"]);
        assert_eq!(method_kind(m), f["kind"].as_str(), "{name}: kind");
        gradient_case(&format!("{name} gradient"), from_dam(m.lc.as_ref()), &f["gradient"]);
        for (i, d) in r["data"].as_array().unwrap().iter().enumerate() {
            let data = json::parse(d.to_string().as_bytes()).unwrap();
            let mut o = Out::default();
            write_rows(&check_against(&data, m), &mut o);
            same(&format!("{name} check {i}"), &parsed(&o.0), &f["checks"][i]);
        }
    }
}

#[test]
fn method_route_and_warnings_match_python() {
    let Some((dir, r)) = reference() else { return };
    for (i, c) in r["methods"].as_array().unwrap().iter().enumerate() {
        if let Some(have) = c.get("warn") {
            let all: Vec<Lab> = r["files"].as_array().unwrap().iter().map(|f| lab(&dir, f["file"].as_str().unwrap())).collect();
            let have: Vec<String> = have.as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
            same(&format!("warnings {i}"), &parsed(&warnings_json(&all, &have)), &c["result"]);
            continue;
        }
        let labs: Vec<Lab> = c["labs"].as_array().unwrap().iter().map(|n| lab(&dir, n.as_str().unwrap())).collect();
        let got = method_json(&c["data"].to_string(), &labs, c["file"].as_str().unwrap()).unwrap();
        same(&format!("method {i} ({})", c["file"]), &parsed(&got), &c["result"]);
    }
}

#[test]
fn gradient_parsers_match_python() {
    let Some((_, r)) = reference() else { return };
    for (i, g) in r["gradients"].as_array().unwrap().iter().enumerate() {
        let what = format!("gradient {i} ({})", g["fmt"]);
        let got = match g["fmt"].as_str().unwrap() {
            "chromeleon" => mzlab_core::chromeleon::parse(g["text"].as_str().unwrap()),
            "csv" => mzlab_core::gradient::parse_csv(g["text"].as_str().unwrap()),
            "course" => mzlab_core::gradient::course(),
            _ => model_from_json(&json::parse(g["model"].to_string().as_bytes()).unwrap()),
        };
        gradient_case(&what, got.map(Some), &g["result"]);
    }
}

#[test]
fn analog_helpers_match_python() {
    let Some((_, r)) = reference() else { return };
    for (i, c) in r["analog"]["pressure"].as_array().unwrap().iter().enumerate() {
        let (y, fixed) = pressure_to_bar(&nums(&c["y"]), c["unit"].as_str().unwrap());
        let want = &c["result"];
        let w = nums(&want[0]);
        assert_eq!(y.len(), w.len());
        assert!(y.iter().zip(&w).all(|(a, b)| close(*a, *b) || (a.is_nan() && b.is_nan())), "pressure {i}");
        assert_eq!(fixed, !want[1].is_null(), "pressure {i}: note");
    }
    for (i, c) in r["analog"]["reduce"].as_array().unwrap().iter().enumerate() {
        let (t, y) = reduce_minmax(&nums(&c["t"]), &nums(&c["y"]), c["n"].as_u64().unwrap() as usize);
        same(&format!("reduce {i} t"), &serde_json::json!(t), &c["result"][0]);
        same(&format!("reduce {i} y"), &serde_json::json!(y), &c["result"][1]);
    }
    for (i, c) in r["analog"]["delay"].as_array().unwrap().iter().enumerate() {
        let got = detector_delay(&nums(&c["ta"]), &nums(&c["ya"]), &nums(&c["tb"]), &nums(&c["yb"]));
        let want = &c["result"];
        match got {
            None => assert!(want.is_null(), "delay {i}: none against {want}"),
            Some(d) => {
                same(&format!("delay {i}"), &serde_json::json!({"delay": d.delay, "peaks": d.peaks, "corr": d.corr}), want);
            }
        }
    }
}
