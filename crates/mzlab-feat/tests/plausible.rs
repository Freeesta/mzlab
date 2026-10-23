//! `plausible/1`: a clean table flags nothing; each broken value is flagged by its own check and by that one only.

use mzlab_core::json::{parse, Value};
use mzlab_feat::plausible::{self, IDS};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};

/// Rows `(mz, rt)` with two files; every cell is a detected peak of area 1000 and width 0.1 min.
fn table(rows: &[(f64, f64)]) -> Aligned {
    let mut t = Aligned::with_capacity(2, rows.len());
    for &(mz, rt) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for _ in 0..2 {
            t.area.push(1000.0);
            t.height.push(100.0);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state.push(CELL_DETECTED);
        }
    }
    t
}

/// Count and flag of a check in the text.
fn read(t: &Aligned, tol: f64) -> Vec<(String, i64, String)> {
    let text = plausible::to_json(t, tol);
    let j = parse(text.as_bytes()).expect("json");
    assert_eq!(j.get("schema").and_then(Value::str), Some("plausible/1"));
    let list = j.get("checks").map(|c| c.arr()).expect("checks");
    list.iter()
        .map(|c| {
            (
                c.get("id").and_then(Value::str).unwrap().to_string(),
                c.get("count").and_then(|x| x.f64()).unwrap() as i64,
                c.get("flag").and_then(Value::str).unwrap().to_string(),
            )
        })
        .collect()
}

fn only(got: &[(String, i64, String)], id: &str) {
    for (i, n, f) in got {
        if i == id {
            assert_eq!(
                (*n, f.as_str()),
                (1, if id == "empty_rows" { "note" } else { "warn" }),
                "{id}"
            );
        } else if i != "ppm_error" {
            assert_eq!(
                (*n, f.as_str()),
                (0, "ok"),
                "{i} should be clean when {id} is broken"
            );
        }
    }
}

#[test]
fn clean_table_is_ok_and_ppm_has_no_identity_to_read() {
    let got = read(&table(&[(200.1, 5.0), (300.2, 6.0)]), 0.005);
    assert_eq!(got.iter().map(|g| g.0.as_str()).collect::<Vec<_>>(), IDS);
    for (i, n, f) in &got {
        assert_eq!(*n, 0, "{i}");
        assert_eq!(f, if i == "ppm_error" { "na" } else { "ok" }, "{i}");
    }
}

#[test]
fn each_broken_value_has_its_own_check() {
    let mut t = table(&[(200.1, 5.0), (300.2, 6.0)]);
    t.area[0] = f64::NAN;
    only(&read(&t, 0.005), "non_finite");
    let t = table(&[(200.1, 5.0), (3.0, 6.0)]);
    only(&read(&t, 0.005), "mz_range");
    let t = table(&[(200.1, 5.0), (300.2, 6000.0)]);
    only(&read(&t, 0.005), "rt_range");
    let mut t = table(&[(200.1, 5.0), (300.2, 6.0)]);
    t.height[0] = -1.0;
    only(&read(&t, 0.005), "negative");
    let mut t = table(&[(200.1, 5.0), (300.2, 6.0)]);
    t.width[0] = 0.0;
    only(&read(&t, 0.005), "peak_shape");
    let mut t = table(&[(200.1, 5.0), (300.2, 6.0)]);
    for c in 0..2 {
        t.area[c] = 0.0;
        t.height[c] = 0.0;
        t.state[c] = CELL_ABSENT;
        t.apex_rt[c] = f32::NAN;
        t.width[c] = f32::NAN;
    }
    only(&read(&t, 0.005), "empty_rows");
}

#[test]
fn ppm_error_is_read_only_with_identity_and_accurate_mass() {
    let mut t = table(&[(200.1, 5.0), (300.2, 6.0)]);
    let h = mzlab_core::elements::adducts()[0].1;
    // row 1 is exact, row 2 is 100 ppm off
    t.ident.neutral = vec![200.1 - h, 300.2 - h + 0.03];
    t.ident.ion = vec![1, 1];
    t.ident.src = vec![1, 1];
    let got = read(&t, 0.005);
    assert_eq!(
        got.iter()
            .find(|g| g.0 == "ppm_error")
            .map(|g| (g.1, g.2.as_str())),
        Some((1, "warn"))
    );
    // unit resolution: the same table says nothing about ppm
    let got = read(&t, 0.3);
    assert_eq!(
        got.iter()
            .find(|g| g.0 == "ppm_error")
            .map(|g| g.2.as_str()),
        Some("na")
    );
}

#[test]
fn same_table_same_bytes() {
    let t = table(&[(200.1, 5.0)]);
    assert_eq!(plausible::to_json(&t, 0.005), plausible::to_json(&t, 0.005));
}
