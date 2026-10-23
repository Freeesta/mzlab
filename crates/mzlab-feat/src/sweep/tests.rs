use super::*;

#[test]
fn own_cursor_gives_the_sums_of_the_shared_one() {
    let mut slab = Slab::default();
    for s in 0..40u32 {
        for k in 0..30u32 {
            slab.mz.push(100.0 + k as f64);
            slab.it.push((s * 31 + k) as f64);
            slab.pos.push(s);
        }
    }
    // table order is by m/z: sort the 1200 peaks the way the index stores them
    let mut ord: Vec<usize> = (0..slab.mz.len()).collect();
    ord.sort_by(|&a, &b| slab.mz[a].total_cmp(&slab.mz[b]).then(a.cmp(&b)));
    slab.it = ord.iter().map(|&i| slab.it[i]).collect();
    slab.pos = ord.iter().map(|&i| slab.pos[i]).collect();
    slab.mz = ord.iter().map(|&i| slab.mz[i]).collect();
    slab.prepare().unwrap();
    let mut cur = slab.cursor();
    for (a, b) in [(0, 400), (390, 800), (700, 1200), (10, 80)] {
        let (mut y1, mut y2) = (vec![0.0; 12], vec![0.0; 12]);
        slab.add_trace(a, b, 5, &mut y1).unwrap();
        slab.add_trace_cur(&mut cur, a, b, 5, &mut y2);
        assert_eq!(y1, y2);
    }
}

#[test]
fn grouping_a_big_slab_by_scan_matches_walking_the_window() {
    // enough peaks to be shared between threads; the table is sorted by m/z, scans interleave
    let (nscan, per) = (600u32, 500u32);
    let mut peaks: Vec<(f64, f64, u32)> = Vec::new();
    for s in 0..nscan {
        for k in 0..per {
            peaks.push((
                100.0 + k as f64 + (s % 7) as f64 * 1e-3,
                (s * 31 + k) as f64,
                s,
            ));
        }
    }
    peaks.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)));
    let mut slab = Slab::default();
    for &(m, y, s) in &peaks {
        slab.mz.push(m);
        slab.it.push(y);
        slab.pos.push(s);
    }
    slab.prepare().unwrap();
    let mut cur = slab.cursor();
    for (a, b, w0) in [(0, 20_000, 100), (5_000, 9_000, 3), (100_000, 160_000, 590)] {
        let (mut y1, mut y2) = (vec![0.0; 10], vec![0.0; 10]);
        walk_into(&slab.pos[a..b], &slab.it[a..b], w0, &mut y1);
        slab.add_trace_cur(&mut cur, a, b, w0, &mut y2);
        assert_eq!(y1, y2);
    }
}
