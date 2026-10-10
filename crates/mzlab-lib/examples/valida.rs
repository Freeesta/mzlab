//! `cargo run --release -p mzlab-lib --example valida -- LIB.msp QUERIES.json` -> JSON on stdout.
//! Used by tests/test_librerie_rust.py (private validation against the data repository): builds the library in memory, then for every query list
//! of `{prec, peaks}` gives the best identity, the best analogues and the time. QUERIES.json = `{"queries": {name: [q…]}, "all": [q…]}`.
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::parse::{detect, parse_all};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::search::{analogs, identity, AnalogOpts, IdentityOpts, Query, Tol};
use serde_json::{json, Value};

fn query(q: &Value) -> Option<Query> {
    let (mut mz, mut it) = (Vec::new(), Vec::new());
    for p in q["peaks"].as_array()? {
        mz.push(p[0].as_f64()?);
        it.push(p[1].as_f64()?);
    }
    Query::new(q["prec"].as_f64()?, 1, &mz, &it)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let msp = std::fs::read(&a[1]).unwrap();
    let qs: Value = serde_json::from_slice(&std::fs::read(&a[2]).unwrap()).unwrap();
    let t0 = std::time::Instant::now();
    let fmt = detect(&a[1], &msp[..msp.len().min(4096)]).unwrap();
    let (specs, _) = parse_all(fmt, &msp);
    let (bytes, info) = build_in_memory(&specs, BuildOpts::default()).unwrap();
    let build_ms = t0.elapsed().as_millis();
    let lib = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    let io = IdentityOpts {
        prec_tol: Tol::Ppm(10.0),
        frag_da: 0.01,
        pol: 1,
        top: 1,
    };
    let name = |i: u32| lib.meta(i).unwrap().name;
    let mut out = json!({"n": info.n_spectra, "bytes": bytes.len(), "build_ms": build_ms, "ident": {}, "analog": {}});
    for (k, lst) in qs["queries"].as_object().unwrap() {
        let rows: Vec<Value> = lst
            .as_array()
            .unwrap()
            .iter()
            .map(|q| match query(q).and_then(|q| identity(&lib, &q, &io).unwrap().into_iter().next()) {
                Some(h) => json!({"name": name(h.index), "ent": h.entropy, "cos": h.cosine, "shared": h.shared, "dprec": h.dprec}),
                None => Value::Null,
            })
            .collect();
        out["ident"][k] = Value::Array(rows);
        let l = lst.as_array().unwrap();
        let q = query(&l[l.len() / 2]).unwrap();
        let t = std::time::Instant::now();
        let an = analogs(
            &lib,
            &q,
            &AnalogOpts {
                pol: 1,
                ..AnalogOpts::default()
            },
        )
        .unwrap();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        out["analog"][k] = json!({"ms": ms, "top": an.iter().take(3).map(|h| json!({"name": name(h.index), "mcos": h.mc.cosine, "delta": h.mc.delta, "matched": h.mc.matched})).collect::<Vec<_>>()});
    }
    let t = std::time::Instant::now();
    let (mut strong, mut names) = (0, std::collections::BTreeMap::<String, u32>::new());
    let all = qs["all"].as_array().unwrap();
    for q in all {
        if let Some(h) = query(q).and_then(|q| identity(&lib, &q, &io).unwrap().into_iter().next())
        {
            if h.entropy >= 0.75 && h.shared >= 6 {
                strong += 1;
                *names.entry(name(h.index)).or_default() += 1;
            }
        }
    }
    out["all"] =
        json!({"ms": t.elapsed().as_millis(), "n": all.len(), "strong": strong, "names": names});
    println!("{out}");
}
