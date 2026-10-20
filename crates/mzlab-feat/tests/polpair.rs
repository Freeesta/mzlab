//! The proof "same neutral mass in the other polarity" (`polpair`): synthetic positive and negative tables of the same files; the pairs of the
//! `polarity_pairs` of the untargeted import (without the filter on the course); decoys with the neutral mass of the negative table moved.

use mzlab_feat::legacy::identity::polarity_pairs;
use mzlab_feat::polpair::{cross_polarity, pairs, PolParams};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const PROTON: f64 = 1.00727646688;
const FILES: usize = 5;

fn table(rows: &[(f64, f64, [f64; FILES])]) -> Aligned {
    let mut t = Aligned::with_capacity(FILES, rows.len());
    for &(mz, rt, a) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for v in a {
            t.area.push(v);
            t.height.push((v / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state.push(CELL_DETECTED);
        }
    }
    t
}

const UP: [f64; FILES] = [1e6, 2e6, 4e6, 8e6, 1.6e7];

fn acid(neutral: f64, rt_neg: f64, neg: [f64; FILES]) -> (Aligned, Aligned) {
    (
        table(&[(neutral + PROTON, 5.0, UP)]),
        table(&[(neutral - PROTON, rt_neg, neg)]),
    )
}

#[test]
fn an_acid_seen_in_both_polarities_with_the_same_course_is_one_pair() {
    let (p, n) = acid(200.1, 5.05, UP.map(|v| v / 5.0));
    let q = pairs(&p, &n, &PolParams::default());
    assert_eq!(q.len(), 1);
    assert_eq!((q[0].ion_pos, q[0].ion_neg), ("[M+H]+", "[M-H]-"));
    assert!(q[0].r.unwrap() >= 0.8 && q[0].n_common == FILES);
    assert!(q[0].ppm.abs() < 0.01 && (q[0].d_rt - 0.05).abs() < 1e-9);
}

#[test]
fn a_minute_apart_or_an_opposite_course_gives_no_pair() {
    let (p, n) = acid(200.1, 6.0, UP);
    assert!(pairs(&p, &n, &PolParams::default()).is_empty());
    let (p, n) = acid(200.1, 5.0, [1.6e7, 8e6, 4e6, 2e6, 1e6]);
    assert!(pairs(&p, &n, &PolParams::default()).is_empty());
    // with the filter off the pair is there, and r says why it would be dropped
    let q = pairs(
        &p,
        &n,
        &PolParams {
            trend: false,
            ..PolParams::default()
        },
    );
    assert_eq!(q.len(), 1);
    assert!(q[0].r.unwrap() < 0.0);
}

fn file(t: &Aligned, recipe: &Recipe, names: &[&str]) -> Vec<u8> {
    let o = Origin {
        inputs: names
            .iter()
            .map(|n| (n.to_string(), "00".repeat(32)))
            .collect(),
        ..Origin::default()
    };
    write_table(t, recipe, &o)
}

#[test]
fn tables_of_other_files_are_refused_and_unit_tables_are_empty() {
    let (p, n) = acid(200.1, 5.0, UP);
    let names = ["a", "b", "c", "d", "e"];
    let (bp, bn) = (
        file(&p, &Recipe::default(), &names),
        file(&n, &Recipe::default(), &names),
    );
    let go = |a: &[u8], b: &[u8]| {
        cross_polarity(
            &mut a.to_vec(),
            a.len() as u64,
            &mut b.to_vec(),
            b.len() as u64,
            "",
        )
    };
    let ok = go(&bp, &bn).unwrap();
    assert!(ok.starts_with("{\"schema\":\"polpair/1\",\"pairs\":[{\"row_pos\":0,\"row_neg\":0,\"id_pos\":1,\"id_neg\":1,\"ion_pos\":\"[M+H]+\",\"ion_neg\":\"[M-H]-\""));
    let other = file(&n, &Recipe::default(), &["a", "b", "c", "d", "x"]);
    assert_eq!(go(&bp, &other).unwrap_err().key, "error.polpair.files");
    let (up, un) = (
        file(&p, &Recipe::lr(0.0, 1.0, 1.0), &names),
        file(&n, &Recipe::lr(0.0, 1.0, 1.0), &names),
    );
    assert_eq!(
        go(&up, &un).unwrap(),
        "{\"schema\":\"polpair/1\",\"pairs\":[]}"
    );
}

/// A deterministic spread of neutral masses and RTs.
fn masses(n: usize) -> Vec<(f64, f64)> {
    let mut s = 12345u64;
    let mut next = move || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 33) as f64 / (1u64 << 31) as f64
    };
    (0..n)
        .map(|_| (150.0 + 450.0 * next(), 2.0 + 10.0 * next()))
        .collect()
}

fn series(i: usize) -> [f64; FILES] {
    UP.map(|v| v * (1.0 + 0.1 * ((i * 7) % 5) as f64))
}

#[test]
fn the_pairs_are_those_of_the_untargeted_import_without_the_course_filter() {
    let ms = masses(40);
    let pos: Vec<_> = ms
        .iter()
        .enumerate()
        .map(|(i, &(m, rt))| (m + PROTON, rt, series(i)))
        .collect();
    // negatives: the same molecules as [M-H]-, every third one as [M+Cl]-, with an RT that sometimes drifts out of the window
    let neg: Vec<_> = ms
        .iter()
        .enumerate()
        .map(|(i, &(m, rt))| {
            (
                if i % 3 == 0 {
                    m + 34.969402
                } else {
                    m - PROTON
                },
                rt + if i % 5 == 0 { 0.3 } else { 0.02 },
                series(i),
            )
        })
        .collect();
    let (tp, tn) = (table(&pos), table(&neg));
    let ours = pairs(
        &tp,
        &tn,
        &PolParams {
            trend: false,
            ..PolParams::default()
        },
    );
    let list = |rows: &[(f64, f64, [f64; FILES])]| {
        rows.iter()
            .enumerate()
            .map(|(k, r)| format!("{{\"k\":{k},\"mz\":{},\"rt\":{}}}", r.0, r.1))
            .collect::<Vec<_>>()
            .join(",")
    };
    let payload = format!(
        "{{\"pos\":[{}],\"neg\":[{}],\"ppm\":5,\"rt_min\":0.15}}",
        list(&pos),
        list(&neg)
    );
    let theirs = polarity_pairs(&payload).unwrap();
    let v = mzlab_core::json::parse(theirs.as_bytes()).unwrap();
    let mut want: Vec<(usize, usize, String, String)> = v
        .arr()
        .iter()
        .map(|x| {
            let f = |k: &str| x.get(k).unwrap();
            (
                f("pos").f64().unwrap() as usize,
                f("neg").f64().unwrap() as usize,
                f("pos_form").str().unwrap().to_string(),
                f("neg_form").str().unwrap().to_string(),
            )
        })
        .collect();
    let mut got: Vec<_> = ours
        .iter()
        .map(|q| {
            (
                q.row_pos,
                q.row_neg,
                q.ion_pos.to_string(),
                q.ion_neg.to_string(),
            )
        })
        .collect();
    want.sort();
    got.sort();
    assert!(!got.is_empty());
    assert_eq!(got, want);
}

#[test]
fn a_neutral_mass_moved_by_a_few_hundredths_of_a_dalton_loses_the_pairs() {
    let ms = masses(60);
    let count = |shift: f64| {
        let pos: Vec<_> = ms
            .iter()
            .enumerate()
            .map(|(i, &(m, rt))| (m + PROTON, rt, series(i)))
            .collect();
        let neg: Vec<_> = ms
            .iter()
            .enumerate()
            .map(|(i, &(m, rt))| (m + shift - PROTON, rt, series(i)))
            .collect();
        pairs(&table(&pos), &table(&neg), &PolParams::default()).len()
    };
    let truth = count(0.0);
    assert!(truth >= 60, "{truth}");
    for shift in [0.02, 0.05, 1.0] {
        let c = count(shift);
        eprintln!("shift {shift}: {c} pairs against {truth}");
        assert!(c * 10 <= truth);
    }
}
