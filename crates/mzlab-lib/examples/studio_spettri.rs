//! `cargo run --release -p mzlab-lib --example studio_spettri -- <scenario> <n_lib> <n_query> <seed>`: one JSON line per arm of the spectral study
//! (`docs/agenti/studio-spettri.md`) with its metrics, the search time of the arm (`ms`) and `det` (the scenario built twice gives the same
//! library bytes and queries, the arm run twice the same rows). `tools/studio_spettri.py` runs it over 3 seeds and prints the MISURA lines.

#[path = "../tests/studio_spettri/mod.rs"]
mod studio_spettri;

use studio_spettri::{arms, baselines_for, tag, Setup};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let name = a.first().map_or("e0", |s| s.as_str());
    let num = |i: usize, d: u64| a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n_lib, n, seed) = (num(1, 20_000) as usize, num(2, 500) as usize, num(3, 1));
    let st = Setup::new(name, seed, n_lib, n);
    let again = Setup::new(name, seed, n_lib, n);
    let same_input = st.bytes == again.bytes
        && st
            .s
            .queries
            .iter()
            .zip(&again.s.queries)
            .all(|(x, y)| x.peaks == y.peaks && x.truth == y.truth);
    let l = arms::open(&st.bytes);
    let qs = st.queries();
    for arm in baselines_for(name) {
        let t0 = std::time::Instant::now();
        let rows = arms::baseline(arm, &l, &qs);
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        let det = same_input && rows == arms::baseline(arm, &l, &qs);
        let mut v = tag(&rows, &st.s, arm, name, seed);
        v["n_lib"] = serde_json::json!(n_lib);
        v["ms"] = serde_json::json!(ms.round());
        v["det"] = serde_json::json!(det);
        println!("{v}");
    }
}
