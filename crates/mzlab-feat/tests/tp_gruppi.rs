//! The conditions in the table of the products (`tptab/1` with `gruppi/1`) and in the network (`arcs_in`): synthetic data only.

use mzlab_core::campioni::{number_replicates, Campione};
use mzlab_core::json::{parse, Value};
use mzlab_feat::hypo::Rules;
use mzlab_feat::samples::Sample;
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::tp::*;

const O: f64 = 15.994915;

fn file(name: &str, kind: &str, series: &str, minutes: Option<f64>, order: u32) -> Campione {
    let mut c = Campione::new(name, "", order);
    c.kind = kind.into();
    c.series = series.into();
    c.set_time(minutes, "min");
    c
}

/// The `campioni/1` of the same files (the table reads the roles from it).
fn samples_of(cs: &[Campione]) -> Vec<Sample> {
    cs.iter()
        .map(|c| Sample {
            file: c.file.clone(),
            sha256: String::new(),
            kind: c.kind.clone(),
            time_min: c.time_min,
            replicate: 1,
            group: String::new(),
            conc: None,
            unit: None,
            order: c.order,
            istd: false,
            note: String::new(),
        })
        .collect()
}

fn table(rows: &[(f64, f64, Vec<f64>)], n_files: usize) -> Aligned {
    let mut t = Aligned::with_capacity(n_files, rows.len());
    for (mz, rt, a) in rows {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for v in a {
            t.area.push(*v);
            t.height.push((v / 10.0) as f32);
            t.apex_rt.push(*rt as f32);
            t.width.push(0.1);
            t.state
                .push(if *v > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t
}

/// blank, t0 (shared) and one 60 min file for each of the six conditions.
fn endpoint() -> (Aligned, Vec<Campione>) {
    let mut cs = vec![
        file("blank", "blank", "", None, 1),
        file("t0", "t0", "", Some(0.0), 2),
    ];
    for (i, g) in ["G1", "G2", "G3", "G4", "G5", "G6"].iter().enumerate() {
        cs.push(file(
            &format!("{g}_60"),
            "sample",
            g,
            Some(60.0),
            3 + i as u32,
        ));
    }
    number_replicates(&mut cs);
    let rows = [
        (300.0, 5.0, vec![0.0, 1e7, 3e6, 2e6, 4e6, 1e6, 5e6, 2e6]),
        (316.0, 4.0, vec![0.0, 0.0, 9e5, 8e5, 7e5, 9e5, 8e5, 6e5]),
        (330.0, 6.0, vec![0.0, 0.0, 0.0, 5e5, 0.0, 0.0, 4e5, 0.0]),
        (332.0, 4.5, vec![0.0, 0.0, 0.0, 0.0, 6e5, 0.0, 0.0, 0.0]),
    ];
    (table(&rows, 8), cs)
}

fn infos() -> Vec<RowInfo> {
    vec![
        RowInfo {
            row: 1,
            neutral_formula: Some("C10H12O".into()),
            ion_formula: Some("C10H13O+".into()),
            ppm: Some(-1.2),
        },
        RowInfo {
            row: 2,
            neutral_formula: Some("C10H12O2".into()),
            ion_formula: None,
            ppm: Some(0.4),
        },
    ]
}

fn texts<'a>(v: &'a Value, key: &str) -> Vec<&'a str> {
    v.get(key)
        .map(|a| a.arr())
        .unwrap_or_default()
        .iter()
        .filter_map(|x| x.str())
        .collect()
}

#[test]
fn six_conditions_give_gruppi_with_the_formulas_and_the_presence_of_the_truth() {
    let (t, cs) = endpoint();
    let s = samples_of(&cs);
    let out = table_json_cond(&t, &s, Some(&[0, 1, 2, 3]), Some((&cs, &infos()))).unwrap();
    let v = parse(out.as_bytes()).unwrap();
    let g = v.get("gruppi").expect("gruppi/1 with six conditions");
    assert_eq!(g.get("format").and_then(|x| x.str()), Some("gruppi"));
    assert_eq!(g.get("version").and_then(|x| x.f64()), Some(1.0));
    let rows = g.get("rows").unwrap().arr();
    let present: Vec<Vec<&str>> = rows.iter().map(|r| texts(r, "present_in")).collect();
    assert_eq!(present[1], ["G1", "G2", "G3", "G4", "G5", "G6"]);
    assert_eq!(present[2], ["G2", "G5"]);
    assert_eq!(present[3], ["G3"]);
    assert_eq!(rows[3].get("only_in").and_then(|x| x.str()), Some("G3"));
    assert_eq!(
        rows[1].get("neutral_formula").and_then(|x| x.str()),
        Some("C10H12O")
    );
    assert_eq!(
        rows[1].get("ion_formula").and_then(|x| x.str()),
        Some("C10H13O+")
    );
    assert_eq!(rows[1].get("ppm").and_then(|x| x.f64()), Some(-1.2));
    assert!(
        rows[3]
            .get("neutral_formula")
            .is_none_or(|x| x.str().is_none()),
        "no hypothesis, no formula"
    );
    // the network note: one time per condition is below the minimum
    let n = v.get("network").unwrap();
    assert_eq!(n.get("min_times").and_then(|x| x.f64()), Some(4.0));
    assert!(n
        .get("conditions")
        .unwrap()
        .arr()
        .iter()
        .all(|c| c.get("used").and_then(|x| x.bool()) == Some(false)));
}

#[test]
fn the_row_is_read_in_the_condition_where_it_is_strongest() {
    let (t, cs) = endpoint();
    let s = samples_of(&cs);
    let v = parse(
        table_json_cond(&t, &s, Some(&[1, 2]), Some((&cs, &[])))
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    let rows = v.get("rows").unwrap().arr();
    let series = |i: usize| -> Vec<f64> {
        rows[i]
            .get("series")
            .unwrap()
            .arr()
            .iter()
            .filter_map(|x| x.f64())
            .collect()
    };
    // row 316: strongest in G1 (9e5), the others count 0; row 330: in G2 (5e5)
    assert_eq!(series(0), [9e5, 0.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(series(1), [0.0, 5e5, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(
        v.get("times").unwrap().arr().len(),
        6,
        "the axis is the one of all the treated files"
    );
}

#[test]
fn with_one_condition_the_text_is_the_one_without_conditions() {
    let (t, mut cs) = endpoint();
    for c in cs.iter_mut().filter(|c| c.kind == "sample") {
        c.series = "G1".into();
    }
    let s = samples_of(&cs);
    let plain = table_json(&t, &s, Some(&[0, 1, 2, 3])).unwrap();
    let one = table_json_cond(&t, &s, Some(&[0, 1, 2, 3]), Some((&cs, &infos()))).unwrap();
    assert_eq!(plain, one);
    for c in cs.iter_mut() {
        c.series = String::new();
    }
    assert_eq!(
        plain,
        table_json_cond(&t, &s, Some(&[0, 1, 2, 3]), Some((&cs, &[]))).unwrap()
    );
    assert!(!plain.contains("gruppi"));
}

/// blank, t0 and five times for each of the conditions A and B.
const TIMES: [f64; 5] = [5.0, 10.0, 20.0, 40.0, 80.0];

fn kinetics(n: usize) -> (Aligned, Vec<Campione>, Vec<f64>, Vec<Vec<usize>>) {
    let mut cs = vec![
        file("blank", "blank", "", None, 1),
        file("t0", "t0", "", Some(0.0), 2),
    ];
    let mut minutes = vec![f64::NAN, 0.0];
    for g in ["A", "B"] {
        for (i, m) in TIMES.iter().take(n).enumerate() {
            cs.push(file(
                &format!("{g}_{m}"),
                "sample",
                g,
                Some(*m),
                3 + cs.len() as u32 + i as u32,
            ));
            minutes.push(*m);
        }
    }
    number_replicates(&mut cs);
    let by = treated_by_condition(&cs);
    let decay = [8e6, 6e6, 4e6, 2e6, 1e6];
    let up = [1e5, 3e5, 6e5, 9e5, 1.2e6];
    let down = [1.2e6, 9e5, 6e5, 3e5, 1e5];
    let pad = |a: &[f64], b: &[f64]| -> Vec<f64> {
        let mut v = vec![0.0; 2];
        v[1] = 1e7;
        v.extend(a.iter().take(n));
        v.extend(b.iter().take(n));
        v
    };
    // the parent decays in both; the product rises in A and (after a spike at t0 of nothing) falls in B
    let rows = [
        (300.0, 5.0, pad(&decay, &decay)),
        (300.0 + O, 4.5, {
            let mut v = pad(&up, &down);
            v[1] = 0.0;
            v
        }),
    ];
    (table(&rows, 2 + 2 * n), cs, minutes, by)
}

fn rules() -> Rules {
    let j = r#"{"format":"regole_tp","version":1,"regole":[{"id":"oh","nome":"hydroxylation","delta_formula":"+O","passi_max":1,"livello_previsione":3,"fonte":"transformation_table","direzione_rt":"piu_polare"}]}"#;
    Rules::parse(j.as_bytes()).unwrap()
}

fn arcs_of(n: usize, by_cond: bool) -> Vec<(u32, u32)> {
    let (t, _, minutes, by) = kinetics(n);
    let r = rules();
    let signs = vec![[0u8; 10]; t.len()];
    let params = TpParams::default();
    let inp = TpInputs {
        aligned: &t,
        minutes: &minutes,
        hyps: &[],
        signs: &signs,
        parent: (300.0, 5.0),
        fragments: None,
        params: &params,
        rules: Some(&r),
    };
    let b = build(&inp).unwrap().expect("parent in the table");
    let used = if by_cond {
        arcs_in(&inp, &r, &b, &NetParams::default(), &by)
    } else {
        arcs(&inp, &r, &b, &NetParams::default())
    };
    used.iter().map(|a| (a.from, a.to)).collect()
}

#[test]
fn the_network_is_read_inside_each_condition_and_one_is_enough() {
    // A: the product rises while the parent decays (an arc); B: it falls with the parent (the opposite coupling, no arc by itself)
    assert_eq!(arcs_of(5, true), [(0, 1)]);
}

#[test]
fn a_condition_with_fewer_than_four_times_gives_no_arc() {
    assert!(arcs_of(3, true).is_empty());
}

#[test]
fn one_condition_reads_as_the_old_network() {
    let (_, cs, _, _) = kinetics(5);
    let one: Vec<Campione> = cs
        .into_iter()
        .map(|mut c| {
            if c.kind == "sample" {
                c.series = "A".into();
            }
            c
        })
        .collect();
    assert!(
        treated_by_condition(&one).is_empty(),
        "one condition: no lists, the old reading"
    );
    // pooled over both conditions the product has no clear coupling: only the per-condition reading finds the arc of A
    assert!(arcs_of(5, false).is_empty());
}
