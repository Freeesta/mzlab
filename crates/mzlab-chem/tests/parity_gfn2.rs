//! GFN2-xTB charges and condensed Fukui functions against tblite 0.7.0 (`tests/data/cc_ref.json`, same geometry):
//! neutral, cation and anion converge; |Δq| ≤ 2e-3 and |Δf| ≤ 3e-3 per atom, in the gas phase and in ALPB water.

use mzlab_chem::xtb::{alpb, System};
use std::time::Instant;

const DQ: f64 = 2e-3;
const DF: f64 = 3e-3;

fn reference() -> serde_json::Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/cc_ref.json"
    ))
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn vec(v: &serde_json::Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn maxdiff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// (max |Δq| of N, N+1, N−1; max |Δf|; all converged) for one molecule and phase.
fn compare(rec: &serde_json::Value, phase: &str) -> (f64, f64, bool) {
    let el: Vec<u8> = rec["z"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();
    let xyz: Vec<[f64; 3]> = rec["xyz"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let v = vec(p);
            [v[0], v[1], v[2]]
        })
        .collect();
    let solv = (phase == "alpb").then_some(&alpb::WATER);
    let sys = System::new(&el, &xyz, solv).unwrap();
    let n = sys.solve(0, None).unwrap();
    let c = sys.solve(1, Some(&n.guess)).unwrap();
    let a = sys.solve(-1, Some(&n.guess)).unwrap();
    let r = &rec[phase];
    let fp: Vec<f64> = n.q.iter().zip(&a.q).map(|(x, y)| x - y).collect();
    let fm: Vec<f64> = c.q.iter().zip(&n.q).map(|(x, y)| x - y).collect();
    let dq = maxdiff(&n.q, &vec(&r["q"]))
        .max(maxdiff(&c.q, &vec(&r["q_cation"])))
        .max(maxdiff(&a.q, &vec(&r["q_anion"])));
    let df = maxdiff(&fp, &vec(&r["f_plus"])).max(maxdiff(&fm, &vec(&r["f_minus"])));
    (dq, df, n.converged && c.converged && a.converged)
}

fn run(phase: &str) -> bool {
    let all = reference();
    let mut ok = true;
    for (name, rec) in all["molecules"].as_object().unwrap() {
        let t = Instant::now();
        let (dq, df, conv) = compare(rec, phase);
        let pass = conv && dq <= DQ && df <= DF;
        ok &= pass;
        println!(
            "{phase} {name:16} dq={dq:.1e} df={df:.1e} converged={conv} {:.0} ms {}",
            t.elapsed().as_secs_f64() * 1e3,
            if pass { "ok" } else { "FAIL" }
        );
    }
    ok
}

#[test]
fn gas_phase() {
    assert!(run("gas"));
}

#[test]
fn alpb_water() {
    assert!(run("alpb"));
}
