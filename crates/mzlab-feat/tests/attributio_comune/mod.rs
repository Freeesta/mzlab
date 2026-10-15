//! Helpers shared by the Attributio parity tests: fixtures, golden data and comparisons.
#![allow(dead_code)]

use mzlab_feat::attributio::campaign::{Campaign, Env};
use mzlab_feat::attributio::detection::Detection;
use mzlab_feat::attributio::frame::{Cell, Frame, Record};
use mzlab_feat::attributio::io::{Io, MemFiles};
use mzlab_feat::attributio::sources::load_sources;
use mzlab_feat::attributio::toml::Value as T;
use mzlab_feat::attributio::Sources;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn golden(dir: &str, name: &str) -> Value {
    let p = root().join("tests/golden/attributio").join(dir).join(name);
    serde_json::from_str(
        &std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())),
    )
    .expect("golden JSON")
}

pub fn walk(base: &Path, dir: &Path, files: &mut MemFiles) {
    for e in std::fs::read_dir(dir).expect("fixture folder") {
        let p = e.expect("entry").path();
        if p.is_dir() {
            walk(base, &p, files);
        } else {
            let rel = p
                .strip_prefix(base)
                .expect("inside")
                .to_string_lossy()
                .replace('\\', "/");
            files.add(&rel, &std::fs::read(&p).expect("fixture file"));
        }
    }
}

pub struct Fixture {
    pub files: MemFiles,
    pub campaign: Campaign,
}

pub fn fixture(name: &str) -> Fixture {
    let base = root().join("tests/data/attributio").join(name);
    let mut files = MemFiles::default();
    walk(&base, &base, &mut files);
    let text =
        String::from_utf8(files.read("campaign.toml").expect("campaign.toml")).expect("utf-8");
    let campaign =
        Campaign::from_toml(&text, "campaign.toml", name, Env::default()).expect("campaign");
    Fixture { files, campaign }
}

#[macro_export]
macro_rules! with_io {
    ($fx:expr, $io:ident) => {
        let read = |p: &str| $fx.files.read(p);
        let list = |d: &str| $fx.files.list(d);
        let $io = Io::new(&read, &list);
    };
}

// ------------------------------------------------------------------------------------------------------------------ comparison
pub fn same(a: &Value, b: &Value, at: &str) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().expect("f64"), y.as_f64().expect("f64"));
            assert!(
                (x - y).abs() <= 1e-9 * x.abs().max(y.abs()).max(1e-300) || x == y,
                "{at}: {x} != {y}"
            );
        }
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{at}: length");
            x.iter()
                .zip(y)
                .enumerate()
                .for_each(|(i, (p, q))| same(p, q, &format!("{at}[{i}]")));
        }
        (Value::Object(x), Value::Object(y)) => {
            let mut kx: Vec<&String> = x.keys().collect();
            let mut ky: Vec<&String> = y.keys().collect();
            kx.sort();
            ky.sort();
            assert_eq!(kx, ky, "{at}: keys");
            x.iter()
                .for_each(|(k, v)| same(v, &y[k], &format!("{at}.{k}")));
        }
        _ => assert_eq!(a, b, "{at}"),
    }
}

pub fn cell_json(c: &Cell) -> Value {
    match c {
        Cell::Null => Value::Null,
        Cell::Bool(b) => json!(b),
        Cell::Int(i) => json!(i),
        Cell::Num(n) => json!(n),
        Cell::Str(s) => json!(s),
    }
}

pub fn rows_json<R: Record>(rows: &[R]) -> Value {
    Value::Array(
        rows.iter()
            .map(|r| {
                Value::Object(
                    R::COLUMNS
                        .iter()
                        .zip(r.cells())
                        .map(|(c, v)| (c.to_string(), cell_json(&v)))
                        .collect(),
                )
            })
            .collect(),
    )
}

pub fn table_block<R: Record>(rows: &[R], golden: &Value, at: &str) {
    assert_eq!(
        golden["righe"].as_u64().expect("righe") as usize,
        rows.len(),
        "{at}: rows"
    );
    assert_eq!(json!(R::COLUMNS), golden["colonne"], "{at}: columns");
    let first: Vec<Value> = rows_json(rows)
        .as_array()
        .expect("array")
        .iter()
        .take(5)
        .cloned()
        .collect();
    same(&Value::Array(first), &golden["prime5"], at);
}

/// A column the way pandas types it: numbers when every present cell is one, text otherwise; missing cells as null.
pub fn frame_rows(f: &Frame, cols: &[&str], limit: usize) -> Value {
    let kinds: HashMap<&str, bool> = cols
        .iter()
        .map(|c| {
            let k = f.col(c).expect("column");
            (
                *c,
                (0..f.len()).all(|r| f.str_na(r, k).is_none() || f.num(r, k).is_some()),
            )
        })
        .collect();
    Value::Array(
        (0..f.len().min(limit))
            .map(|r| {
                let obj = cols.iter().map(|c| {
                    let k = f.col(c).expect("column");
                    let v = match (f.str_na(r, k), kinds[c]) {
                        (None, _) => Value::Null,
                        (Some(_), true) => json!(f.num(r, k)),
                        (Some(t), false) => json!(t),
                    };
                    (c.to_string(), v)
                });
                Value::Object(obj.collect())
            })
            .collect(),
    )
}

pub fn toml_json(v: &T) -> Value {
    match v {
        T::Str(s) => json!(s),
        T::Int(i) => json!(i),
        T::Float(f) => json!(f),
        T::Bool(b) => json!(b),
        T::Array(a) => Value::Array(a.iter().map(toml_json).collect()),
        T::Table(t) => Value::Object(t.0.iter().map(|(k, v)| (k.clone(), toml_json(v))).collect()),
    }
}

pub fn detection_json(rows: &[Detection]) -> Value {
    Value::Array(
        rows.iter()
            .map(|d| {
                let mut o = serde_json::Map::new();
                o.insert("_id".into(), json!(d.id));
                o.insert("_pol".into(), json!(d.pol));
                o.insert("mz".into(), json!(d.mz));
                o.insert("rt".into(), json!(d.rt));
                d.in_site
                    .iter()
                    .for_each(|(s, b)| drop(o.insert(format!("in_{s}"), json!(b))));
                d.any_sorbent
                    .iter()
                    .for_each(|(s, b)| drop(o.insert(format!("any_{s}"), json!(b))));
                Value::Object(o)
            })
            .collect(),
    )
}

pub fn tables(fx: &Fixture) -> Sources {
    with_io!(fx, io);
    load_sources(&fx.campaign, &io).expect("sources")
}
