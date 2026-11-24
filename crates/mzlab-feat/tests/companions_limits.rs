//! `features.companions@1`: the caps and the degenerate series. The weak list is cut at 200 and keeps the strongest; `max_edges` cuts after the implied
//! edges are left out and keeps the strongest; a constant series is untested (no Pearson), and a row with zero heights shares no file with any other.

use mzlab_core::json::{parse, Value};
use mzlab_feat::companions::companions_text;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const FILES: usize = 5;
const S: [f64; FILES] = [1.0, 2.0, 3.0, 4.0, 5.0];
const P: [f64; FILES] = [5.0, 1.0, 4.0, 2.0, 3.0];

/// A table of `rows` = (m/z, RT, one height per file), every cell detected.
fn table(rows: &[(f64, f64, [f64; FILES])]) -> Vec<u8> {
    let mut t = Aligned::with_capacity(FILES, rows.len());
    for (mz, rt, h) in rows {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for x in h {
            t.area.push(x * 6.0);
            t.height.push(*x as f32);
            t.apex_rt.push(*rt as f32);
            t.width.push(0.08);
            t.state.push(CELL_DETECTED);
        }
    }
    let origin = Origin {
        inputs: (0..FILES)
            .map(|k| (format!("c{k}.mzML"), format!("{k:02x}").repeat(32)))
            .collect(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

fn run(rows: &[(f64, f64, [f64; FILES])], params: &[(String, Value)]) -> Value {
    parse(
        companions_text(params, &table(rows))
            .unwrap()
            .text
            .as_bytes(),
    )
    .unwrap()
}

fn n(v: &Value, path: &[&str]) -> f64 {
    path.iter().fold(v, |v, k| v.get(k).unwrap()).f64().unwrap()
}

fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).unwrap().arr()
}

fn mix(a: f64) -> [f64; FILES] {
    std::array::from_fn(|f| 1e5 * (a * S[f] + (1.0 - a) * P[f] + 1.0))
}

/// 100 rows one Da apart at one RT, each a mix of two profiles, so many pairs correlate between `weak_corr` and `min_corr`.
fn dense() -> Vec<(f64, f64, [f64; FILES])> {
    (0..100)
        .map(|k| (300.0 + k as f64, 3.0, mix(((k * 37) % 100) as f64 / 99.0)))
        .collect()
}

#[test]
fn the_weak_list_keeps_the_strongest_and_counts_the_rest() {
    let v = run(&dense(), &[]);
    let weak = arr(&v, "weak");
    let total = n(&v, &["counts", "weak"]) as usize;
    assert!(total > 200, "the table must overflow the cap: {total}");
    assert_eq!(weak.len(), 200);
    assert_eq!(n(&v, &["weak_dropped"]) as usize, total - 200);
    assert_eq!(v.get("weak_truncated"), Some(&Value::Bool(true)));
    let r: Vec<f64> = weak
        .iter()
        .map(|e| e.get("r").unwrap().f64().unwrap())
        .collect();
    assert!(
        r.windows(2).all(|w| w[0].abs() >= w[1].abs()),
        "strongest first: {r:?}"
    );
    assert!(r.iter().all(|x| (0.5..0.9).contains(x)), "{r:?}");
    // a table with no overflow drops nothing
    let small = run(&dense()[..10], &[]);
    assert_eq!(n(&small, &["weak_dropped"]), 0.0);
    assert_eq!(small.get("weak_truncated"), Some(&Value::Bool(false)));
}

#[test]
fn max_edges_keeps_the_strongest_after_the_implied_ones_are_left_out() {
    let rows = dense();
    let full = run(&rows, &[]);
    let all = arr(&full, "edges");
    assert!(all.len() > 50);
    let cap = vec![("max_edges".to_string(), Value::Num(50.0))];
    let cut = run(&rows, &cap);
    let kept = arr(&cut, "edges");
    assert_eq!(kept.len(), 50);
    let c = |v: &Value, k: &str| n(v, &["counts", k]);
    assert_eq!(c(&cut, "dropped"), (all.len() - 50) as f64);
    assert_eq!(c(&full, "dropped"), 0.0);
    // the cap comes after `implied` and leaves every other count to its kind
    assert_eq!(c(&cut, "implied"), c(&full, "implied"));
    let kinds = c(&cut, "isotope") + c(&cut, "relation") + c(&cut, "coelution");
    assert_eq!(kinds, 50.0);
    // every kept edge is at least as strong as every edge that was cut, and the kept ones are in row order
    let key = |e: &Value| {
        (
            n(e, &["from"]) as i64,
            n(e, &["to"]) as i64,
            e.get("kind").and_then(Value::str).unwrap().to_string(),
        )
    };
    let kept_keys: Vec<_> = kept.iter().map(key).collect();
    assert!(kept_keys
        .windows(2)
        .all(|w| (w[0].0, w[0].1) <= (w[1].0, w[1].1)));
    let weakest_kept = kept
        .iter()
        .map(|e| n(e, &["r"]))
        .fold(f64::INFINITY, f64::min);
    let strongest_cut = all
        .iter()
        .filter(|e| !kept_keys.contains(&key(e)))
        .map(|e| n(e, &["r"]))
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        weakest_kept >= strongest_cut,
        "{weakest_kept} < {strongest_cut}"
    );
    assert!(kept_keys.iter().all(|k| all.iter().any(|e| &key(e) == k)));
    // a cap of zero keeps none and counts all
    let none = run(&rows, &[("max_edges".to_string(), Value::Num(0.0))]);
    assert!(arr(&none, "edges").is_empty());
    assert_eq!(c(&none, "dropped"), all.len() as f64);
}

#[test]
fn a_constant_series_is_untested_never_an_edge() {
    let flat = [1e5; FILES];
    let rows = [
        (301.0, 3.0, flat),
        (302.0, 3.0, mix(1.0)),
        (303.0, 3.0, flat),
    ];
    let v = run(&rows, &[]);
    assert!(arr(&v, "edges").is_empty(), "{v:?}");
    assert!(n(&v, &["counts", "untested"]) >= 3.0);
    // the pairs that fit an isotope step are listed as weak with no number
    let weak = arr(&v, "weak");
    assert!(weak.len() >= 2);
    assert!(
        weak.iter().all(|e| e.get("r").unwrap().is_null()),
        "{weak:?}"
    );
    assert_eq!(n(&v, &["counts", "weak"]), 0.0);
}

#[test]
fn zero_heights_share_no_file_and_never_panic() {
    let rows = [
        (301.0, 3.0, mix(1.0)),
        (302.0, 3.0, [0.0; FILES]),
        (303.0, 3.0, mix(1.0)),
    ];
    let v = run(&rows, &[]);
    // rows 1 and 3 are the same series 2 Da apart: an isotope-step edge; row 2 has nothing to compare
    let edges = arr(&v, "edges");
    assert!(
        edges
            .iter()
            .all(|e| n(e, &["from"]) != 2.0 && n(e, &["to"]) != 2.0),
        "{edges:?}"
    );
    assert!(edges
        .iter()
        .any(|e| n(e, &["from"]) == 1.0 && n(e, &["to"]) == 3.0));
    let touching: Vec<&Value> = arr(&v, "weak")
        .iter()
        .filter(|e| n(e, &["from"]) == 2.0 || n(e, &["to"]) == 2.0)
        .collect();
    assert!(
        touching
            .iter()
            .all(|e| e.get("r").unwrap().is_null() && n(e, &["n_files"]) == 0.0),
        "{touching:?}"
    );
    assert!(n(&v, &["counts", "untested"]) >= 2.0);
}
