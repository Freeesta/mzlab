//! The bridge on the synthetic campaign: its table is, byte for byte, the one of the native code (`mzlab-feat` with `attributio`), written in order at
//! offsets, and its levels are the expected ones of `livelli_attesi.csv`.
use mzlab_attributio_wasm::{render_to, run_with, Files};
use mzlab_feat::attributio::annotate::{gnps_comparer, load_annotations};
use mzlab_feat::attributio::campaign::Env;
use mzlab_feat::attributio::curation::TableCuration;
use mzlab_feat::attributio::io::{Io, MemFiles};
use mzlab_feat::attributio::sources::load_sources;
use mzlab_feat::attributio::table::Run;
use mzlab_feat::attributio::Campaign;
use std::path::{Path, PathBuf};

fn base() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/attributio/campagna_sintetica")
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
    for e in std::fs::read_dir(dir).expect("folder") {
        let p = e.expect("entry").path();
        if p.is_dir() {
            walk(base, &p, out);
        } else {
            let rel = p
                .strip_prefix(base)
                .expect("inside")
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, std::fs::read(&p).expect("file")));
        }
    }
}

fn folder() -> Vec<(String, Vec<u8>)> {
    let mut v = Vec::new();
    walk(&base(), &base(), &mut v);
    v
}

fn names(files: &[(String, Vec<u8>)]) -> String {
    let list: Vec<String> = files
        .iter()
        .map(|(p, d)| format!("{{\"path\":{p:?},\"size\":{}}}", d.len()))
        .collect();
    format!(
        "{{\"root\":\"campagna_sintetica\",\"files\":[{}]}}",
        list.join(",")
    )
}

fn bridge(kind: &str) -> (Vec<u8>, String) {
    let files = folder();
    let f = Files::from_json(&names(&files)).expect("names");
    let run = run_with(&f, &|k, off, len| {
        let d = &files[k].1;
        Some(d[off as usize..(off + len) as usize].to_vec())
    })
    .expect("run");
    let mut out: Vec<u8> = Vec::new();
    let mut next = 0u64;
    assert!(render_to(&run, kind, &mut |off, b| {
        assert_eq!(off, next, "written in order");
        next += b.len() as u64;
        out.extend_from_slice(b);
        true
    }));
    (out, run.summary())
}

/// The native path, composed by hand from the public steps and rendered by the same renderers.
fn native(kind: &str) -> Vec<u8> {
    let mut m = MemFiles::default();
    for (p, d) in folder() {
        m.add(&p, &d);
    }
    let text = String::from_utf8(m.read("campaign.toml").expect("toml")).expect("utf-8");
    let c = Campaign::from_toml(&text, "campaign.toml", "campagna_sintetica", Env::default())
        .expect("campaign");
    let read = |p: &str| m.read(p);
    let list = |d: &str| m.list(d);
    let io = Io::new(&read, &list);
    let src = load_sources(&c, &io).expect("sources");
    let cur = TableCuration::load(&c, &io).expect("curation");
    let a = load_annotations(&c, &src, None, &cur, &gnps_comparer).expect("annotations");
    let run = Run {
        features: a.features,
        warnings: Vec::new(),
        fingerprint: String::new(),
        settings: Default::default(),
        counts: Default::default(),
    };
    let mut out = String::new();
    let mut emit = |s: &str| out.push_str(s);
    match kind {
        "csv" => run.csv_to(&mut emit),
        "xlsx" => run.xlsx_rows_to(&mut emit),
        _ => run.json_to(&mut emit),
    }
    out.into_bytes()
}

#[test]
fn the_three_renderings_are_the_native_ones_byte_for_byte() {
    for kind in ["json", "csv", "xlsx"] {
        assert_eq!(bridge(kind).0, native(kind), "{kind}");
    }
    let (_, summary) = bridge("json");
    let s: serde_json::Value = serde_json::from_str(&summary).expect("summary");
    assert_eq!(s["n_features"], 15);
    assert_eq!(s["fingerprint"].as_str().map(str::len), Some(24));
}

/// Rows of a `;` text with quotes (a quoted cell may hold line breaks).
fn rows(text: &str) -> Vec<Vec<String>> {
    let (mut out, mut row, mut cell, mut quoted) = (Vec::new(), Vec::new(), String::new(), false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ';' if !quoted => row.push(std::mem::take(&mut cell)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut cell));
                out.push(std::mem::take(&mut row));
            }
            _ => cell.push(c),
        }
    }
    out
}

#[test]
fn the_levels_are_the_expected_ones_and_the_csv_has_a_line_each() {
    let got = rows(&String::from_utf8(bridge("csv").0).expect("utf-8"));
    let want = rows(&std::fs::read_to_string(base().join("livelli_attesi.csv")).expect("expected"));
    assert_eq!(got.len(), 16, "header + 15 features");
    assert_eq!(
        got[0].join(";"),
        "pol;id;mz;rt;level;sub;basis;name;identity;reasons"
    );
    // livelli_attesi: _pol;_id;mz;rt;level;level_sub;level_basis;name;level_decisive
    let pick = |r: &Vec<String>, i: [usize; 5]| i.map(|k| r[k].clone());
    let mut a: Vec<_> = got[1..].iter().map(|r| pick(r, [0, 1, 4, 5, 6])).collect();
    let mut b: Vec<_> = want[1..].iter().map(|r| pick(r, [0, 1, 4, 5, 6])).collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
    let names = |t: &Vec<Vec<String>>, k: usize| -> Vec<(String, String, String)> {
        let mut v: Vec<_> = t[1..]
            .iter()
            .map(|r| (r[0].clone(), r[1].clone(), r[k].clone()))
            .collect();
        v.sort();
        v
    };
    assert_eq!(names(&got, 7), names(&want, 7), "names");
}

#[test]
fn the_sheet_rows_and_a_missing_toml() {
    let v: serde_json::Value = serde_json::from_slice(&bridge("xlsx").0).expect("json");
    assert_eq!(v["hojas"][0]["filas"].as_array().expect("rows").len(), 16);
    let f = Files::from_json("{\"root\":\"x\",\"files\":[]}").expect("names");
    let e = run_with(&f, &|_, _, _| None).err().expect("error");
    assert_eq!(e.key, "error.attributio");
}
