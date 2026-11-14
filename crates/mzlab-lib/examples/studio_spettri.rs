//! `cargo run --release -p mzlab-lib --example studio_spettri -- <scenario> <n_lib> <n_query> <seed> [base|null|idf]`: one JSON line per arm of
//! the spectral study (`docs/agenti/studio-spettri.md`) with its metrics, the time of the arm set divided by its arms (`ms`) and `det` (the
//! scenario and the arms run twice give the same library bytes, queries and JSON). `tools/studio_spettri.py` runs it over 3 seeds.

#[path = "../tests/studio_spettri/mod.rs"]
mod studio_spettri;

use studio_spettri::{runner, Setup};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let name = a.first().map_or("e0", |s| s.as_str());
    let num = |i: usize, d: u64| a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n_lib, n, seed) = (num(1, 20_000) as usize, num(2, 500) as usize, num(3, 1));
    let run = runner(a.get(4).map_or("base", |s| s.as_str()));
    let st = Setup::new(name, seed, n_lib, n);
    let again = Setup::new(name, seed, n_lib, n);
    let same_input = st.bytes == again.bytes
        && st
            .s
            .queries
            .iter()
            .zip(&again.s.queries)
            .all(|(x, y)| x.peaks == y.peaks && x.truth == y.truth);
    let t0 = std::time::Instant::now();
    let rows = run(name, seed, n_lib, n);
    let ms = t0.elapsed().as_secs_f64() * 1e3 / rows.len().max(1) as f64;
    let det = same_input
        && serde_json::Value::from(rows.clone())
            == serde_json::Value::from(run(name, seed, n_lib, n));
    for mut v in rows {
        v["n_lib"] = serde_json::json!(n_lib);
        v["ms"] = serde_json::json!(ms.round());
        v["det"] = serde_json::json!(det);
        println!("{v}");
    }
}
