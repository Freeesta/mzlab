//! The rules `table.filter` gives an NTS workflow: blanks matched by batch or matrix, replicate presence, normalisation by the internal standard and the
//! internal standard kept from the blank rule. Ten files: two blanks in each of two batches, then two groups of three replicates (one per batch).

use mzlab_feat::filters_blank::Labels;
use mzlab_feat::norm::{self, Method, Reference};
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::view;

const N: usize = 10;
// rows (indices): the contaminant, seen in 2 of 3, seen in 1 of 3, the standard (id 4)
const CONTAM: u32 = 0;
const TWO: u32 = 1;
const ONE: u32 = 2;
const STD: u32 = 3;

fn samples() -> Vec<Sample> {
    let names: Vec<(String, String)> = (0..N)
        .map(|k| (format!("f{k}.mzML"), "00".repeat(32)))
        .collect();
    let mut s = propose(&names);
    for (k, x) in s.iter_mut().enumerate() {
        x.kind = if k < 4 { "blank" } else { "sample" }.into();
        x.time_min = if k < 7 { Some(10.0) } else { Some(20.0) };
        x.replicate = (k as u32 + 4 - 4) % 3 + 1;
    }
    s
}

fn labels(batch: bool) -> Labels {
    let b: Vec<String> = (0..N)
        .map(|k| {
            if k == 2 || k == 3 || k >= 7 {
                "B2"
            } else {
                "B1"
            }
            .to_string()
        })
        .collect();
    Labels {
        batch: if batch { b } else { Vec::new() },
        ..Labels::default()
    }
}

/// Rows, areas per file (0 = absent): 0 is in the blank of batch 2 only, 1 is seen in 2 of 3 replicates of one group, 2 in 1 of 3 of every group, 3 is the standard.
fn table() -> Aligned {
    let rows: [(f64, [f64; N]); 4] = [
        (
            200.0,
            [
                0.0, 0.0, 1000.0, 1000.0, 0.0, 0.0, 0.0, 2000.0, 2000.0, 2000.0,
            ],
        ),
        (
            210.0,
            [0.0, 0.0, 0.0, 0.0, 5000.0, 5000.0, 0.0, 0.0, 0.0, 5000.0],
        ),
        (
            220.0,
            [0.0, 0.0, 0.0, 0.0, 5000.0, 0.0, 0.0, 5000.0, 0.0, 0.0],
        ),
        (
            230.0,
            [1e6, 1.2e6, 1e6, 1.2e6, 1e6, 2e6, 1e6, 1.5e6, 1e6, 1e6],
        ),
    ];
    let mut t = Aligned::with_capacity(N, rows.len());
    for (mz, areas) in rows {
        t.mz.push(mz);
        t.rt.push(5.0);
        for a in areas {
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(5.0);
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

fn kept(spec: &str) -> Vec<u32> {
    let v = view::from_spec(&table(), &samples(), spec.as_bytes()).unwrap();
    v.rows
}

fn spec(rules: &str, method: &str, labels: &Labels) -> String {
    let l = if labels.is_empty() {
        String::new()
    } else {
        format!(",\"etichette\":{}", labels.to_json())
    };
    format!("{{\"normalizza\":{method},\"filtra\":[{rules}]{l}}}")
}

const NONE: &str = r#"{"metodo":"nessuna"}"#;
const ISTD: &str = r#"{"metodo":"istd","istd":{"id":4}}"#;

fn blank(extra: &str) -> String {
    format!(r#"{{"regola":"bianco","valore":3,"statistica":"media"{extra}}}"#)
}

#[test]
fn a_blank_of_batch_2_only_removes_the_row_for_batch_2_with_batch_matching() {
    // the batch-2 blanks hold 1000 against 0 in batch 1: the mean of all four (500) lets 2000 through, the matched level (1000) does not
    let all = kept(&spec(&blank(""), NONE, &Labels::default()));
    assert!(
        all.contains(&CONTAM),
        "all blanks dilute the contaminant: {all:?}"
    );
    let matched = kept(&spec(
        &blank(r#","abbinamento":"lotto""#),
        NONE,
        &labels(true),
    ));
    assert!(
        !matched.contains(&CONTAM),
        "matched blanks remove it: {matched:?}"
    );
    // the rows with no blank signal are unaffected
    assert!(matched.contains(&TWO) && matched.contains(&ONE));
}

#[test]
fn a_sample_with_no_blank_of_its_batch_falls_back_to_all_blanks_and_says_so() {
    let mut l = labels(true);
    l.batch[7] = "B3".into();
    l.batch[8] = "B3".into();
    l.batch[9] = "B3".into();
    let s = spec(&blank(r#","abbinamento":"lotto""#), NONE, &l);
    let v = view::from_spec(&table(), &samples(), s.as_bytes()).unwrap();
    let f = v.fallback.clone().unwrap();
    assert_eq!(f.iter().filter(|&&x| x).count(), 3, "{f:?}");
    assert!(f[7] && f[8] && f[9] && !f[4]);
    assert!(v.rows.contains(&CONTAM), "all blanks again");
    assert!(v.counts_json().contains("\"bianco_ripiego\":[false"));
    // without a matching rule nothing is recorded
    let plain = view::from_spec(
        &table(),
        &samples(),
        spec(&blank(""), NONE, &Labels::default()).as_bytes(),
    )
    .unwrap();
    assert!(plain.fallback.is_none() && !plain.counts_json().contains("ripiego"));
}

#[test]
fn matching_by_matrix_reads_the_matrix_labels() {
    let l = Labels {
        matrix: labels(true).batch,
        ..Labels::default()
    };
    let got = kept(&spec(&blank(r#","abbinamento":"matrice""#), NONE, &l));
    assert!(!got.contains(&CONTAM));
    // a rule that matches batches with no batch label compares with all blanks
    let none = kept(&spec(
        &blank(r#","abbinamento":"lotto""#),
        NONE,
        &Labels::default(),
    ));
    assert_eq!(none, kept(&spec(&blank(""), NONE, &Labels::default())));
}

#[test]
fn a_replicate_fraction_keeps_two_of_three_in_one_group_and_drops_one_of_three_everywhere() {
    let rule = r#"{"regola":"repliche_min","valore":0.67}"#;
    let got = kept(&spec(rule, NONE, &Labels::default()));
    assert!(
        got.contains(&TWO),
        "row seen in 2 of 3 replicates of a group: {got:?}"
    );
    assert!(
        !got.contains(&ONE),
        "row seen in 1 of 3 in every group: {got:?}"
    );
    assert!(
        got.contains(&CONTAM) && got.contains(&STD),
        "present in all: {got:?}"
    );
    // a group set in the labels splits the files differently: all six samples in one group, 3 of 6 is below 0.67
    let l = Labels {
        group: (0..N)
            .map(|k| {
                if k < 4 {
                    String::new()
                } else {
                    "all".to_string()
                }
            })
            .collect(),
        ..Labels::default()
    };
    let one = kept(&spec(rule, NONE, &l));
    assert!(!one.contains(&TWO));
    // a fraction outside 0..1 is refused
    assert!(view::from_spec(
        &table(),
        &samples(),
        spec(
            r#"{"regola":"repliche_min","valore":1.5}"#,
            NONE,
            &Labels::default()
        )
        .as_bytes()
    )
    .is_err());
}

#[test]
fn normalising_by_the_standard_gives_the_factors_of_norm() {
    let v = view::from_spec(
        &table(),
        &samples(),
        spec("", ISTD, &Labels::default()).as_bytes(),
    )
    .unwrap();
    let want = norm::factors(&table(), &samples(), &Method::Istd(Reference::Id(4))).unwrap();
    assert_eq!(
        v.factors.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        want.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
    );
    assert!(v.factors.iter().all(|f| f.is_finite()));
}

#[test]
fn the_standard_row_can_skip_the_blank_rule() {
    // the standard is in the blanks at 1e6: the blank rule (3 x the largest blank) drops it unless it is kept
    let max = r#","statistica":"massimo""#;
    let drop = |extra: &str| {
        kept(&spec(
            &format!(r#"{{"regola":"bianco","valore":3{max}{extra}}}"#),
            ISTD,
            &Labels::default(),
        ))
    };
    assert!(!drop("").contains(&STD));
    assert!(drop(r#","istd_esente":true"#).contains(&STD));
    // there must be a standard to keep
    let s = spec(&blank(r#","istd_esente":true"#), NONE, &Labels::default());
    assert!(view::from_spec(&table(), &samples(), s.as_bytes()).is_err());
}

#[test]
fn the_defaults_write_the_old_json_and_the_new_rules_round_trip() {
    let plain = r#"{"normalizza":{"metodo":"nessuna"},"filtra":[{"regola":"bianco","valore":3,"grandezza":"area","statistica":"massimo","ambito":"max","sottrai":false}]}"#;
    let s = r#"[{"regola":"bianco","valore":3,"statistica":"massimo"}]"#;
    let v = view::from_spec(
        &table(),
        &samples(),
        format!("{{\"filtra\":{s}}}").as_bytes(),
    )
    .unwrap();
    assert_eq!(v.spec_json(), plain);
    let full = spec(
        &blank(r#","abbinamento":"matrice","istd_esente":true"#),
        ISTD,
        &labels(true),
    );
    let v = view::from_spec(&table(), &samples(), full.as_bytes()).unwrap();
    let again = view::from_spec(&table(), &samples(), v.spec_json().as_bytes()).unwrap();
    assert_eq!(v.spec_json(), again.spec_json());
    assert_eq!(v.digest(&table()), again.digest(&table()));
}
