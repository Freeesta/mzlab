//! The checks of the table (`tabcheck/1`) on series made like the synthetic files of `tools/dati_sintetici.py`: the parent 364.4 at 14.3 min that decays, its
//! isotopes 365.4 and 366.4, the in-source fragments 194.2, 152.2 and 124.3 at a constant ratio, the sodium adduct 386.3, three products, a contaminant 391.3 at
//! 18.3 min in every file; two blanks and three QC that lose a quarter of the signal. Each check gives the flag the series was made to give, the text is the
//! same for the same table and stays inside the token cap.

use mzlab_core::json::{parse, Value};
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::tabcheck::{self, Flag, Options, IDS, MAX_TOKENS};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};

/// Kind of each file in the order of injection.
const KINDS: [&str; 11] = [
    "blank", "qc", "sample", "sample", "sample", "qc", "sample", "sample", "sample", "qc", "blank",
];
const QC_LEVEL: [f64; 3] = [1.0, 0.875, 0.75];

/// Small deterministic wobble (+-2 %) of a cell.
fn jitter(k: usize, r: usize) -> f64 {
    1.0 + 0.01 * (((k * 7 + r * 13) % 5) as f64 - 2.0)
}

/// Position of file `k` among the files of its kind.
fn rank(k: usize) -> usize {
    KINDS[..k].iter().filter(|x| **x == KINDS[k]).count()
}

fn lr() -> Options {
    Options {
        polarity: 1,
        expected_polarity: None,
        mz_tol_da: 0.3,
    }
}

fn build(
    rows: &[(f64, f64, Vec<f64>)],
    kinds: &[&str],
    groups: &[&str],
    shift: impl Fn(usize) -> f32,
) -> (Aligned, Vec<Sample>) {
    let n = kinds.len();
    let names: Vec<(String, String)> = (0..n)
        .map(|k| (format!("f{k}.mzML"), format!("{k:02x}").repeat(32)))
        .collect();
    let mut s = propose(&names);
    for k in 0..n {
        s[k].kind = kinds[k].into();
        s[k].group = groups.get(k).map_or(String::new(), |g| (*g).into());
        s[k].order = k as u32 + 1;
    }
    let mut sorted: Vec<&(f64, f64, Vec<f64>)> = rows.iter().collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut t = Aligned::with_capacity(n, rows.len());
    for (mz, rt, areas) in sorted {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for (k, &a) in areas.iter().enumerate() {
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(*rt as f32 + shift(k));
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    (t, s)
}

/// The series: `iso_ratio` is M+1/M of the parent and of its first isotope.
fn series(iso_ratio: f64, shift: impl Fn(usize) -> f32) -> (Aligned, Vec<Sample>) {
    let parent = |k: usize| -> f64 {
        match KINDS[k] {
            "blank" => 0.0,
            "qc" => 1e6 * QC_LEVEL[rank(k)],
            _ => 1e6 * (-0.5 * rank(k) as f64).exp(),
        }
    };
    let product = |k: usize, mid: f64, scale: f64| -> f64 {
        match KINDS[k] {
            "blank" => 0.0,
            "qc" => scale * QC_LEVEL[rank(k)],
            _ => scale * (0.3 + (-((rank(k) as f64 - mid).powi(2)) / 2.0).exp()),
        }
    };
    let per = |f: &dyn Fn(usize, usize) -> f64, r: usize| -> Vec<f64> {
        (0..KINDS.len()).map(|k| f(k, r)).collect()
    };
    let rows = vec![
        (364.4, 14.3, per(&|k, r| parent(k) * jitter(k, r), 0)),
        (
            365.4,
            14.3,
            per(&|k, r| parent(k) * iso_ratio * jitter(k, r), 1),
        ),
        (
            366.4,
            14.3,
            per(&|k, r| parent(k) * iso_ratio * iso_ratio * jitter(k, r), 2),
        ),
        (194.2, 14.3, per(&|k, r| parent(k) * 0.3 * jitter(k, r), 3)),
        (152.2, 14.3, per(&|k, r| parent(k) * 0.1 * jitter(k, r), 4)),
        (124.3, 14.3, per(&|k, r| parent(k) * 0.05 * jitter(k, r), 5)),
        (386.3, 14.3, per(&|k, r| parent(k) * 0.15 * jitter(k, r), 6)),
        (
            305.3,
            14.7,
            per(&|k, r| product(k, 2.0, 4e5) * jitter(k, r), 7),
        ),
        (
            229.1,
            7.07,
            per(&|k, r| product(k, 3.5, 2e5) * jitter(k, r), 8),
        ),
        (
            224.2,
            13.0,
            per(&|k, r| product(k, 1.0, 3e5) * jitter(k, r), 9),
        ),
        (391.3, 18.3, per(&|_, r| 5e4 * jitter(r, r + 1), 10)),
    ];
    build(&rows, &KINDS, &[], shift)
}

fn id_of(t: &Aligned, mz: f64) -> u32 {
    (0..t.len())
        .find(|&r| (t.mz[r] - mz).abs() < 1e-9)
        .map(|r| t.id(r))
        .unwrap_or_else(|| panic!("no row {mz}"))
}

fn find<'a>(c: &'a [tabcheck::Check], id: &str) -> &'a tabcheck::Check {
    c.iter()
        .find(|x| x.id == id)
        .unwrap_or_else(|| panic!("no check {id}"))
}

fn proof(c: &tabcheck::Check) -> Value {
    parse(c.proof.as_bytes()).expect("proof is JSON")
}

#[test]
fn the_synthetic_series_gives_the_flags_it_was_made_for() {
    let (t, s) = series(0.2, |k| 0.03 * k as f32);
    let c = tabcheck::checks(&t, &s, None, &lr()).unwrap();
    assert_eq!(c.iter().map(|x| x.id).collect::<Vec<_>>(), IDS);
    let (parent, na, contaminant) = (id_of(&t, 364.4), id_of(&t, 386.3), id_of(&t, 391.3));

    // the contaminant is in every file, blanks included, with the same area
    let k = find(&c, "contaminants");
    assert_eq!((k.flag, k.rows.clone()), (Flag::Warn, vec![contaminant]));
    let b = find(&c, "blank");
    assert_eq!(b.flag, Flag::Warn);
    assert!(b.rows.contains(&contaminant) && !b.rows.contains(&parent));

    // the QC lose a quarter of the signal: the level changes, the replicates still agree within the limit
    let q = find(&c, "qc_intensity");
    assert_eq!(q.flag, Flag::Warn);
    assert!((q.value.unwrap() + 0.25).abs() < 0.03, "{:?}", q.value);
    assert_eq!(find(&c, "qc_cv").flag, Flag::Ok);

    // the apex RT moves 0.03 min per injection against a window of about 0.06
    let d = find(&c, "rt_drift");
    assert_eq!(d.flag, Flag::Warn);
    assert!(d.value.unwrap() > d.threshold.unwrap());

    // the sodium adduct of the parent
    let a = find(&c, "adducts");
    assert_eq!(a.flag, Flag::Note);
    assert!(
        a.rows.contains(&parent) && a.rows.contains(&na),
        "{:?}",
        a.rows
    );
    assert!(a.proof.contains("[M+Na]+"));

    // the isotopes are inside the band of their carbon count
    let i = find(&c, "isotopes");
    assert_eq!(i.flag, Flag::Ok);
    assert_eq!(proof(i).get("pairs").and_then(Value::f64), Some(2.0));

    // the three fragments at a constant ratio
    let f = find(&c, "insource");
    assert_eq!(f.flag, Flag::Note);
    for mz in [364.4, 194.2, 152.2, 124.3] {
        assert!(f.rows.contains(&id_of(&t, mz)), "{mz}: {:?}", f.rows);
    }

    assert_eq!(find(&c, "duplicates").flag, Flag::Ok);
    assert_eq!(find(&c, "size").flag, Flag::Ok);
    assert_eq!(find(&c, "polarity_units").flag, Flag::Ok);
    // unit resolution has no accurate mass to read a Kendrick series from
    assert_eq!(find(&c, "homologous").value, None);
    // the nine injections of samples and QC are enough for the PCA
    assert!(find(&c, "structure").value.is_some());
}

#[test]
fn without_drift_the_rt_check_is_quiet_and_the_text_is_stable_and_small() {
    let (t, s) = series(0.2, |_| 0.0);
    let c = tabcheck::checks(&t, &s, None, &lr()).unwrap();
    assert_eq!(find(&c, "rt_drift").flag, Flag::Ok);
    let a = tabcheck::to_json(&t, &s, None, &lr()).unwrap();
    assert_eq!(a, tabcheck::to_json(&t, &s, None, &lr()).unwrap());
    assert!(tabcheck::tokens(&a) <= MAX_TOKENS);
    let v = parse(a.as_bytes()).unwrap();
    assert_eq!(v.get("schema").and_then(Value::str), Some("tabcheck/1"));
    let ids: Vec<&str> = v
        .get("checks")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|c| c.get("id").and_then(Value::str))
        .collect();
    assert_eq!(ids, IDS);
    for c in v.get("checks").unwrap().arr() {
        assert!(c.get("rows").unwrap().arr().len() <= tabcheck::ROWS_CAP);
        assert!(["ok", "note", "warn"].contains(&c.get("flag").and_then(Value::str).unwrap()));
    }
}

#[test]
fn a_first_isotope_as_strong_as_the_parent_is_outside_the_band() {
    let (t, s) = series(1.0, |_| 0.0);
    let c = tabcheck::checks(&t, &s, None, &lr()).unwrap();
    let i = find(&c, "isotopes");
    assert_eq!(i.flag, Flag::Note);
    assert!(i.rows.contains(&id_of(&t, 364.4)) && i.rows.contains(&id_of(&t, 365.4)));
}

#[test]
fn a_row_split_in_two_is_a_duplicate() {
    let flat = vec![1e6; 5];
    let rows = vec![
        (300.1000, 5.0, flat.clone()),
        (300.1010, 5.01, flat.clone()),
        (450.2, 8.0, flat),
    ];
    let (t, s) = build(&rows, &["sample"; 5], &[], |_| 0.0);
    let opts = Options::default();
    let c = tabcheck::checks(&t, &s, None, &opts).unwrap();
    let d = find(&c, "duplicates");
    assert_eq!((d.flag, d.value), (Flag::Warn, Some(1.0)));
    // the heavier row of the pair first
    assert_eq!(d.rows, vec![2, 1]);
}

#[test]
fn rows_a_unit_apart_in_a_high_resolution_table_are_a_homologous_series() {
    const CF2: f64 = 49.996_810_5;
    let rows: Vec<(f64, f64, Vec<f64>)> = (0..6)
        .map(|j| {
            (
                212.9792 + CF2 * j as f64,
                4.0 + 0.8 * j as f64,
                (0..5)
                    .map(|k| 1e5 * (1.0 + 0.1 * ((j + k) % 3) as f64))
                    .collect(),
            )
        })
        .chain([(100.0521, 2.0, vec![1e5; 5]), (777.7777, 9.0, vec![1e5; 5])])
        .collect();
    let (t, s) = build(&rows, &["sample"; 5], &[], |_| 0.0);
    let c = tabcheck::checks(&t, &s, None, &Options::default()).unwrap();
    let h = find(&c, "homologous");
    assert_eq!(h.flag, Flag::Note, "{}", h.proof);
    assert!(h.rows.len() >= 5 && h.rows.contains(&id_of(&t, 212.9792)));
}

#[test]
fn missing_cells_concentrated_in_one_group_are_flagged() {
    let kinds = ["sample"; 6];
    let groups = ["A", "A", "A", "B", "B", "B"];
    let rows: Vec<(f64, f64, Vec<f64>)> = (0..10)
        .map(|r| {
            (
                200.0 + r as f64 * 10.0,
                3.0 + r as f64,
                (0..6)
                    .map(|k| if k >= 3 && r % 3 != 0 { 0.0 } else { 1e5 })
                    .collect(),
            )
        })
        .collect();
    let (t, s) = build(&rows, &kinds, &groups, |_| 0.0);
    let c = tabcheck::checks(&t, &s, None, &Options::default()).unwrap();
    let m = find(&c, "missing");
    assert_eq!(m.flag, Flag::Warn, "{}", m.proof);
    assert_eq!(m.files, vec!["f3.mzML", "f4.mzML", "f5.mzML"]);
    // the table has no blank and no QC: those checks say what they lack, with no number
    let b = find(&c, "blank");
    assert_eq!((b.flag, b.value), (Flag::Note, None));
    assert!(b.proof.contains("\"missing\":\"blank\""));
    assert_eq!(find(&c, "qc_cv").value, None);
}

#[test]
fn a_polarity_the_files_do_not_have_and_minutes_that_are_seconds_are_flagged() {
    let rows = vec![(300.0, 600.0, vec![1e5; 4]), (400.0, 900.0, vec![1e5; 4])];
    let (t, s) = build(&rows, &["sample"; 4], &[], |_| 0.0);
    let opts = Options {
        polarity: 1,
        expected_polarity: Some(-1),
        mz_tol_da: 0.005,
    };
    let c = tabcheck::checks(&t, &s, None, &opts).unwrap();
    let p = find(&c, "polarity_units");
    assert_eq!((p.flag, p.value), (Flag::Warn, Some(2.0)));
    assert!(p.proof.contains("polarity_mismatch") && p.proof.contains("rt_not_minutes"));
}

#[test]
fn an_empty_table_and_a_wrong_number_of_samples_are_handled() {
    let (t, s) = series(0.2, |_| 0.0);
    assert!(tabcheck::checks(&t, &s[1..], None, &lr()).is_err());
    let empty = Aligned::with_capacity(2, 0);
    let s2 = propose(&[
        ("a.mzML".into(), "00".repeat(32)),
        ("b.mzML".into(), "11".repeat(32)),
    ]);
    let c = tabcheck::checks(&empty, &s2, None, &Options::default()).unwrap();
    assert_eq!(c.len(), 15);
    assert!(c.iter().all(|x| x.flag == Flag::Note && x.value.is_none()));
}

#[test]
fn a_text_over_the_cap_loses_examples_not_checks() {
    // 30 files with names of 6000 characters, 8 of them with a single row: the list of files of the size check alone is larger than the cap
    let n = 30;
    let names: Vec<(String, String)> = (0..n)
        .map(|k| {
            (
                format!("{}{k}.mzML", "x".repeat(6000)),
                format!("{k:02x}").repeat(32),
            )
        })
        .collect();
    let mut s = propose(&names);
    let mut t = Aligned::with_capacity(n, 40);
    for r in 0..40 {
        t.mz.push(200.0 + r as f64);
        t.rt.push(5.0);
        for k in 0..n {
            let on = k < 22 || r == 0;
            t.area.push(if on { 1e5 } else { 0.0 });
            t.height.push(1.0);
            t.apex_rt.push(5.0);
            t.width.push(0.1);
            t.state.push(if on { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    s.iter_mut().for_each(|x| x.kind = "sample".into());
    let text = tabcheck::to_json(&t, &s, None, &Options::default()).unwrap();
    assert!(
        tabcheck::tokens(&text) <= MAX_TOKENS,
        "{}",
        tabcheck::tokens(&text)
    );
    let v = parse(text.as_bytes()).unwrap();
    assert_eq!(v.get("truncated").and_then(Value::bool), Some(true));
    assert_eq!(v.get("checks").unwrap().arr().len(), 15);
}
