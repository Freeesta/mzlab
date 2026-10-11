//! `iteminfo::item_info` answers like `Item.info()` of `mzlab/explore.py` (`tools/info_riferimento.py`) on the synthetic files and the examples,
//! whole files and the parts of mixed files: from the `.mzidx` and a few header facts. Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::iteminfo::{item_info, Analog, Facts, Meta, Part};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}
fn same(path: &str, got: &Value, want: &Value) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => assert!(
            close(a.as_f64().unwrap(), b.as_f64().unwrap()),
            "{path}: {a} against {b}"
        ),
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(&format!("{path}[{i}]"), x, y);
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(
                a.keys().collect::<Vec<_>>(),
                b.keys().collect::<Vec<_>>(),
                "{path}: keys"
            );
            for (k, y) in b {
                same(&format!("{path}.{k}"), &a[k], y);
            }
        }
        (a, b) => assert_eq!(a, b, "{path}"),
    }
}
fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

#[test]
fn item_info_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab-info-{}", std::process::id()));
    let mut cmd = Command::new("python3");
    cmd.current_dir(root())
        .arg("tools/info_riferimento.py")
        .arg(&out);
    if let Ok(rd) = std::fs::read_dir(root().join("mzlab/web/esempi")) {
        let mut v: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "mzML"))
            .collect();
        v.sort();
        cmd.args(v);
    }
    let listing = match cmd.output() {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout).unwrap(),
        other => {
            if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
                panic!("the reference did not run: {other:?}");
            }
            eprintln!("python3/numpy not available: test skipped");
            return;
        }
    };
    let (mut files, mut items, mut parts_seen) = (0, 0, 0);
    for line in listing.lines().filter(|l| !l.is_empty()) {
        let stem = PathBuf::from(line)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let idx_path = out.join(format!("{stem}.mzidx"));
        let size = std::fs::metadata(&idx_path).unwrap().len();
        let mut idx = Index::open(std::fs::File::open(&idx_path).unwrap(), size).unwrap();
        let rows: Vec<Value> =
            serde_json::from_slice(&std::fs::read(out.join(format!("{stem}.info.json"))).unwrap())
                .unwrap();
        for r in &rows {
            let m = &r["meta"];
            let meta = Meta {
                file: s(&m["file"]),
                label: s(&m["label"]),
                sha256: s(&m["sha256"]),
                time: m["time"].as_f64(),
                typ: s(&m["type"]),
                conc: m["conc"].as_f64(),
                cunit: m["cunit"].as_str().map(String::from),
                rep: m["rep"].as_str().map(String::from),
            };
            let f = &r["facts"];
            let facts = Facts {
                instrument: s(&f["instrument"]),
                n_chromatograms: f["n_chromatograms"].as_i64().unwrap(),
                srm: f["srm"].as_u64().unwrap() as usize,
                pda: f["pda"].as_bool().unwrap(),
                header_polarity: s(&f["header_polarity"]),
                srm_rt: f["srm_rt"]
                    .as_array()
                    .map(|a| (a[0].as_f64().unwrap(), a[1].as_f64().unwrap())),
                analog: f["analog"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| Analog {
                        i: a["i"].as_i64().unwrap(),
                        kind: s(&a["kind"]),
                        id: s(&a["id"]),
                        unit: s(&a["unit"]),
                        t0: a["t0"].as_f64().unwrap(),
                        t1: a["t1"].as_f64().unwrap(),
                        n: a["n"].as_i64().unwrap(),
                    })
                    .collect(),
            };
            let part = r["part"].as_object().map(|p| Part {
                mode: match p["mode"].as_str().unwrap() {
                    "ms1" => "ms1",
                    "ms2" => "ms2",
                    _ => "mrm",
                },
                pol: p["pol"].as_i64().map(|v| v as i8),
                tag: s(&p["tag"]),
            });
            let got = item_info(&mut idx, &facts, &meta, part.as_ref())
                .unwrap_or_else(|e| panic!("{stem}: {e}"));
            let got: Value = serde_json::from_str(&got).unwrap();
            same(&format!("{stem} {}", meta.file), &got, &r["info"]);
            items += 1;
            parts_seen += part.is_some() as usize;
        }
        files += 1;
    }
    assert!(
        files >= 8 && items >= 8 && parts_seen >= 3,
        "{files} files, {items} items ({parts_seen} parts)"
    );
    let _ = std::fs::remove_dir_all(&out);
}
