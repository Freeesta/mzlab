//! Study C32 (docs/studi/C32_reattivita_precalcolata.md): runs `reactivity_json` with the app defaults over every SMILES of
//! `composti.csv`, prints timings and writes a compact per-compound table. Not part of the wasm build nor of the tests.
//! Usage: cargo run --release -p mzlab-chem --example precalc_reatt -- <composti.csv> <out.json> [step] [threads]

use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&a[1]).expect("csv");
    let step: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let threads: usize = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(1);
    // Names may contain commas: the last four fields are fixed.
    let rows: Vec<(String, String)> = text
        .lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.rsplitn(5, ',').collect();
            (f.len() == 5).then(|| (f[0].to_string(), f[1].to_string()))
        })
        .enumerate()
        .filter(|(i, _)| i % step == 0)
        .map(|(_, r)| r)
        .collect();
    let t0 = Instant::now();
    let chunk = rows.len().div_ceil(threads);
    let outs: Vec<Vec<(String, f64, Result<String, String>)>> = std::thread::scope(|s| {
        let hs: Vec<_> = rows
            .chunks(chunk)
            .map(|c| {
                s.spawn(move || {
                    c.iter()
                        .map(|(ik, smi)| {
                            let t = Instant::now();
                            let r = mzlab_chem::reactivity_json(smi, "").map_err(|e| format!("{e:?}"));
                            (ik.clone(), t.elapsed().as_secs_f64(), r)
                        })
                        .collect()
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let total = t0.elapsed().as_secs_f64();
    let all: Vec<_> = outs.into_iter().flatten().collect();
    for (ik, t, r) in &all {
        if let Err(e) = r {
            println!("FAIL {ik} {t:.1}s {e}");
        }
    }
    let mut times: Vec<f64> = all.iter().map(|x| x.1).collect();
    times.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let ok = all.iter().filter(|x| x.2.is_ok()).count();
    let full: usize = all.iter().filter_map(|x| x.2.as_ref().ok()).map(|s| s.len()).sum();
    let n = times.len();
    println!(
        "n={n} ok={ok} fail={} wall={total:.1}s sum={:.1}s median={:.3}s p90={:.3}s max={:.3}s full_json_bytes={full}",
        n - ok,
        times.iter().sum::<f64>(),
        times[n / 2],
        times[n * 9 / 10],
        times[n - 1]
    );
    // Compact table: {inchikey: [[q, f+, f-, homo, lumo] per atom (3 decimals; H included)]} + per-compound HOMO/LUMO/gap.
    let mut o = String::from("{\"engine\":\"");
    o += mzlab_chem::ENGINE;
    o += "\",\"rows\":{";
    let mut first = true;
    for (ik, _, r) in &all {
        let Ok(s) = r else { continue };
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        let g = |a: &serde_json::Value, k: &str| a.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
        let atoms: Vec<String> = v["atoms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| {
                format!(
                    "[{:.3},{:.3},{:.3},{:.3},{:.3}]",
                    g(a, "q"),
                    g(a, "f_plus"),
                    g(a, "f_minus"),
                    g(a, "homo"),
                    g(a, "lumo")
                )
            })
            .collect();
        if !first {
            o.push(',');
        }
        first = false;
        o += &format!(
            "\"{ik}\":{{\"homo\":{:.3},\"lumo\":{:.3},\"a\":[{}]}}",
            g(&v["global"], "homo_ev"),
            g(&v["global"], "lumo_ev"),
            atoms.join(",")
        );
    }
    o += "}}";
    std::fs::write(&a[2], &o).unwrap();
    println!("compact_bytes={}", o.len());
}
