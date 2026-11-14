//! The spectral study (`docs/agenti/studio-spettri.md`): synthetic generator, scenarios E0-E6, arms and metrics, shared by the tests
//! `studio_spettri_*.rs` (small N, pinned numbers) and the example `studio_spettri` (large N, release mode).
#![allow(dead_code)]

pub mod analogue;
pub mod arms;
pub mod r#gen;
pub mod idf;
pub mod metrics;
pub mod null;
pub mod scen;

use mzlab_lib::search::Query;
use serde_json::{json, Value};

/// A scenario with its library bytes and its cleaned queries.
pub struct Setup {
    pub s: scen::Scen,
    pub bytes: Vec<u8>,
}

impl Setup {
    pub fn new(name: &str, seed: u64, n_lib: usize, n: usize) -> Setup {
        let s = scen::build(name, seed, n_lib, n);
        let bytes = arms::build_lib(&s, None);
        Setup { s, bytes }
    }
    pub fn queries(&self) -> Vec<Option<Query>> {
        self.s
            .queries
            .iter()
            .map(|q| arms::query(&q.peaks))
            .collect()
    }
}

/// The baselines that mean something on a scenario: the identity arms everywhere but E5, the analogue search on E5 and on E0 (where any analogue
/// it returns is a false one, the true entry being closer than `min_delta`).
pub fn baselines_for(name: &str) -> Vec<&'static str> {
    match name {
        "e5" => vec!["analogs"],
        "e0" => arms::BASELINES.to_vec(),
        _ => arms::BASELINES[..3].to_vec(),
    }
}

/// The metrics of one arm's rows, tagged with the arm, the scenario and the seed.
pub fn tag(rows: &[arms::Row], s: &scen::Scen, arm: &str, name: &str, seed: u64) -> Value {
    let mut v = metrics::report(rows, &s.queries);
    v["arm"] = json!(arm);
    v["scenario"] = json!(name);
    v["seed"] = json!(seed);
    v
}

/// One JSON object per baseline arm on scenario `name`.
pub fn run_baselines(name: &str, seed: u64, n_lib: usize, n: usize) -> Vec<Value> {
    let st = Setup::new(name, seed, n_lib, n);
    let l = arms::open(&st.bytes);
    let qs = st.queries();
    baselines_for(name)
        .iter()
        .map(|arm| tag(&arms::baseline(arm, &l, &qs), &st.s, arm, name, seed))
        .collect()
}

/// Library size and seed of the pinned tests (the example takes them from the command line).
pub const N_LIB: usize = 600;
pub const SEED: u64 = 1;
/// The numbers are deterministic; the tolerance only absorbs the rounding of the report (4 decimals).
pub const TOL: f64 = 1e-3;

/// The entropy arm is the engine's own target-decoy search: same best target and same q as `decoy::search_fdr` on every query.
pub fn check_engine(name: &str, n: usize) {
    use mzlab_lib::decoy::{search_fdr, DecoyKind};
    let st = Setup::new(name, SEED, N_LIB, n);
    let l = arms::open(&st.bytes);
    let qs = st.queries();
    let rows = arms::baseline("entropy", &l, &qs);
    let f = search_fdr(&l.lib, &qs, &arms::identity_opts(), DecoyKind::Fragments).unwrap();
    for (r, x) in rows.iter().zip(&f) {
        let top = x.best.index.and_then(|i| l.ids[i as usize]);
        assert_eq!(r.ranked.first().map(|t| t.0), top);
        if top.is_some() {
            assert_eq!(r.q, x.fdr.q_value);
        }
    }
}

/// A function that runs a set of arms on a scenario: (name, seed, n_lib, n) -> one JSON object per arm.
pub type Runner = fn(&str, u64, usize, usize) -> Vec<Value>;

/// The arm sets of the example and of `tools/studio_spettri.py`: the baselines, arm A (`null`) and arm B (`idf`).
pub fn runner(set: &str) -> Runner {
    match set {
        "base" => run_baselines,
        "null" => null::run_null,
        "idf" => idf::run_idf,
        _ => panic!("unknown arm set {set}"),
    }
}

/// Runs the baselines of `name` twice (the JSON must be byte-identical) and compares every pin (arm, group, metric, value) within `TOL`.
/// The measured lines go to stderr, so a deliberate change of the engine or the generator re-pins them from the test output.
pub fn pinned(name: &str, n: usize, pins: &[(&str, &str, &str, f64)]) {
    pinned_with(run_baselines, name, n, pins)
}

/// As `pinned` for the arms of `run`.
pub fn pinned_with(run: Runner, name: &str, n: usize, pins: &[(&str, &str, &str, f64)]) {
    let a = run(name, SEED, N_LIB, n);
    let b = run(name, SEED, N_LIB, n);
    assert_eq!(
        Value::from(a.clone()).to_string(),
        Value::from(b).to_string(),
        "two runs differ"
    );
    check_engine(name, n);
    for v in &a {
        eprintln!("{v}");
    }
    let mut bad = Vec::new();
    for &(arm, g, m, want) in pins {
        let v = a
            .iter()
            .find(|v| v["arm"] == arm)
            .unwrap_or_else(|| panic!("no arm {arm}"));
        let x = if g == "all" {
            &v["all"][m]
        } else {
            &v["groups"][g][m]
        };
        let got = x.as_f64().unwrap_or(f64::NAN);
        if (got - want).abs() > TOL || got.is_nan() {
            bad.push(format!("{arm} {g} {m}: {got} (pinned {want})"));
        }
    }
    assert!(bad.is_empty(), "{name}:\n{}", bad.join("\n"));
}
