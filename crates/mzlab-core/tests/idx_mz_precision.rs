//! The m/z rounding of an index: the MS1 of a high-resolution file is stored as f32 (`<f4`), every other section as f64 (`<f8`).
//! Pins the cost: every stored m/z within half an f32 ulp (2^-24 relative, 0.0596 ppm) of the source, f64 sections within 0.001 ppm.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)]

mod fake_reader;

use std::io::Cursor;

use fake_reader::Finto;
use mzlab_core::idxread::Index;
use mzlab_core::readers::Reader;
use mzlab_core::{mzidx, upload};

/// Largest relative error (ppm) of the stored m/z of section `si` against the nearest m/z of its source scan.
fn worst_ppm(ix: &mut Index<Vec<u8>>, l: &mut Finto, si: usize) -> f64 {
    let ids = ix.arrays(si).unwrap().scan_ids.clone();
    let n = ix.sections[si].n_peaks;
    let mut stored = Vec::new();
    ix.each_peak(si, 0, n, |p, mz, _| stored.push((p, mz)))
        .unwrap();
    let mut worst = 0.0f64;
    for (p, mz) in stored {
        let source = l.scan(0, ids[p] as usize).unwrap().mz;
        let near = source
            .iter()
            .map(|&m| (m - mz).abs() / m)
            .fold(f64::INFINITY, f64::min);
        worst = worst.max(near * 1e6);
    }
    worst
}

#[test]
fn hr_ms1_mz_rounds_to_half_an_f32_ulp_and_f64_sections_keep_it() {
    let mut l = Finto::vario(300, 200, 31337);
    let p = upload::prepare_reader(&mut l, 0, &[], "x.mzML").unwrap();
    let mut out = Cursor::new(Vec::new());
    mzidx::build_reader(&p.table, &mut l, 0, &mut out, mzidx::DEFAULT_BUDGET).unwrap();
    let b = out.into_inner();
    let size = b.len() as u64;
    let mut ix = Index::open(b, size).unwrap();
    let half_ulp_ppm = 2f64.powi(-24) * 1e6;
    let (mut f4, mut f8) = (0.0f64, 0.0f64);
    for si in 0..ix.sections.len() {
        let w = worst_ppm(&mut ix, &mut l, si);
        if ix.sections[si].f4 {
            assert_eq!(ix.sections[si].level, 1, "only MS1 is stored as f32");
            f4 = f4.max(w);
        } else {
            f8 = f8.max(w);
        }
    }
    assert!(f4 <= half_ulp_ppm, "f32 m/z off by {f4} ppm");
    // the cost is real: a round trip at 0.001 ppm does not hold for the f32 sections
    assert!(f4 > 0.001, "f32 m/z off by only {f4} ppm");
    assert!(f8 < 0.001, "f64 m/z off by {f8} ppm");
    eprintln!("worst m/z rounding: f32 {f4:.4} ppm, f64 {f8:.2e} ppm");
}
