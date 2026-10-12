//! Targeted pass on a series of `.mzidx` files: hypotheses (parent x rules, plus decoys and real TPs of a synthetic truth), time, and the numbers.
//!
//!     cargo run --release -p mzlab-feat --example mirato -- <file.mzidx>... --truth verita.json [--esche] [--oracle] [--neg] [--rules FILE]
//!
//! `--extra N` adds N invented masses (timing of thousands of hypotheses). The parent comes from the truth (`parent.ion` minus H, `parent.rt_obs`). `--esche` adds the decoy ions of the truth (invented rules, absent from the
//! data) as imported masses; `--oracle` adds the true products as imported masses (the sensitivity of the extraction alone). Prints how many
//! hypotheses were tried and found, the products found by the detection and by the targeted pass (weak = peak under 10 times the noise floor), the
//! decoys that give a targeted row (any, and with an MS2 linked: the only way over level 4) and the time of two runs with their SHA-256.

use std::fs::File;
use std::time::Instant;

use mzlab_core::elements::{fmt, parse_formula};
use mzlab_core::idxread::Index;
use mzlab_feat::hypo::{generate, HypoParams, Imported, Inputs, Registry, Rules};
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::table::{FileFeatures, FLAG_FT_LOBE, FLAG_GOOD};
use mzlab_feat::target::{run, TargetParams, TargetResult};
use mzlab_feat::trace::Params;
use serde_json::Value;
use sha2::{Digest, Sha256};

const PROTON: f64 = 1.007_276;

struct Ion {
    id: String,
    role: String,
    mz: f64,
    rt: f64,
}

fn truth(path: &str) -> (Vec<Ion>, String, f64) {
    let v: Value = serde_json::from_slice(&std::fs::read(path).expect("truth")).expect("json");
    let ions = v["ioni"]
        .as_array()
        .expect("ioni")
        .iter()
        .filter(|x| x["presente"].as_i64().unwrap_or(1) == 1 || x["ruolo"] == "esca");
    let ions = ions
        .map(|x| Ion {
            id: x["id"].as_str().unwrap_or("").to_string(),
            role: x["ruolo"].as_str().unwrap_or("").to_string(),
            mz: x["mz"].as_f64().unwrap_or(0.0),
            rt: x["rt_obs"]
                .get(0)
                .and_then(Value::as_f64)
                .unwrap_or(f64::NAN),
        })
        .collect();
    let mut f = parse_formula(v["parent"]["ion"].as_str().expect("parent.ion")).expect("formula");
    for e in f.iter_mut().filter(|e| e.0 == "H") {
        e.1 -= 1;
    }
    (
        ions,
        fmt(&f),
        v["parent"]["rt_obs"].as_f64().unwrap_or(f64::NAN),
    )
}

fn load(path: &str, pol: i8) -> (Index<File>, FileFeatures) {
    let f = File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    let mut ix = Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"));
    let p = Params {
        polarity: Some(pol),
        ..Default::default()
    };
    let (feat, _) =
        analyse(&mut ix, &p, &Steps::default()).unwrap_or_else(|e| panic!("{path}: {e}"));
    (ix, feat)
}

fn load_all(files: &[&String], pol: i8) -> (Vec<Index<File>>, Vec<FileFeatures>) {
    let (mut ix, mut feats) = (Vec::new(), Vec::new());
    for p in files {
        let (i, f) = load(p, pol);
        ix.push(i);
        feats.push(f);
    }
    (ix, feats)
}

fn imported(ions: &[Ion], role: &[&str], tag: &str) -> Vec<Imported> {
    let it = ions.iter().filter(|i| role.contains(&i.role.as_str()));
    it.map(|i| Imported {
        name: format!("{tag}:{}", i.id),
        formula: String::new(),
        mass: Some(i.mz - PROTON),
    })
    .collect()
}

fn near(row_mz: f64, row_rt: f64, i: &Ion) -> bool {
    (row_mz - i.mz).abs() <= i.mz * 5e-6 && (i.rt.is_nan() || (row_rt - i.rt).abs() <= 0.15)
}

fn detected_good(feats: &[FileFeatures], i: &Ion) -> bool {
    feats.iter().any(|f| {
        (0..f.len()).any(|k| {
            f.flags[k] & FLAG_GOOD != 0
                && f.flags[k] & FLAG_FT_LOBE == 0
                && near(f.mz[k], f.rt[k] as f64, i)
        })
    })
}

fn hit_rows(res: &TargetResult, i: &Ion) -> Vec<usize> {
    (0..res.rows.len())
        .filter(|&r| near(res.rows[r].mz, res.rows[r].rt, i))
        .collect()
}

fn pct(a: usize, b: usize) -> f64 {
    100.0 * a as f64 / b.max(1) as f64
}

fn report_products(
    ions: &[Ion],
    feats: &[FileFeatures],
    res: &TargetResult,
    ix: &mut [Index<File>],
) {
    let tps: Vec<&Ion> = ions
        .iter()
        .filter(|i| ["tp1", "tp2", "isomero"].contains(&i.role.as_str()))
        .collect();
    let (mut base, mut targ, mut weak, mut weak_b, mut weak_t) = (0, 0, 0, 0, 0);
    for i in &tps {
        let height = max_height(ix, i);
        let (b, t) = (detected_good(feats, i), !hit_rows(res, i).is_empty());
        base += b as usize;
        targ += (t || b) as usize;
        if height < 2e5 {
            weak += 1;
            weak_b += b as usize;
            weak_t += (t || b) as usize;
        }
    }
    println!("products (tp1, tp2, isomer): {}; detection {base}; detection + targeted pass {targ} (+{:.1} % of the products)", tps.len(), pct(targ.saturating_sub(base), tps.len()));
    println!("weak products (peak < 2e5): {weak}; detection {weak_b} ({:.1} %), targeted pass {weak_t} ({:.1} %)", pct(weak_b, weak), pct(weak_t, weak));
}

fn max_height(ix: &mut [Index<File>], i: &Ion) -> f64 {
    let mut best = 0.0f64;
    for x in ix.iter_mut() {
        let Some(si) = x.section_of(1, Some(1)) else {
            continue;
        };
        let rt = x.arrays(si).expect("arrays").rt.clone();
        let d = i.mz * 5e-6;
        let (a, b) = (
            x.search(si, i.mz - d, false).unwrap(),
            x.search(si, i.mz + d, true).unwrap(),
        );
        let y = x.sum_window(si, a, b).unwrap();
        for (k, v) in y.iter().enumerate() {
            if (rt[k] - i.rt).abs() <= 0.1 {
                best = best.max(*v);
            }
        }
    }
    best
}

fn report_decoys(ions: &[Ion], res: &TargetResult, reg: &Registry) {
    let esche: Vec<&Ion> = ions.iter().filter(|i| i.role == "esca").collect();
    let mut rows = Vec::new();
    for e in &esche {
        for r in hit_rows(res, e) {
            if reg.hyps[res.rows[r].hyp as usize]
                .label
                .starts_with("esche:")
            {
                rows.push(r);
            }
        }
    }
    let with_ms2 = rows
        .iter()
        .filter(|&&r| res.rows[r].n_targeted > 0 && res.rows[r].ms2_total > 0)
        .count();
    println!("decoys (invented rules): {}; with a targeted row {} ({:.1} %); with a targeted row and MS2 {with_ms2} ({:.1} %)", esche.len(), rows.len(), pct(rows.len(), esche.len()), pct(with_ms2, esche.len()));
    let known = |row: &mzlab_feat::target::TargetRow| {
        ions.iter()
            .any(|i| i.role != "esca" && near(row.mz, row.rt, i))
    };
    let false_rows: Vec<_> = res
        .rows
        .iter()
        .filter(|r| reg.hyps[r.hyp as usize].sources & 1 != 0 && !known(r))
        .collect();
    let n_rule = reg.hyps.iter().filter(|h| h.sources & 1 != 0).count();
    let ms2 = false_rows.iter().filter(|r| r.ms2_total > 0).count();
    println!("hypotheses of the rules: {n_rule}; rows that match no ion of the truth: {} ({:.1} %), with MS2: {ms2}", false_rows.len(), pct(false_rows.len(), n_rule));
}

fn report_cap(res: &TargetResult) {
    let capped = res.rows.iter().filter(|r| r.level_cap().is_some()).count();
    let mut p: Vec<f64> = res
        .rows
        .iter()
        .filter(|r| r.n_targeted > 0)
        .map(|r| r.coincidence)
        .collect();
    p.sort_by(|a, b| a.total_cmp(b));
    println!("rows capped at level 4: {capped} of {}; chance of coincidence (rows with a targeted cell) median {:.3}", res.rows.len(), p.get(p.len() / 2).copied().unwrap_or(f64::NAN));
}

fn build_imports(ions: &[Ion], esche: bool, oracle: bool, extra: Option<String>) -> Vec<Imported> {
    let mut imp = Vec::new();
    if esche {
        imp.extend(imported(ions, &["esca"], "esche"));
    }
    if oracle {
        imp.extend(imported(ions, &["tp1", "tp2", "isomero"], "orac"));
    }
    if let Some(n) = extra.and_then(|v| v.parse::<usize>().ok()) {
        // n invented neutral masses, evenly spread and off the mass-defect ridge, to time the pass on thousands of hypotheses
        imp.extend((0..n).map(|k| Imported {
            name: format!("extra:{k}"),
            formula: String::new(),
            mass: Some(150.0 + 650.0 * (k as f64 + 0.37) / n as f64),
        }));
    }
    imp
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let val = |f: &str| {
        args.iter()
            .position(|a| a == f)
            .and_then(|k| args.get(k + 1))
            .cloned()
    };
    let pol: i8 = if flag("--neg") { -1 } else { 1 };
    let files: Vec<&String> = args.iter().filter(|a| a.ends_with(".mzidx")).collect();
    let (ions, parent, parent_rt) = truth(&val("--truth").expect("--truth verita.json"));
    let rules_path = val("--rules").unwrap_or_else(|| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../TP_Mine/data/regole_tp.json")
            .display()
            .to_string()
    });
    let rules = Rules::parse(&std::fs::read(rules_path).expect("rules")).expect("rules");
    let imp = build_imports(&ions, flag("--esche"), flag("--oracle"), val("--extra"));
    let t = Instant::now();
    let (mut ix, feats) = load_all(&files, pol);
    println!(
        "{} files read and detected in {:.1} s; parent {parent} at {parent_rt} min",
        files.len(),
        t.elapsed().as_secs_f64()
    );
    let hp = HypoParams {
        polarity: pol,
        ..Default::default()
    };
    let inp = Inputs {
        parent: &parent,
        suspects: None,
        families: &[],
        imported: &imp,
        fragments: &[],
    };
    let mut reg = generate(&rules, &inp, &hp).expect("hypotheses");
    let tp = TargetParams {
        polarity: pol,
        parent_rt: parent_rt.is_finite().then_some(parent_rt),
        ..Default::default()
    };
    let refs: Vec<&FileFeatures> = feats.iter().collect();
    let mut digests = Vec::new();
    let mut res = None;
    for k in 0..2 {
        let mut r2 = reg.clone();
        let t = Instant::now();
        let out = run(&mut ix, &refs, &mut r2, &tp).expect("target");
        println!(
            "run {}: {} hypotheses in {:.2} s, {} found",
            k + 1,
            out.tried,
            t.elapsed().as_secs_f64(),
            out.found
        );
        digests.push(Sha256::digest(
            [out.to_bytes(), r2.to_json().into_bytes()].concat(),
        ));
        reg = r2;
        res = Some(out);
    }
    println!("SHA-256 of two runs equal: {}", digests[0] == digests[1]);
    let res = res.expect("run");
    println!(
        "by source: {:?}; found by hypothesis: {} of {}",
        reg.by_source(),
        reg.found(),
        reg.len()
    );
    report_products(&ions, &feats, &res, &mut ix);
    report_decoys(&ions, &res, &reg);
    report_cap(&res);
}
