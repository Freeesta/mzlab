//! Kinetic evidence on the synthetic identity series (V1) with known kinetics: right shape above 10 times the noise, generation right or
//! "undecidable", share of decoys (noise coincidences, random formation rates) with a coherent kinetics, time per 1000 identities.
//! Areas: `tests/golden/feat/cinetica_v1.json` (`tools/feat_riferimento.py OUT --solo cinetica --golden`). `--nocapture` prints the tables.

use std::collections::BTreeMap;
use std::time::Instant;

use mzlab_feat::kinetic::{
    kinetic_report, parent_kinetics, ChainBank, Form, FormBank, Gen, ParentSource, Report, Setup,
    Sign, K_PARENT_MIN,
};
use serde_json::Value;

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

struct Case {
    id: String,
    role: String,
    form: String,
    y: Vec<f64>,
    ctrl: Vec<f64>,
    report: Report,
}

fn load() -> (Value, Vec<f64>) {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/feat/cinetica_v1.json");
    let doc: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    let t = floats(&doc["times"]);
    (doc, t)
}

fn run(doc: &Value, t: &[f64], key: &str) -> (Vec<Case>, Report) {
    let (forms, chain) = (FormBank::new(t), ChainBank::new(t, K_PARENT_MIN));
    let (py, piso) = (
        floats(&doc["parent"]["y"]),
        floats(&doc["parent"]["y_isotope"]),
    );
    let parent = parent_kinetics(&forms, t, &py, Some(&piso));
    let s = Setup {
        times: t,
        forms: &forms,
        chain: &chain,
        parent,
        pseudo: doc["pseudo"].as_f64().unwrap(),
    };
    let cases = doc[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            let (y, ctrl) = (floats(&d["y"]), floats(&d["ctrl"]));
            let report = kinetic_report(&s, &y, &ctrl);
            Case {
                id: d["id"].as_str().unwrap().into(),
                role: d["ruolo"].as_str().unwrap().into(),
                form: d["forma"].as_str().unwrap().into(),
                y,
                ctrl,
                report,
            }
        })
        .collect();
    (cases, kinetic_report(&s, &py, &[0.0, 0.0]))
}

/// Shapes accepted as right for a true kinetics of V1.
fn shape_ok(c: &Case) -> bool {
    let Some(f) = c.report.shape else {
        return false;
    };
    let instant =
        f == Form::Formation && c.report.coupling == mzlab_feat::kinetic::Coupling::Unresolved;
    match c.form.as_str() {
        "monotona" => f == Form::Formation,
        "bateman" => f == Form::Bateman,
        "piatto" => f == Form::Flat || instant,
        "decrescente" => f == Form::Decreasing,
        "ritardo" => matches!(f, Form::Bateman | Form::Formation),
        _ => false,
    }
}

fn pct(a: usize, n: usize) -> f64 {
    100.0 * a as f64 / n.max(1) as f64
}

fn loud(c: &Case) -> bool {
    c.y.iter().copied().fold(0.0, f64::max) > 10.0 * 3e5
}

/// n, right shape, generation right or undecidable, undecidable, for, against: per true shape.
fn table(cases: &[Case]) -> BTreeMap<String, [usize; 6]> {
    let mut by: BTreeMap<String, [usize; 6]> = BTreeMap::new();
    for c in cases
        .iter()
        .filter(|c| matches!(c.role.as_str(), "tp1" | "isomero" | "tp2") && loud(c))
    {
        let e = by.entry(c.form.clone()).or_default();
        let g = c.report.generation.map(|g| g.verdict);
        let want = if c.role == "tp2" {
            Gen::Second
        } else {
            Gen::First
        };
        let sign = c.report.evidence.sign;
        for (k, v) in [
            (0, true),
            (1, shape_ok(c)),
            (2, g == Some(want) || g == Some(Gen::Undecidable)),
            (3, g == Some(Gen::Undecidable)),
            (4, sign == Sign::For),
            (5, sign == Sign::Against),
        ] {
            e[k] += v as usize;
        }
    }
    by
}

fn print_table(title: &str, by: &BTreeMap<String, [usize; 6]>) {
    println!(
        "{title}\n{:<12} {:>4} {:>8} {:>10} {:>9} {:>6} {:>8}",
        "true shape", "n", "shape ok", "gen ok/und", "undecid.", "for", "against"
    );
    for (k, e) in by {
        println!(
            "{:<12} {:>4} {:>7.1}% {:>9.1}% {:>8.1}% {:>5.1}% {:>7.1}%",
            k,
            e[0],
            pct(e[1], e[0]),
            pct(e[2], e[0]),
            pct(e[3], e[0]),
            pct(e[4], e[0]),
            pct(e[5], e[0])
        );
    }
}

fn share(cases: &[&Case], sign: Sign) -> f64 {
    pct(
        cases
            .iter()
            .filter(|c| c.report.evidence.sign == sign)
            .count(),
        cases.len(),
    )
}

#[test]
fn measure_on_v1() {
    let (doc, t) = load();
    let (cases, _) = run(&doc, &t, "ions");
    let (coupled, _) = run(&doc, &t, "ions_coupled");
    let by = table(&cases);
    print_table(
        "V1 as built (formation rates independent of the parent)",
        &by,
    );
    print_table(
        "same products with k_form = k_parent (direct route)",
        &table(&coupled),
    );
    let tot = by.values().fold([0usize; 6], |mut a, e| {
        a.iter_mut().zip(e).for_each(|(x, y)| *x += y);
        a
    });
    println!(
        "shape right above 10x noise: {:.1}% of {}",
        pct(tot[1], tot[0]),
        tot[0]
    );
    let dec: Vec<(&str, Vec<&Case>)> = ["coincidenza", "esca_casuale"]
        .iter()
        .map(|r| (*r, cases.iter().filter(|c| c.role == *r).collect()))
        .collect();
    for (role, d) in &dec {
        println!(
            "{role}: n={} for={:.1}% against={:.1}%",
            d.len(),
            share(d, Sign::For),
            share(d, Sign::Against)
        );
    }
    let tp: Vec<&Case> = coupled.iter().filter(|c| loud(c)).collect();
    println!(
        "coupled products: n={} for={:.1}% against={:.1}%",
        tp.len(),
        share(&tp, Sign::For),
        share(&tp, Sign::Against)
    );
    assert!(
        cases
            .iter()
            .chain(&coupled)
            .all(|c| c.report.evidence.points == 0.0),
        "informative only: no points"
    );
    assert!(coupled.iter().any(|c| c
        .report
        .evidence
        .params
        .iter()
        .any(|(k, v)| *k == "points_if_weighed" && *v > 0.0)));
    assert!(pct(tot[1], tot[0]) >= 90.0, "shape");
    for (k, e) in &by {
        assert!(pct(e[2], e[0]) >= 90.0, "generation {k}");
    }
    assert!(
        share(&dec[0].1, Sign::For) < 5.0,
        "noise coincidences with a coherent kinetics"
    );
}

#[test]
fn parent_saturation_and_speed() {
    let (doc, t) = load();
    let (cases, parent) = run(&doc, &t, "ions");
    let (forms, chain) = (FormBank::new(&t), ChainBank::new(&t, K_PARENT_MIN));
    let (py, piso) = (
        floats(&doc["parent"]["y"]),
        floats(&doc["parent"]["y_isotope"]),
    );
    let pk = parent_kinetics(&forms, &t, &py, Some(&piso));
    println!(
        "parent: k={:?} true={} saturated={} source={:?} shape={:?}",
        pk.k, doc["parent"]["k_dec"], pk.saturated, pk.source, parent.shape
    );
    let s = Setup {
        times: &t,
        forms: &forms,
        chain: &chain,
        parent: pk,
        pseudo: 1e6,
    };
    let t0 = Instant::now();
    for i in 0..1000 {
        let c = &cases[i % cases.len()];
        let _ = kinetic_report(&s, &c.y, &c.ctrl);
    }
    println!(
        "1000 identities: {:.3} s (banks built before: {:?})",
        t0.elapsed().as_secs_f64(),
        ParentSource::Direct
    );
}

#[test]
fn evidence_two_runs_same_bytes() {
    use sha2::{Digest, Sha256};
    let (doc, t) = load();
    let dump = || {
        let (cases, _) = run(&doc, &t, "ions");
        let mut h = Sha256::new();
        for c in &cases {
            let e = &c.report.evidence;
            h.update(c.id.as_bytes());
            h.update([e.sign as u8]);
            for v in [e.value, e.threshold, e.points].into_iter().chain(e.params.iter().map(|p| p.1)) {
                h.update(v.to_le_bytes());
            }
        }
        h.finalize()
    };
    assert_eq!(dump(), dump());
}
