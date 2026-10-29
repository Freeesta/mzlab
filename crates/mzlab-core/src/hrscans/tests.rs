use super::*;

/// The sums per cell as they were taken before (an ordered map filled peak by peak): the replacements must give the same numbers bit for bit.
fn by_map(keys: &[i64], mz: &[f64], y: &[f64], div: f64) -> (Vec<f64>, Vec<f64>) {
    let mut cells: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for ((k, m), v) in keys.iter().zip(mz).zip(y) {
        let e = cells.entry(*k).or_insert((0.0, 0.0));
        e.0 += v;
        e.1 += v * m;
    }
    (
        cells
            .values()
            .map(|(sy, smy)| smy / sy.max(1e-12))
            .collect(),
        cells.values().map(|(sy, _)| sy / div).collect(),
    )
}

fn random(n: usize, seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut s = seed;
    let mut rnd = move || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 33) as f64 / (1u64 << 31) as f64
    };
    (
        (0..n).map(|_| 50.0 + rnd() * 1000.0).collect(),
        (0..n)
            .map(|_| rnd() * rnd() * 1e6 + if rnd() < 0.05 { 0.0 } else { 1.0 })
            .collect(),
    )
}

#[test]
fn the_cells_of_an_average_equal_the_ordered_map_sums() {
    for (hr, n) in [(true, 5000), (false, 5000), (true, 0), (false, 1)] {
        let (mz, y) = random(n, n as u64 + 3);
        let cell = Cell::of(&HrProfile {
            on: hr,
            dec: 4,
            ppm: 5.0,
        });
        let keys: Vec<i64> = mz.iter().map(|m| cell.at(*m)).collect();
        let mut cells = Cells::default();
        for ((k, m), v) in keys.iter().zip(&mz).zip(&y) {
            cells.add(*k, *v, *m);
        }
        for div in [1.0, 7.0] {
            let (a, b) = by_map(&keys, &mz, &y, div);
            let (c, d) = cells.clone().finish(div);
            assert!(
                a.iter().zip(&c).all(|(x, z)| x.to_bits() == z.to_bits()) && a.len() == c.len(),
                "mz {hr} {n}"
            );
            assert!(
                b.iter().zip(&d).all(|(x, z)| x.to_bits() == z.to_bits()) && b.len() == d.len(),
                "y {hr} {n}"
            );
        }
    }
}

#[test]
fn bin_cells_in_one_pass_equals_the_ordered_map_for_sorted_and_unsorted_peaks() {
    for (sorted, bin) in [
        (true, 0.1),
        (true, 1.0),
        (true, 0.0003),
        (false, 0.1),
        (false, 1.0),
    ] {
        let (mut mz, y) = random(4000, 11);
        if sorted {
            mz.sort_by(f64::total_cmp);
        }
        let keys: Vec<i64> = mz.iter().map(|m| (m / bin).floor() as i64).collect();
        let (want_mz, want_y) = by_map(&keys, &mz, &y, 1.0);
        let (got_mz, got_y) = bin_cells(&mz, &y, bin);
        assert!(
            want_mz.len() == got_mz.len()
                && want_mz
                    .iter()
                    .zip(&got_mz)
                    .all(|(a, b)| a.to_bits() == b.to_bits()),
            "{sorted} {bin}"
        );
        assert!(
            want_y
                .iter()
                .zip(&got_y)
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "{sorted} {bin}"
        );
    }
    assert_eq!(bin_cells(&[], &[], 0.1), (vec![], vec![]));
}
