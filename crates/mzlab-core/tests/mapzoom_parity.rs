//! A region map folded from the per-scan columns of its window (`mapzoom`) is the one the peaks give, bit by bit: same window, other RT stretches, linear and log axes.
#[path = "lettore_finto/mod.rs"]
mod finto;

use std::io::Cursor;

use mzlab_core::idxread::Index;
use mzlab_core::routes::Routes;
use mzlab_core::{mzidx, upload};

fn routes(n: usize, peaks: usize) -> Routes<Vec<u8>> {
    let mut l = finto::Finto::vario(n, 1, 7);
    let mut rng = finto::Caso(5);
    for (k, sc) in l.schede.iter_mut().enumerate() {
        let sp = &mut sc.spettro;
        sp.level = 1;
        sp.polarity = 1;
        sp.profile = false;
        sp.precursor = None;
        sp.wide = None;
        sp.rt = 0.3 + k as f64 * 25.0 / n as f64;
        sp.mz = (0..peaks).map(|_| 100.0 + rng.unit() * 900.0).collect();
        mzlab_core::sortc::slice_by(&mut sp.mz, |a, b| a.total_cmp(b));
        sp.intensity = (0..peaks).map(|_| (rng.unit() * 1e6) as f32).collect();
    }
    let p = upload::prepare_lettore(&mut l, 0, &[], "x.mzML").expect("table");
    let mut out = Cursor::new(Vec::new());
    mzidx::build_lettore(&p.table, &mut l, 0, &mut out, 192 << 20).expect("index");
    let bytes = out.into_inner();
    let size = bytes.len() as u64;
    let mut r = Routes::new(Index::open(bytes, size).expect("open"));
    r.pol = Some(1);
    r
}

#[test]
fn columns_give_the_same_region_as_the_peaks() {
    let mut r = routes(1500, 2000);
    let cases = [
        ((0.0, 26.0), (100.0, 1000.0), false),
        ((5.0, 12.0), (100.0, 1000.0), false),
        ((8.1, 9.7), (100.0, 1000.0), false),
        ((8.1, 9.7), (100.0, 1000.0), true),
        ((0.0, 26.0), (150.5, 990.25), false),
        ((10.0, 20.0), (150.5, 990.25), false),
    ];
    for (rt, mz, log) in cases {
        r.set_scan_cells(true);
        let fast = r
            .region_map(1, rt, 300, mz, (1021, log), false)
            .expect("region");
        let third = r
            .region_map(1, (rt.0 + 0.1, rt.1), 300, mz, (1021, log), false)
            .expect("region");
        let again = r
            .region_map(1, (rt.0 + 0.3, rt.1), 300, mz, (1021, log), false)
            .expect("region");
        r.set_scan_cells(false);
        let slow = r
            .region_map(1, rt, 300, mz, (1021, log), false)
            .expect("region");
        let slow3 = r
            .region_map(1, (rt.0 + 0.1, rt.1), 300, mz, (1021, log), false)
            .expect("region");
        let slow2 = r
            .region_map(1, (rt.0 + 0.3, rt.1), 300, mz, (1021, log), false)
            .expect("region");
        let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&fast), bits(&slow), "{rt:?} {mz:?} log {log}");
        assert_eq!(bits(&third), bits(&slow3), "{rt:?} {mz:?} log {log} third");
        assert_eq!(
            bits(&again),
            bits(&slow2),
            "{rt:?} {mz:?} log {log} shifted"
        );
    }
}
