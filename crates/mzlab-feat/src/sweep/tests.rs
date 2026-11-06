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

#[test]
fn the_peaks_of_one_scan_are_the_ones_of_filtering_the_window() {
    let mut slab = Slab::default();
    let mut x = 0x0bad_cafe_f00d_1234u64;
    for k in 0..3_000u32 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        slab.mz.push(100.0 + k as f64 * 1e-3);
        slab.it.push(k as f64);
        slab.pos.push((x % 37) as u32);
    }
    for (a, b) in [(0, 3_000), (10, 11), (500, 1_700), (2_999, 3_000), (40, 40)] {
        for s in 0..40u32 {
            let want: Vec<u32> = (a..b)
                .filter(|&j| slab.pos[j] == s)
                .map(|j| j as u32)
                .collect();
            assert_eq!(
                slab.of_scan(s, a, b).unwrap(),
                &want[..],
                "{a}..{b} scan {s}"
            );
        }
    }
}

/// An HR index (f32 m/z, as an Orbitrap survey) of `nscan` scans of random peaks, some m/z repeated exactly across scans.
fn random_index(nscan: usize, per: usize) -> Index<Vec<u8>> {
    use mzlab_core::mzml::Spectrum;
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(nscan).unwrap();
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    for s in 0..nscan {
        let mut mz: Vec<f64> = (0..per)
            .map(|k| {
                if k % 9 == 0 {
                    300.0 + k as f64 * 0.5
                } else {
                    100.0 + 900.0 * next()
                }
            })
            .collect();
        mz.sort_by(f64::total_cmp);
        let intensity = (0..per).map(|_| (1.0 + 1e5 * next()) as f32).collect();
        w.spectrum(&Spectrum {
            index: s,
            id: format!("scan={}", s + 1),
            level: 1,
            rt: s as f64 * 0.01,
            polarity: 1,
            filter: "FTMS + p ESI Full ms [100.0000-1000.0000]".into(),
            mz,
            intensity,
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    let mut src = b.clone();
    let p = mzlab_core::upload::prepare(&mut src, b.len() as u64, "r.mzML").unwrap();
    let idx = mzlab_core::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
        .unwrap()
        .0;
    let n = idx.len() as u64;
    Index::open(idx, n).unwrap()
}

#[test]
fn sparse_slabs_give_every_range_the_peaks_of_the_whole_table() {
    let mut ix = random_index(120, 400);
    let si = ix.section_of(1, None).unwrap();
    assert!(ix.sections[si].f4);
    let mut all: Vec<(f64, f64, u32)> = Vec::new();
    let n = ix.sections[si].n_peaks;
    ix.each_peak(si, 0, n, |p, m, y| all.push((m, y, p as u32)))
        .unwrap();
    // narrow, overlapping, empty, wide and repeated ranges, by lower bound; the bounds compare in f32 like the table
    let mut ranges: Vec<(f64, f64)> = vec![
        (50.0, 60.0),
        (300.0, 300.0),
        (300.0, 301.0),
        (999.0, 2000.0),
        (400.0, 650.0),
        (100.0, 100.002),
    ];
    ranges.extend(ppm_ranges(
        &(0..300)
            .map(|k| 101.0 + k as f64 * 2.97)
            .collect::<Vec<_>>(),
        5.0,
    ));
    ranges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let cast = |v: f64| v as f32 as f64;
    let want: Vec<Vec<(f64, f64, u32)>> = ranges
        .iter()
        .map(|&(lo, hi)| {
            all.iter()
                .copied()
                .filter(|p| p.0 >= cast(lo) && p.0 <= cast(hi))
                .collect()
        })
        .collect();
    for max_peaks in [1, 7, 64, 500, 5_000, 1 << 20] {
        let mut got: Vec<Vec<(f64, f64, u32)>> = vec![Vec::new(); ranges.len()];
        let mut order = Vec::new();
        windows(&mut ix, si, &ranges, max_peaks, |k, a, b, slab| {
            order.push(k);
            got[k] = (a..b)
                .map(|j| (slab.mz[j], slab.it[j], slab.pos[j]))
                .collect();
            Ok(())
        })
        .unwrap();
        assert_eq!(order, (0..ranges.len()).collect::<Vec<_>>(), "{max_peaks}");
        assert_eq!(got, want, "{max_peaks}");
    }
}
