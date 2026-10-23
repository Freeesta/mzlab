//! `Routes::xic_into` (the numbers of `api/xicbin`) equals bit for bit what `JSON.parse` gives for `Routes::xic` (`api/xic`), on a synthetic
//! `.mzidx`: every window of a spread of m/z, a window with no peaks, a level that does not exist, a buffer that is too short.
#![allow(clippy::indexing_slicing)]

#[path = "lettore_finto/mod.rs"]
mod finto;

use std::io::BufWriter;

use mzlab_core::idxread::Index;
use mzlab_core::json::{self, Value};
use mzlab_core::routes::Routes;
use mzlab_core::{lettori_mzml, mzidx, upload};

fn routes() -> Routes<Vec<u8>> {
    let mut l = finto::Finto::vario(400, 300, 4242);
    let mut mzml = Vec::new();
    lettori_mzml::scrivi(&mut l, 0, BufWriter::new(&mut mzml), &mut |_, _| {}).unwrap();
    let p = upload::prepare(&mut mzml.clone(), mzml.len() as u64, "x.mzML").unwrap();
    let (idx, _) = mzidx::build_mzml_to_vec(&p.table, &mzml, mzidx::DEFAULT_BUDGET).unwrap();
    let size = idx.len() as u64;
    Routes::new(Index::open(idx, size).unwrap())
}

fn doubles(v: &Value) -> Vec<f64> {
    v.arr().iter().map(|x| x.f64().unwrap()).collect()
}

#[test]
fn xicbin_numbers_are_the_json_numbers() {
    let mut r = routes();
    let mut compared = 0;
    for level in [1u8, 2] {
        let n = r.xic_len(level);
        assert!(n > 0, "level {level} has scans");
        let mut out = vec![f64::NAN; 2 * n];
        for k in 0..60 {
            let mz = 50.0 + k as f64 * 13.37;
            for tol in [0.0, 0.35, 5.0] {
                let got = r.xic_into(mz, tol, level, &mut out).unwrap();
                assert_eq!(got, n);
                let want = json::parse(r.xic(mz, tol, level).unwrap().as_bytes()).unwrap();
                let (rt, y) = (
                    doubles(want.get("rt").unwrap()),
                    doubles(want.get("y").unwrap()),
                );
                assert_eq!(rt.len(), n);
                for i in 0..n {
                    assert_eq!(out[i].to_bits(), rt[i].to_bits(), "rt {i} at {mz}");
                    assert_eq!(out[n + i].to_bits(), y[i].to_bits(), "y {i} at {mz}");
                }
                compared += n;
            }
        }
    }
    assert!(compared > 10_000);
}

#[test]
fn xicbin_edges() {
    let mut r = routes();
    let n = r.xic_len(1);
    assert_eq!(r.xic_len(9), 0);
    assert_eq!(r.xic_into(100.0, 1.0, 9, &mut vec![0.0; 2 * n]).unwrap(), 0);
    assert_eq!(
        r.xic_into(100.0, 1.0, 1, &mut vec![0.0; 2 * n - 1])
            .unwrap(),
        0
    );
    let mut out = vec![1.0; 2 * n];
    assert_eq!(r.xic_into(1.0e6, 0.1, 1, &mut out).unwrap(), n);
    assert!(out[n..].iter().all(|&v| v == 0.0));
}
