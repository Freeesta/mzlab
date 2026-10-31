//! Benchmark of the peak-integration candidates on synthetic XIC traces with known areas (docs/studi/INTEGRAZIONE_PICCHI.md).
//! Each case is drawn with 30 noise seeds; the table (area error, boundary error, missed and false peaks, time) is printed with
//! `cargo test --release -p mzlab-core --test integrazione_bench -- --nocapture`; the asserts keep the chosen method's numbers.

#[path = "integrazione_bench/candidati.rs"]
mod candidati;
#[path = "integrazione_bench/tracce.rs"]
mod tracce;

use candidati::*;
use std::time::Instant;
use tracce::*;

#[derive(Default, Clone)]
struct Score {
    area_err: Vec<f64>,
    edge_err: Vec<f64>,
    missed: usize,
    falses: usize,
    truths: usize,
    nanos: u128,
    runs: usize,
}

fn score(m: Method, c: &Case) -> Score {
    let mut sc = Score::default();
    for seed in 0..SEEDS {
        let (rt, y, shapes) = draw(c, seed);
        let clicks: Vec<f64> = if c.peaks.is_empty() {
            vec![3.0]
        } else {
            shapes.iter().map(|s| rt[true_limits(s).1]).collect()
        };
        let t0 = Instant::now();
        let found = m(&rt, &y, &clicks);
        sc.nanos += t0.elapsed().as_nanos();
        sc.runs += 1;
        let mut used = vec![false; found.len()];
        for (p, s) in c.peaks.iter().zip(&shapes) {
            sc.truths += 1;
            let (lo, apex, hi) = true_limits(s);
            // the window that holds the true apex, nearest centre first
            let best = found
                .iter()
                .enumerate()
                .filter(|(k, w)| !used[*k] && w.0 <= apex && apex <= w.1)
                .min_by_key(|(_, w)| ((w.0 + w.1) as i64 - 2 * apex as i64).abs());
            match best {
                Some((k, w)) => {
                    used[k] = true;
                    sc.area_err
                        .push(100.0 * (w.2 - true_area(p)) / true_area(p));
                    sc.edge_err.push(
                        ((w.0 as f64 - lo as f64).abs() + (w.1 as f64 - hi as f64).abs()) / 2.0
                            * DT
                            / p.s,
                    );
                }
                None => sc.missed += 1,
            }
        }
        sc.falses += used.iter().filter(|u| !**u).count();
    }
    sc
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

fn mean_abs(v: &[f64]) -> f64 {
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().map(|x| x.abs()).sum::<f64>() / v.len() as f64
    }
}

const METHODS: [(&str, Method); 8] = [
    ("A cwt", m_cwt),
    ("B page click", m_page),
    ("C prominence", m_prom),
    ("D prom+drop", m_prom_drop),
    ("E prom+asls", m_prom_asls),
    ("F0 bg, edge at 0", m_f0),
    ("F1 bg, edge at 1 sd", m_f1),
    ("F2 = F1 + slope stop", m_f2),
];

#[test]
fn benchmark_table() {
    let cs = cases();
    dump(&cs);
    let mut all: Vec<Vec<Score>> = Vec::new();
    println!("\n| case | method | mean abs area error % | bias % | mean edge error (sigma) | missed | false | us/trace |");
    println!("|---|---|---|---|---|---|---|---|");
    for c in &cs {
        let mut row = Vec::new();
        for (name, m) in METHODS {
            let s = score(m, c);
            println!(
                "| {} | {} | {:.1} | {:+.1} | {:.2} | {}/{} | {} | {:.0} |",
                c.name,
                name,
                mean_abs(&s.area_err),
                mean(&s.area_err),
                mean(&s.edge_err),
                s.missed,
                s.truths,
                s.falses,
                s.nanos as f64 / s.runs as f64 / 1000.0
            );
            row.push(s);
        }
        all.push(row);
    }
    println!("\n| method | mean abs area error % (all matched peaks) | missed | false |");
    println!("|---|---|---|---|");
    for (k, (name, _)) in METHODS.iter().enumerate() {
        let e: Vec<f64> = all.iter().flat_map(|r| r[k].area_err.clone()).collect();
        let (mi, fa, tr): (usize, usize, usize) = all.iter().fold((0, 0, 0), |a, r| {
            (a.0 + r[k].missed, a.1 + r[k].falses, a.2 + r[k].truths)
        });
        println!(
            "| {} | {:.1} | {}/{} | {} |",
            name,
            mean_abs(&e),
            mi,
            tr,
            fa
        );
    }
    check_f2(&cs, &all);
}

/// MZLAB_BENCH_DUMP=<file>: one trace per case with the edges of the Rust port of autoEdges, for the check against picchi.js.
fn dump(cs: &[Case]) {
    if let Ok(path) = std::env::var("MZLAB_BENCH_DUMP") {
        let rows: Vec<String> = cs
            .iter()
            .filter(|c| !c.peaks.is_empty())
            .map(|c| {
                let (rt, y, shapes) = draw(c, 1);
                let x = rt[true_limits(&shapes[0]).1];
                let (l, h) = auto_edges(&rt, &y, x);
                format!(
                    "{{\"x\":{x},\"y\":{:?},\"edges\":[{},{}]}}",
                    y, rt[l], rt[h]
                )
            })
            .collect();
        std::fs::write(path, format!("[{}]", rows.join(","))).unwrap();
    }
}

/// The chosen method (F2, the last one) keeps its accuracy: mean abs area error per case, no false peaks, few missed ones.
fn check_f2(cs: &[Case], all: &[Vec<Score>]) {
    let f2 = METHODS.len() - 1;
    for (c, r) in cs.iter().zip(all) {
        let s = &r[f2];
        let lim = match c.name {
            n if n.starts_with("shoulder") => 45.0,
            n if n.starts_with("low") => 10.0,
            _ => 5.0,
        };
        assert!(
            s.area_err.is_empty() || mean_abs(&s.area_err) < lim,
            "{}: {:.1} % over {lim}",
            c.name,
            mean_abs(&s.area_err)
        );
        assert_eq!(s.falses, 0, "{}: false peaks", c.name);
    }
    let e: Vec<f64> = all.iter().flat_map(|r| r[f2].area_err.clone()).collect();
    let missed: usize = all.iter().map(|r| r[f2].missed).sum();
    assert!(
        mean_abs(&e) < 6.0 && missed <= 35,
        "overall {:.1} %, {missed} missed",
        mean_abs(&e)
    );
}
