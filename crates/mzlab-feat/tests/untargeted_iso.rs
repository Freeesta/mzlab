//! Isotope grouping after the grouper of mzmine 3.9.0 (`untargeted::isotopes`). Parity with `isotopes.csv` of the synthetic data (the grouper run on the
//! resolved features of each file): the same groups with the same charge; the same groups again when the rows are the ions of `aligned.csv`. Then the cases
//! made by hand: z = 2 recognised, z = 1 wins on equal rows, `monotonic` stops the search below the start row, tolerances, the representative.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mzlab_feat::table::{Aligned, CELL_DETECTED};
use mzlab_feat::untargeted::isotopes::{group, group_items, IsoParams, Item, Representative};

type Row = BTreeMap<String, String>;

fn csv(name: &str) -> Vec<Row> {
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

fn num(r: &Row, k: &str) -> f64 {
    r[k].parse().unwrap()
}

/// Groups expected from `isotopes.csv`: per file, the ids of each principal and its isotopologues, with the charge.
fn expected() -> BTreeMap<String, Vec<(BTreeSet<String>, i64)>> {
    let rows = csv("isotopes.csv");
    let mut out: BTreeMap<String, Vec<(BTreeSet<String>, i64)>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r["role"] == "principal") {
        let mut ids: BTreeSet<String> = rows
            .iter()
            .filter(|x| x["file"] == r["file"] && x["principal"] == r["id"])
            .map(|x| x["id"].clone())
            .collect();
        ids.insert(r["id"].clone());
        out.entry(r["file"].clone())
            .or_default()
            .push((ids, r["charge"].parse().unwrap()));
    }
    out
}

#[test]
fn the_groups_of_each_file_are_those_of_mzmine() {
    let rows = csv("isotopes.csv");
    let want = expected();
    let (mut total, mut same, mut extra) = (0, 0, 0);
    for (file, groups) in &want {
        let mine: Vec<&Row> = rows.iter().filter(|r| &r["file"] == file).collect();
        let items: Vec<Item> = mine
            .iter()
            .map(|r| Item {
                mz: num(r, "mz"),
                rt: num(r, "rt"),
                height: num(r, "height"),
            })
            .collect();
        let got = group_items(&items, &IsoParams::default());
        let got: Vec<(BTreeSet<String>, i64)> = got
            .groups
            .iter()
            .map(|g| {
                (
                    g.members.iter().map(|&i| mine[i]["id"].clone()).collect(),
                    i64::from(g.charge),
                )
            })
            .collect();
        total += groups.len();
        same += groups.iter().filter(|g| got.contains(g)).count();
        extra += got.iter().filter(|g| !groups.contains(g)).count();
    }
    assert_eq!(total, 8, "two groups in each of the four files");
    assert!(
        same as f64 / total as f64 >= 0.99,
        "{same} of {total} groups equal"
    );
    assert_eq!(extra, 0, "no group that mzmine does not have");
}

/// The ions of `aligned.csv` as an `Aligned` table (one cell per file).
fn aligned() -> (Aligned, Vec<Row>) {
    let rows = csv("aligned.csv");
    let n = rows.iter().map(|r| num(r, "row") as usize).max().unwrap();
    let files: Vec<String> = rows
        .iter()
        .map(|r| r["file"].clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut t = Aligned::with_capacity(files.len(), n);
    t.n_files = files.len();
    for i in 1..=n {
        let mine: Vec<&Row> = rows
            .iter()
            .filter(|r| num(r, "row") as usize == i)
            .collect();
        t.mz.push(num(mine[0], "mz"));
        t.rt.push(num(mine[0], "rt"));
        for f in &files {
            let c = mine.iter().find(|r| &r["file"] == f);
            t.area.push(c.map_or(0.0, |r| num(r, "area")));
            t.height.push(c.map_or(0.0, |r| num(r, "height") as f32));
            t.apex_rt
                .push(c.map_or(f32::NAN, |r| num(r, "rt_file") as f32));
            t.width.push(0.1);
            t.state.push(if c.is_some() {
                CELL_DETECTED
            } else {
                mzlab_feat::table::CELL_ABSENT
            });
        }
    }
    (t, rows)
}

#[test]
fn the_ions_of_the_aligned_list_group_the_same_way() {
    let (t, rows) = aligned();
    let got = group(&t, &IsoParams::default());
    // the row of a feature of a file: found by the m/z and RT of the file as aligned.csv prints them
    let row_of = |file: &str, mz: f64, rt: f64| {
        rows.iter()
            .find(|r| {
                r["file"] == file
                    && (num(r, "mz_file") - mz).abs() < 2e-5
                    && (num(r, "rt_file") - rt).abs() < 1e-3
            })
            .map(|r| num(r, "row") as usize - 1)
    };
    let iso = csv("isotopes.csv");
    // the group of a principal ion is the union over the files of the rows of its isotopologues (an isotopologue missing in one file is there in another)
    let mut by_principal: BTreeMap<usize, (BTreeSet<usize>, i64)> = BTreeMap::new();
    for r in iso.iter().filter(|r| r["role"] == "principal") {
        let main = row_of(&r["file"], num(r, "mz"), num(r, "rt")).expect("row of the principal");
        let e = by_principal
            .entry(main)
            .or_insert_with(|| (BTreeSet::new(), r["charge"].parse().unwrap()));
        for x in iso
            .iter()
            .filter(|x| x["file"] == r["file"] && (x["id"] == r["id"] || x["principal"] == r["id"]))
        {
            e.0.insert(row_of(&x["file"], num(x, "mz"), num(x, "rt")).expect("row of the feature"));
        }
    }
    let want: BTreeSet<(BTreeSet<usize>, i64)> = by_principal.into_values().collect();
    let have: BTreeSet<(BTreeSet<usize>, i64)> = got
        .groups
        .iter()
        .map(|g| (g.members.iter().copied().collect(), i64::from(g.charge)))
        .collect();
    assert_eq!(want.len(), 2, "z = 1 and z = 2");
    assert_eq!(have, want);
}

fn it(mz: f64, height: f64) -> Item {
    Item {
        mz,
        rt: 5.0,
        height,
    }
}

#[test]
fn charge_two_is_recognised() {
    let g = group_items(
        &[it(600.3003, 4e5), it(600.8014, 2e5), it(601.3034, 8e4)],
        &IsoParams::default(),
    );
    assert_eq!(g.groups.len(), 1);
    assert_eq!(
        (
            g.groups[0].charge,
            g.groups[0].members.clone(),
            g.groups[0].representative
        ),
        (2, vec![0, 1, 2], 0)
    );
}

#[test]
fn equal_rows_go_to_the_lowest_charge() {
    // 501.0033 fits z = 1 (one step up) and 499.4983 fits z = 2 (one half step down): one row each, so z = 1 wins
    let g = group_items(
        &[it(500.0, 9e5), it(499.4983, 1e5), it(501.0033, 1e5)],
        &IsoParams::default(),
    );
    assert_eq!(g.groups.len(), 1);
    assert_eq!(g.groups[0].charge, 1);
    assert_eq!(g.groups[0].members, vec![0, 2]);
}

#[test]
fn the_most_rows_win_over_the_lowest_charge() {
    let g = group_items(
        &[
            it(700.0, 9e5),
            it(700.5017, 5e5),
            it(701.0033, 2e5),
            it(701.5050, 1e5),
        ],
        &IsoParams::default(),
    );
    assert_eq!(g.groups[0].charge, 2);
    assert_eq!(g.groups[0].members.len(), 4);
}

#[test]
fn monotonic_does_not_look_below_the_start_row() {
    // the start row (most intense) is the second isotopologue: its M and M+1 are lower
    let rows = [it(300.0, 2e5), it(301.0033, 9e5), it(302.0066, 3e5)];
    let free = group_items(&rows, &IsoParams::default());
    assert_eq!(free.groups[0].members, vec![0, 1, 2]);
    let mono = group_items(
        &rows,
        &IsoParams {
            monotonic: true,
            ..IsoParams::default()
        },
    );
    assert_eq!(mono.groups.len(), 1);
    assert_eq!(
        mono.groups[0].members,
        vec![1, 2],
        "the row below is left alone"
    );
}

#[test]
fn tolerances_and_the_gap_stop_the_pattern() {
    let p = IsoParams::default();
    // M+2 without M+1 is not found; RT outside the window is not found; 0.003 Da off at 300 is outside 0.0015 Da / 5 ppm
    assert!(group_items(&[it(400.0, 9e5), it(402.0066, 1e5)], &p)
        .groups
        .is_empty());
    assert!(group_items(
        &[
            it(400.0, 9e5),
            Item {
                mz: 401.0033,
                rt: 5.2,
                height: 1e5
            }
        ],
        &p
    )
    .groups
    .is_empty());
    assert!(group_items(&[it(300.0, 9e5), it(301.0063, 1e5)], &p)
        .groups
        .is_empty());
    assert_eq!(
        group_items(&[it(300.0, 9e5), it(301.0040, 1e5)], &p)
            .groups
            .len(),
        1
    );
}

#[test]
fn the_representative_is_the_most_intense_or_the_lowest_mz() {
    let rows = [it(300.0, 2e5), it(301.0033, 9e5)];
    assert_eq!(
        group_items(&rows, &IsoParams::default()).groups[0].representative,
        1
    );
    let low = IsoParams {
        representative: Representative::LowestMz,
        ..IsoParams::default()
    };
    assert_eq!(group_items(&rows, &low).groups[0].representative, 0);
}

#[test]
fn per_row_marks_the_members() {
    let g = group_items(
        &[it(300.0, 9e5), it(301.0033, 1e5), it(900.0, 1e5)],
        &IsoParams::default(),
    );
    let (id, z) = g.per_row(3);
    assert_eq!((id, z), (vec![1, 1, 0], vec![1, 1, 0]));
}
