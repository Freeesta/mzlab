use super::*;
use crate::aligned::CELL_DETECTED;

fn median_by_sort(mut v: Vec<f64>) -> f64 {
    crate::sortc::by(&mut v, f64::total_cmp);
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

#[test]
fn selection_gives_the_median_of_a_full_sort() {
    let mut seed = 3u64;
    let mut r = |n: u64| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) % n
    };
    let specials = [f64::NAN, -0.0, 0.0, f64::INFINITY, f64::NEG_INFINITY];
    for case in 0..5000 {
        let n = r(12) as usize;
        let v: Vec<f64> = (0..n)
            .map(|_| {
                if case % 3 == 0 && r(6) == 0 {
                    specials[r(5) as usize]
                } else {
                    r(7) as f64 * 0.5 - 1.0
                }
            })
            .collect();
        assert_eq!(
            median_by_sort(v.clone()).to_bits(),
            median(v).to_bits(),
            "case {case}"
        );
    }
}

fn table(rows: &[(f64, f64)]) -> Aligned {
    let mut t = Aligned::with_capacity(1, rows.len());
    for &(mz, rt) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(rt as f32);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    t
}

#[test]
fn the_nearest_standard_counts_the_mass_error_as_well_as_the_rt() {
    // an interferent 4.9 ppm off, 0.001 min from the RT given, and the standard 0.1 ppm off, 0.1 min away
    let (mz, rt) = (300.0, 5.0);
    let t = table(&[
        (mz * (1.0 + 4.9e-6), rt + 0.001),
        (mz * (1.0 + 0.1e-6), rt + 0.1),
    ]);
    let by_rt_only =
        (0..t.len()).min_by(|&a, &b| (t.rt[a] - rt).abs().total_cmp(&(t.rt[b] - rt).abs()));
    assert_eq!(by_rt_only, Some(0));
    let near = Reference::Nearest {
        mz,
        rt,
        ppm: 5.0,
        rt_tol: 0.2,
    };
    assert_eq!(standard(&t, &near).unwrap(), 1);
    // outside the window of either axis a row is not a candidate; equal distances keep the first row
    let t = table(&[(mz, rt + 0.3), (mz, rt + 0.1), (mz, rt - 0.1)]);
    assert_eq!(standard(&t, &near).unwrap(), 1);
    let none = Reference::Nearest {
        mz: 400.0,
        rt,
        ppm: 5.0,
        rt_tol: 0.2,
    };
    assert!(standard(&t, &none).is_err());
    // zero tolerances accept only an exact match instead of dividing by zero
    let exact = Reference::Nearest {
        mz,
        rt: rt + 0.1,
        ppm: 0.0,
        rt_tol: 0.0,
    };
    assert_eq!(standard(&t, &exact).unwrap(), 1);
}

#[test]
fn the_sum_streams_in_row_order_and_keeps_nan_for_a_file_without_cells() {
    let mut t = table(&[(100.0, 1.0), (101.0, 2.0), (102.0, 3.0)]);
    t.area = vec![0.1, 1e16, -1.0];
    t.state[2] = CELL_ABSENT;
    assert_eq!(
        per_file(&t, &Method::Sum, None)[0].to_bits(),
        (0.1f64 + 1e16).to_bits()
    );
    t.state = vec![CELL_ABSENT; 3];
    assert!(per_file(&t, &Method::Sum, None)[0].is_nan());
    assert!(per_file(&t, &Method::Median, None)[0].is_nan());
}

#[test]
fn an_istd_id_must_be_a_whole_row_number() {
    let parse = |s: &str| Method::from_json(&mzlab_core::json::parse(s.as_bytes()).unwrap());
    assert_eq!(
        parse(r#"{"metodo":"istd","istd":{"id":3}}"#).unwrap(),
        Method::Istd(Reference::Id(3))
    );
    for bad_id in ["-1", "2.5", "4294967296", "1e30"] {
        assert!(
            parse(&format!(r#"{{"metodo":"istd","istd":{{"id":{bad_id}}}}}"#)).is_err(),
            "{bad_id}"
        );
    }
}
