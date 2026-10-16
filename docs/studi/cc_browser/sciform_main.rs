// Study probe (docs/studi/CC_MOTORE_NEL_BROWSER.md, section 3), not part of the program.
// Built against a local copy of sci-form 0.15.2 (MIT) whose solve_gfn2 reads the total charge from GFN2_CHARGE
// (n_electrons -= charge; initial shell charges = charge / n_shells). Usage: sciform_main ref.json sf.json
use serde_json::{json, Value};
use std::time::Instant;
fn q(z: &[u8], x: &[[f64; 3]], c: i64) -> (Vec<f64>, bool) {
    unsafe { std::env::set_var("GFN2_CHARGE", c.to_string()) };
    let r = sci_form::xtb::gfn2::solve_gfn2(z, x).expect("gfn2");
    (r.mulliken_charges, r.converged)
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let d: Value = serde_json::from_str(&std::fs::read_to_string(&a[1]).unwrap()).unwrap();
    let mut out = serde_json::Map::new();
    for (name, r) in d.as_object().unwrap() {
        let z: Vec<u8> = r["Z"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u8).collect();
        let x: Vec<[f64; 3]> = r["xyz"].as_array().unwrap().iter().map(|p| { let p = p.as_array().unwrap(); [p[0].as_f64().unwrap(), p[1].as_f64().unwrap(), p[2].as_f64().unwrap()] }).collect();
        let t = Instant::now();
        let (q0, c0) = q(&z, &x, 0); let (qc, c1) = q(&z, &x, 1); let (qa, c2) = q(&z, &x, -1);
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let fm: Vec<f64> = qc.iter().zip(&q0).map(|(a, b)| a - b).collect();
        let fp: Vec<f64> = q0.iter().zip(&qa).map(|(a, b)| a - b).collect();
        let t2 = Instant::now();
        let e = sci_form::compute_fukui_descriptors(&z, &x).expect("eht");
        let ms_eht = t2.elapsed().as_secs_f64() * 1e3;
        let t3 = Instant::now();
        let emb = sci_form::embed(r["smiles"].as_str().unwrap(), 42);
        let ms_emb = t3.elapsed().as_secs_f64() * 1e3;
        out.insert(name.clone(), json!({"q0": q0, "fm": fm, "fp": fp, "conv": [c0, c1, c2], "ms": ms,
            "eht_fm": e.f_minus, "ms_eht": ms_eht, "emb_ok": emb.error.is_none(), "emb_err": emb.error, "emb_n": emb.num_atoms, "emb_el": emb.elements, "emb_xyz": emb.coords, "ms_emb": ms_emb}));
    }
    std::fs::write(&a[2], serde_json::to_string(&out).unwrap()).unwrap();
}
