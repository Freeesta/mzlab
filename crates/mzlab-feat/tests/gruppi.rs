//! `gruppi/1`: the fixture byte for byte, presence with replicates, sets and ratios, controls (shared and per condition), the "Prev." suggestions,
//! warnings, errors and determinism under permutation of the columns.

mod gruppi_comune;

use gruppi_comune::*;
use mzlab_core::json::{parse, Value};
use mzlab_feat::gruppi::{build, gruppi_json, run_json, GroupOpts, GroupTable};

const FIXTURE: &str = "tests/data/gruppi_fixture.json";

fn fixture_text() -> String {
    let (tb, cs, opts) = scenario();
    let input = input_json(&tb, &cs, &opts);
    let out = gruppi_json(&tb, &cs, &opts).unwrap();
    format!("{{\"input\":{input},\n\"expected\":{out}}}\n")
}

#[test]
fn fixture_is_byte_for_byte() {
    let text = fixture_text();
    if std::env::var("GRUPPI_FIXTURE_WRITE").is_ok() {
        std::fs::write(FIXTURE, &text).unwrap();
    }
    let file = std::fs::read_to_string(FIXTURE).expect("fixture");
    assert_eq!(file, text, "regenerate with GRUPPI_FIXTURE_WRITE=1");
    // The JSON entry of the module gives the expected text from the input of the file.
    let (head, tail) = file.split_once(",\n\"expected\":").unwrap();
    let input = head.strip_prefix("{\"input\":").unwrap();
    let expected = tail.strip_suffix("}\n").unwrap();
    assert_eq!(run_json(input.as_bytes()).unwrap(), expected);
}

fn doc() -> Value {
    let (tb, cs, opts) = scenario();
    parse(gruppi_json(&tb, &cs, &opts).unwrap().as_bytes()).unwrap()
}

fn row(d: &Value, id: usize) -> Value {
    d.get("rows")
        .unwrap()
        .arr()
        .iter()
        .find(|r| r.get("row").unwrap().f64() == Some(id as f64))
        .unwrap()
        .clone()
}

fn cell(r: &Value, cond: &str) -> Value {
    r.get("cond")
        .unwrap()
        .arr()
        .iter()
        .find(|c| c.get("id").unwrap().str() == Some(cond))
        .unwrap()
        .clone()
}

fn present(r: &Value, c: &str) -> bool {
    cell(r, c).get("present").unwrap().bool().unwrap()
}

fn txt(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::str).map(str::to_string)
}

#[test]
fn conditions_in_order_of_injection() {
    let d = doc();
    let ids: Vec<String> = d
        .get("conditions")
        .unwrap()
        .arr()
        .iter()
        .map(|c| txt(c, "id").unwrap())
        .collect();
    assert_eq!(ids, ["O2", "Air", "N2", "Ar", "CO2", "NoGas"]);
    let air = &d.get("conditions").unwrap().arr()[1];
    assert_eq!(air.get("files").unwrap().arr().len(), 3);
    assert_eq!(air.get("replicates").unwrap().f64(), Some(3.0));
    let o2 = &d.get("conditions").unwrap().arr()[0];
    assert_eq!(
        o2.get("factors").unwrap().get("catalyst").unwrap().str(),
        Some("TiO2 P25")
    );
}

#[test]
fn replicates_decide_presence_and_filled_cells_do_not_count() {
    let d = doc();
    // 2 of 3 detected: present with min_rep_frac 0.5; 1 of 3: absent.
    assert!(present(&row(&d, 5), "Air"));
    assert!(!present(&row(&d, 6), "Air"));
    let c = cell(&row(&d, 6), "Air");
    assert_eq!(c.get("n_det").unwrap().f64(), Some(1.0));
    assert_eq!(c.get("n_rep").unwrap().f64(), Some(3.0));
    // A filled cell is absent for the presence and keeps its area.
    let f = cell(&row(&d, 10), "O2");
    assert!(!f.get("present").unwrap().bool().unwrap());
    assert_eq!(
        f.get("kin").unwrap().arr()[0].get("area").unwrap().f64(),
        Some(7e4)
    );
    assert_eq!(f.get("ratio_ref").unwrap().f64(), None); // the reference itself is absent
                                                         // The mean of the detected replicates, with sd and cv.
    let a = cell(&row(&d, 1), "Air");
    assert!((a.get("area").unwrap().f64().unwrap() - 6e5).abs() < 1e-6);
    assert!(a.get("area_sd").unwrap().f64().unwrap() > 0.0);
    assert!(a.get("cv").unwrap().f64().unwrap() > 0.0);
    let single = cell(&row(&d, 1), "O2");
    assert!(single.get("area_sd").unwrap().is_null() && single.get("cv").unwrap().is_null());
}

#[test]
fn min_rep_frac_and_min_area_are_options() {
    let (tb, cs, _) = scenario();
    let strict = GroupOpts {
        min_rep_frac: 1.0,
        ..scenario_opts()
    };
    let d = parse(gruppi_json(&tb, &cs, &strict).unwrap().as_bytes()).unwrap();
    assert!(!present(&row(&d, 5), "Air"), "2 of 3 is not enough at 1.0");
    let big = GroupOpts {
        min_area: 1e6,
        ..scenario_opts()
    };
    let d = parse(gruppi_json(&tb, &cs, &big).unwrap().as_bytes()).unwrap();
    assert!(!present(&row(&d, 1), "N2"), "8e5 is below 1e6");
    assert!(present(&row(&d, 1), "O2"));
    assert_eq!(
        cell(&row(&d, 1), "O2")
            .get("prev")
            .unwrap()
            .get("above_min")
            .unwrap()
            .str(),
        Some("pass")
    );
}

#[test]
fn only_in_in_all_sets_and_ratios() {
    let d = doc();
    assert_eq!(txt(&row(&d, 2), "only_in").as_deref(), Some("O2"));
    assert!(row(&d, 2).get("only_in").unwrap().str().is_some());
    assert_eq!(row(&d, 1).get("in_all").unwrap().bool(), Some(true));
    assert!(row(&d, 1).get("only_in").unwrap().is_null());
    // The reference is absent in row 5: no ratio anywhere; row 2 has the reference but Air is absent: 0.
    for c in ["Air", "N2"] {
        assert!(cell(&row(&d, 5), c).get("ratio_ref").unwrap().is_null());
    }
    assert_eq!(
        cell(&row(&d, 2), "Air").get("ratio_ref").unwrap().f64(),
        Some(0.0)
    );
    assert_eq!(
        cell(&row(&d, 1), "O2").get("ratio_ref").unwrap().f64(),
        Some(1.0)
    );
    assert_eq!(
        cell(&row(&d, 1), "Air").get("ratio_ref").unwrap().f64(),
        Some(0.5)
    );
    let sets = d.get("sets").unwrap().arr();
    let names = |s: &Value| {
        s.get("conditions")
            .unwrap()
            .arr()
            .iter()
            .map(|x| x.str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    let rows = |s: &Value| {
        s.get("rows")
            .unwrap()
            .arr()
            .iter()
            .map(|x| x.f64().unwrap() as u32)
            .collect::<Vec<_>>()
    };
    let find = |want: &[&str]| sets.iter().find(|s| names(s) == want).map(rows);
    assert_eq!(find(&["O2"]), Some(vec![2]));
    assert_eq!(
        find(&["O2", "Air", "N2", "Ar", "CO2", "NoGas"]),
        Some(vec![0, 1])
    );
    assert_eq!(find(&["O2", "N2"]), Some(vec![4, 6, 9]));
    assert_eq!(find(&["O2", "Air", "N2"]), Some(vec![3, 7]));
    assert_eq!(find(&["O2", "Ar"]), Some(vec![11]));
    // Row 8 (nowhere) is in no set; the order is by number of rows, then names.
    assert!(sets.iter().all(|s| !rows(s).contains(&8)));
    let sizes: Vec<usize> = sets
        .iter()
        .map(|s| s.get("rows").unwrap().arr().len())
        .collect();
    assert!(sizes.windows(2).all(|w| w[0] >= w[1]));
}

#[test]
fn prev_suggestions_with_their_proof() {
    let d = doc();
    let prev = |r: usize, c: &str| cell(&row(&d, r), c).get("prev").unwrap().clone();
    let v = |r: usize, c: &str| txt(&prev(r, c), "verdict").unwrap();
    // Time zero has the ion: unlikely. The blank has it: unlikely.
    assert_eq!(txt(&prev(3, "O2"), "absent_t0").as_deref(), Some("fail"));
    assert_eq!(v(3, "O2"), "Prev. unlikely");
    assert_eq!(txt(&prev(9, "O2"), "absent_blank").as_deref(), Some("fail"));
    assert_eq!(v(9, "O2"), "Prev. unlikely");
    // In the parent standard: a suggestion, the row stays.
    assert_eq!(txt(&prev(4, "O2"), "in_parent_std").as_deref(), Some("yes"));
    assert_eq!(v(4, "O2"), "Prev. possible");
    // MS2 linked in O2, none in Ar; every control passes where the MS2 is linked.
    assert_eq!(txt(&prev(11, "O2"), "ms2").as_deref(), Some("linked"));
    assert_eq!(v(11, "O2"), "Prev. likely");
    assert_eq!(txt(&prev(11, "Ar"), "ms2").as_deref(), Some("none"));
    assert_eq!(v(11, "Ar"), "Prev. possible");
    // Absent in a condition: no suggestion; the parent never gets one.
    assert!(cell(&row(&d, 2), "Air").get("prev").unwrap().is_null());
    assert!(cell(&row(&d, 0), "O2").get("prev").unwrap().is_null());
}

#[test]
fn parent_decay_per_condition() {
    let d = doc();
    let o2 = &d.get("conditions").unwrap().arr()[0];
    let p = o2.get("parent").unwrap();
    // One time only: no rate constant, but the fraction left against the time zero (2e6 / 1e7).
    assert!(p.get("k").unwrap().is_null());
    assert_eq!(p.get("remaining").unwrap().f64(), Some(0.2));
}

#[test]
fn warnings_cover_the_design() {
    let (tb, cs, _) = scenario();
    let o = GroupOpts {
        reference: Some("Fe".into()),
        ..GroupOpts::default()
    };
    let d = parse(gruppi_json(&tb, &cs, &o).unwrap().as_bytes()).unwrap();
    assert!(d.get("reference").unwrap().is_null());
    let w: Vec<(String, String)> = d
        .get("warnings")
        .unwrap()
        .arr()
        .iter()
        .map(|w| (txt(w, "code").unwrap(), txt(w, "condition").unwrap()))
        .collect();
    assert_eq!(w[0], ("refMissing".to_string(), "Fe".to_string()));
    assert!(w.contains(&("condOnePoint".to_string(), "O2".to_string())));
    assert!(w.contains(&("condNoReplicates".to_string(), "O2".to_string())));
    assert!(!w.contains(&("condNoReplicates".to_string(), "Air".to_string())));
    assert!(w
        .iter()
        .all(|(c, _)| c != "condNoControls" && c != "mixedTimes"));
    for r in d.get("rows").unwrap().arr() {
        for c in r.get("cond").unwrap().arr() {
            assert!(c.get("ratio_ref").unwrap().is_null());
        }
    }
}

fn two_conditions(
    cs: Vec<mzlab_core::campioni::Campione>,
    rows: Vec<(mzlab_feat::gruppi::RowMeta, Vec<(u8, f64)>)>,
) -> GroupTable {
    table(rows, cs.len())
}

#[test]
fn controls_shared_and_per_condition() {
    // A shared time zero and blank; O2 has its own time zero (the ion is in the shared one only).
    let cs = files(vec![
        file("t0.mzML", "t0", "", Some(0.0), 1),
        file("blank.mzML", "blank", "", None, 2),
        file("t0_O2.mzML", "t0", "O2", Some(0.0), 3),
        file("O2.mzML", "sample", "O2", Some(30.0), 4),
        file("Air.mzML", "sample", "Air", Some(30.0), 5),
    ]);
    let rows = vec![(
        meta(0, "TP 1", 100.0),
        vec![d(9e5), x(), x(), d(1e6), d(1e6)],
    )];
    let tb = two_conditions(cs.clone(), rows);
    let g = build(&tb, &cs, &GroupOpts::default()).unwrap();
    let c = |id: &str| g.conditions.iter().find(|c| c.id == id).unwrap();
    assert_eq!(c("O2").controls.t0, vec![2], "the own control wins");
    assert_eq!(
        c("Air").controls.t0,
        vec![0],
        "no own control: the shared one"
    );
    assert_eq!(c("Air").controls.blank, vec![1]);
    let status = |id: &str| {
        g.rows[0]
            .cond
            .iter()
            .find(|x| x.id == id)
            .unwrap()
            .prev
            .clone()
            .unwrap()
            .absent_t0
    };
    assert_eq!(status("O2"), "pass");
    assert_eq!(status("Air"), "fail");
}

#[test]
fn errors_have_keys_and_params() {
    let (tb, cs, opts) = scenario();
    let key = |r: Result<String, mzlab_core::error::Error>| r.unwrap_err();
    let e = key(gruppi_json(&tb, &cs[..5], &opts));
    assert_eq!(e.key, "error.gruppi.sizeMismatch");
    let mut short = tb.clone();
    short.area.pop();
    assert_eq!(
        key(gruppi_json(&short, &cs, &opts)).key,
        "error.gruppi.sizeMismatch"
    );
    let istd = GroupOpts {
        norm: "istd".into(),
        ..opts.clone()
    };
    let e = key(gruppi_json(&tb, &cs, &istd));
    assert_eq!(
        (e.key, e.params[0].0, e.params[0].1.as_str()),
        ("error.gruppi.badOpts", "norm", "istd")
    );
    for bad in [
        GroupOpts {
            min_rep_frac: 0.0,
            ..opts.clone()
        },
        GroupOpts {
            shape_drop: 2.0,
            ..opts.clone()
        },
        GroupOpts {
            min_area: -1.0,
            ..opts.clone()
        },
    ] {
        assert_eq!(key(gruppi_json(&tb, &cs, &bad)).key, "error.gruppi.badOpts");
    }
    let ghost = GroupOpts {
        factors: vec![("Nope".into(), vec![])],
        ..opts.clone()
    };
    assert_eq!(
        key(gruppi_json(&tb, &cs, &ghost)).key,
        "error.gruppi.badOpts"
    );
    let mut none = cs.clone();
    for c in &mut none {
        c.series.clear();
    }
    assert_eq!(
        key(gruppi_json(&tb, &none, &opts)).key,
        "error.gruppi.noConditions"
    );
}

#[test]
fn factors_must_be_strings() {
    let (tb, cs, opts) = scenario();
    let ok = input_json(&tb, &cs, &opts);
    assert!(run_json(ok.as_bytes()).is_ok());
    let bad = ok.replace("\"catalyst\":\"TiO2 P25\"", "\"catalyst\":5");
    assert_eq!(
        run_json(bad.as_bytes()).unwrap_err().key,
        "error.gruppi.badOpts"
    );
}

/// Permutes the columns (and the files, and the files named by the MS2 links) the same way.
fn permuted(perm: &[usize]) -> (GroupTable, Vec<mzlab_core::campioni::Campione>, GroupOpts) {
    let (tb, cs, opts) = scenario();
    let mut t = permute(&tb, perm);
    for m in &mut t.rows {
        for f in &mut m.ms2_files {
            *f = perm.iter().position(|&o| o == *f).unwrap();
        }
    }
    (t, perm.iter().map(|&o| cs[o].clone()).collect(), opts)
}

fn remap(v: &Value, perm: &[usize], key: &str, out: &mut Vec<String>) {
    match v {
        Value::Obj(m) => {
            for (k, x) in m {
                if ["files", "t0", "blank", "std", "dark"].contains(&k.as_str()) {
                    let ids: Vec<String> = x
                        .arr()
                        .iter()
                        .map(|n| perm[n.f64().unwrap() as usize].to_string())
                        .collect();
                    out.push(format!("{key}.{k}={}", ids.join(",")));
                } else {
                    remap(x, perm, k, out);
                }
            }
        }
        Value::Arr(a) => a.iter().for_each(|x| remap(x, perm, key, out)),
        _ => {}
    }
}

#[test]
fn columns_in_any_order_give_the_same_bytes() {
    let (tb, cs, opts) = scenario();
    let base = gruppi_json(&tb, &cs, &opts).unwrap();
    let identity: Vec<usize> = (0..11).collect();
    let mut base_idx = Vec::new();
    remap(
        &parse(base.as_bytes()).unwrap(),
        &identity,
        "",
        &mut base_idx,
    );
    let perms: [[usize; 11]; 5] = [
        [10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
        [3, 0, 8, 1, 10, 5, 2, 7, 4, 9, 6],
        [1, 2, 0, 5, 4, 3, 8, 7, 6, 10, 9],
        [6, 5, 4, 3, 2, 1, 0, 10, 9, 8, 7],
        [4, 7, 1, 9, 0, 3, 10, 2, 5, 8, 6],
    ];
    for p in perms {
        let (t, c, o) = permuted(&p);
        let out = gruppi_json(&t, &c, &o).unwrap();
        // Everything after the conditions (rows, sets, warnings) is byte for byte the same.
        let tail = |s: &str| s[s.find("\"rows\":[").unwrap()..].to_string();
        assert_eq!(tail(&out), tail(&base), "{p:?}");
        let mut idx = Vec::new();
        remap(&parse(out.as_bytes()).unwrap(), &p, "", &mut idx);
        assert_eq!(idx, base_idx, "{p:?}");
    }
}

/// 2 000 rows x 60 files (6 conditions of 10 files): by hand with `cargo test --release -- --ignored gruppi_speed`.
#[test]
#[ignore]
fn gruppi_speed() {
    let mut v = vec![
        file("t0.mzML", "t0", "", Some(0.0), 1),
        file("blank.mzML", "blank", "", None, 2),
    ];
    for k in 0..58 {
        let c = format!("C{}", k % 6);
        v.push(file(
            &format!("{c}_{k}.mzML"),
            "sample",
            &c,
            Some(10.0 * (1 + k / 6 % 5) as f64),
            3 + k as u32,
        ));
    }
    let cs = files(v);
    let mut s = 12345u64;
    let rows: Vec<_> = (0..2000)
        .map(|r| {
            let cells = (0..60)
                .map(|_| {
                    s = s
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    if (s >> 33).is_multiple_of(3) {
                        x()
                    } else {
                        d(1e4 + ((s >> 40) % 1000) as f64)
                    }
                })
                .collect();
            (meta(r, "TP", 100.0 + r as f64), cells)
        })
        .collect();
    let tb = table(rows, 60);
    let opts = GroupOpts {
        reference: Some("C0".into()),
        ..GroupOpts::default()
    };
    let mut ms = f64::MAX;
    for _ in 0..3 {
        let t = std::time::Instant::now();
        let g = build(&tb, &cs, &opts).unwrap();
        let build_ms = t.elapsed().as_secs_f64() * 1e3;
        let out = mzlab_feat::gruppi::to_json(&g);
        let total = t.elapsed().as_secs_f64() * 1e3;
        println!(
            "gruppi 2000 x 60: build {build_ms:.1} ms, total {total:.1} ms, {} bytes",
            out.len()
        );
        ms = ms.min(total);
    }
    assert!(ms <= 50.0, "{ms} ms");
}
