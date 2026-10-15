//! Join aligner of mzmine 3.9.0 rewritten in Rust (`untargeted::join`). Parity: from the per-file features of `resolved.csv` to the rows of `aligned.csv`
//! (`tests/data/untargeted/mzmine39/`, synthetic files) the same features are in the same rows and the row m/z and RT agree within the print precision of
//! the CSV (m/z 5 decimals, RT 4). Then the cases made by hand: tie, out of tolerance, the base list chosen by size, two features of one file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mzlab_feat::untargeted::join::{align, JoinParams};
use mzlab_feat::untargeted::Feature;

fn csv(name: &str) -> Vec<BTreeMap<String, String>> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/untargeted/mzmine39")
        .join(name);
    let text = std::fs::read_to_string(p).unwrap();
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    lines
        .map(|l| {
            head.iter()
                .map(|h| h.to_string())
                .zip(l.split(',').map(str::to_string))
                .collect()
        })
        .collect()
}

fn num(r: &BTreeMap<String, String>, k: &str) -> f64 {
    r[k].parse().unwrap()
}

/// Identity of a feature across the two tables: file, m/z (5 decimals) and RT (3 decimals) as written by mzmine.
fn key(file: &str, mz: f64, rt: f64) -> String {
    format!("{file}|{mz:.5}|{rt:.3}")
}

fn feat(mz: f64, rt: f64) -> Feature {
    Feature {
        mz,
        rt,
        rt_start: rt - 0.1,
        rt_end: rt + 0.1,
        height: 1e5,
        area: 1e4,
        filled: false,
    }
}

#[test]
fn rows_like_mzmine() {
    let resolved = csv("resolved.csv");
    let files: Vec<String> = resolved
        .iter()
        .map(|r| r["file"].clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let lists: Vec<Vec<Feature>> = files
        .iter()
        .map(|f| {
            resolved
                .iter()
                .filter(|r| &r["file"] == f)
                .map(|r| Feature {
                    mz: num(r, "mz"),
                    rt: num(r, "rt"),
                    rt_start: num(r, "rt_start"),
                    rt_end: num(r, "rt_end"),
                    height: num(r, "height"),
                    area: num(r, "area"),
                    filled: false,
                })
                .collect()
        })
        .collect();
    let got = align(&lists, &JoinParams::default());
    let ours: Vec<(BTreeSet<String>, f64, f64)> = got
        .rows
        .iter()
        .map(|r| {
            let set = r
                .cells
                .iter()
                .enumerate()
                .filter_map(|(k, c)| c.as_ref().map(|f| key(&files[k], f.mz, f.rt)))
                .collect();
            (set, r.mz, r.rt)
        })
        .collect();
    let mut gold: BTreeMap<String, (BTreeSet<String>, f64, f64)> = BTreeMap::new();
    for r in csv("aligned.csv") {
        let e =
            gold.entry(r["row"].clone())
                .or_insert((BTreeSet::new(), num(&r, "mz"), num(&r, "rt")));
        e.0.insert(key(&r["file"], num(&r, "mz_file"), num(&r, "rt_file")));
    }
    assert!(!gold.is_empty());
    let same = gold
        .values()
        .filter(|(set, mz, rt)| {
            ours.iter()
                .any(|(s, m, t)| s == set && (m - mz).abs() <= 2e-5 && (t - rt).abs() <= 1e-4)
        })
        .count();
    let share = same as f64 / gold.len() as f64;
    assert!(
        share >= 0.99,
        "rows equal to mzmine: {same} of {} ({share:.3})",
        gold.len()
    );
    assert_eq!(ours.len(), gold.len(), "number of rows");
    // the rows come out in RT order, like the list of mzmine after the renumbering
    assert!(got.rows.windows(2).all(|w| w[0].rt <= w[1].rt));
}

#[test]
fn tie_goes_to_the_first_row_and_the_other_stays_alone() {
    // y lies halfway in RT between x1 and x2 (same m/z): equal scores. The base is the list of two rows.
    let p = JoinParams::default();
    let a = align(
        &[
            vec![feat(300.1, 1.0), feat(300.1, 1.2)],
            vec![feat(300.1, 1.1)],
        ],
        &p,
    );
    assert_eq!(a.rows.len(), 2);
    assert!(a.rows[0].cells[1].is_some() && a.rows[0].cells[0].as_ref().unwrap().rt == 1.0);
    assert!(a.rows[1].cells[1].is_none());
    // the order of the base list does not change the outcome
    let b = align(
        &[
            vec![feat(300.1, 1.2), feat(300.1, 1.0)],
            vec![feat(300.1, 1.1)],
        ],
        &p,
    );
    assert_eq!(
        a.rows.iter().map(|r| r.rt).collect::<Vec<_>>(),
        b.rows.iter().map(|r| r.rt).collect::<Vec<_>>()
    );
    assert!(b.rows[0].cells[1].is_some() && b.rows[1].cells[1].is_none());
}

#[test]
fn out_of_tolerance_makes_a_new_row() {
    let p = JoinParams::default();
    let r = align(
        &[
            vec![feat(200.0, 1.0), feat(250.0, 2.0)],
            vec![feat(200.0, 1.05), feat(250.0, 2.5), feat(200.01, 1.0)],
        ],
        &p,
    );
    // 200.0/1.05 joins 200.0/1.0; 250.0/2.5 is 0.5 min away; 200.01 is 0.01 Da away (limit 0.0015): both are alone
    let together = r
        .rows
        .iter()
        .filter(|x| x.cells.iter().all(Option::is_some))
        .count();
    assert_eq!((r.rows.len(), together), (4, 1));
}

#[test]
fn the_base_is_the_list_with_most_rows() {
    // Two files with the same features except that file 1 has one more; the larger list is the base whichever file it is,
    // so the rows are the same set for both orders of the files.
    let big = vec![feat(200.0, 1.0), feat(300.0, 2.0), feat(400.0, 3.0)];
    let small = vec![feat(200.0, 1.02), feat(300.0, 2.02)];
    let p = JoinParams::default();
    let ab = align(&[small.clone(), big.clone()], &p);
    let ba = align(&[big, small], &p);
    assert_eq!(ab.rows.len(), 3);
    assert_eq!(ba.rows.len(), 3);
    let shape = |r: &mzlab_feat::untargeted::join::Rows| {
        r.rows
            .iter()
            .map(|x| x.cells.iter().flatten().count())
            .collect::<Vec<_>>()
    };
    assert_eq!(shape(&ab), vec![2, 2, 1]);
    assert_eq!(shape(&ba), vec![2, 2, 1]);
    // rows are numbered by RT and the mean RT is the mean of the two features
    assert!((ab.rows[0].rt - 1.01).abs() < 1e-6);
}

#[test]
fn two_features_of_one_file_close_together_only_one_enters() {
    // y1 and y2 (same file, 0.04 min apart) both lie in the tolerance of x. The base is the list of two; x enters the closer one.
    let p = JoinParams::default();
    let r = align(
        &[
            vec![feat(350.17, 2.50)],
            vec![feat(350.17, 2.40), feat(350.17, 2.54)],
        ],
        &p,
    );
    assert_eq!(r.rows.len(), 2);
    let joined: Vec<_> = r.rows.iter().filter(|x| x.cells[0].is_some()).collect();
    assert_eq!(joined.len(), 1);
    assert_eq!(joined[0].cells[1].as_ref().unwrap().rt, 2.54);
}

#[test]
fn zero_weights_align_nothing() {
    let p = JoinParams {
        w_mz: 0.0,
        w_rt: 0.0,
        ..JoinParams::default()
    };
    assert!(align(&[vec![feat(200.0, 1.0)]], &p).rows.is_empty());
}
