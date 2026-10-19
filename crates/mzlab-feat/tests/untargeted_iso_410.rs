//! Isotope grouping after the grouper of mzmine 4.10 (`untargeted::isotopes`). Parity with `isotopes.csv` of the synthetic data (the grouper run on the
//! resolved features of each file): the same groups with the same charge; the same groups again when the rows are the ions of `aligned.csv`. Then the cases
//! made by hand: z = 2 recognised, z = 1 wins on equal rows, `monotonic` stops the search below the start row, tolerances, the representative.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mzlab_feat::table::{Aligned, CELL_DETECTED};
use mzlab_feat::untargeted::isotopes::{group, group_items, IsoParams, Item};

type Row = BTreeMap<String, String>;

fn csv(name: &str) -> Vec<Row> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/untargeted/mzmine410")
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
    if !std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/untargeted/mzmine410"
    ))
    .exists()
    {
        eprintln!("SKIP the_groups_of_each_file_are_those_of_mzmine: tests/data/untargeted/mzmine410 is missing (the 4.10 gold data need an mzmine account: tools/bench/oro_mzmine410.py)");
        return;
    }
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
    if !std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/untargeted/mzmine410"
    ))
    .exists()
    {
        eprintln!("SKIP the_ions_of_the_aligned_list_group_the_same_way: tests/data/untargeted/mzmine410 is missing (the 4.10 gold data need an mzmine account: tools/bench/oro_mzmine410.py)");
        return;
    }
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
