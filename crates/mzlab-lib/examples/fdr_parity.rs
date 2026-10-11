//! `cargo run --release -p mzlab-lib --example fdr_parity -- OUTDIR`
//! Writes a synthetic library with decoys as MSP (`lib.msp`), a batch of queries (`queries.json`) and the native answer of `search_fdr_json`
//! (`native.json`); `tests/test_librerie_fdr_wasm.py` gives the same MSP and queries to the `.wasm` and compares the bytes.
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::decoy::*;
use mzlab_lib::parse::{Format, Parser};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::search::{IdentityOpts, Query, Tol};
use mzlab_lib::spec::{Meta, Spectrum};
use std::cell::RefCell;
use std::fmt::Write;
use std::rc::Rc;

fn next(s: &mut u64) -> f64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*s >> 33) % 1_000_000) as f64 / 1_000_000.0
}

fn make_targets(r: &mut u64) -> Vec<Spectrum> {
    let mut targets: Vec<Spectrum> = Vec::new();
    for i in 0..300 {
        let prec = ((200.0 + next(r) * 600.0) * 1e4).round() / 1e4;
        let mut mz = Vec::new();
        let mut m = 60.0 + next(r) * 10.0;
        while mz.len() < 10 && m < prec - 5.0 {
            mz.push((m * 1e4).round() / 1e4);
            m += 5.0 + next(r) * 40.0;
        }
        let it: Vec<f64> = mz
            .iter()
            .map(|_| 1.0 + (next(r) * 1000.0).round())
            .collect();
        let meta = Meta {
            accession: format!("SYN{i:04}"),
            name: format!("Synthetic {i}"),
            ..Default::default()
        };
        targets.push(Spectrum {
            prec,
            pol: 1,
            mz,
            it,
            meta,
        });
    }
    targets
}

fn to_msp(spectra: &[Spectrum]) -> String {
    let mut msp = String::new();
    for s in spectra {
        let _ = write!(
            msp,
            "Name: {}\nAccession: {}\nPrecursorMZ: {}\nIon_mode: Positive\nNum Peaks: {}\n",
            s.meta.name,
            s.meta.accession,
            s.prec,
            s.mz.len()
        );
        for (m, i) in s.mz.iter().zip(&s.it) {
            let _ = writeln!(msp, "{m} {i}");
        }
        msp.push('\n');
    }
    msp
}

/// Queries concatenated like the arguments of `search_with_fdr`: noisy copies of every 3rd target, then 40 wrong spectra at the precursor of a target.
fn make_queries(targets: &[Spectrum], r: &mut u64) -> (Vec<f64>, Vec<u32>, Vec<f64>, Vec<f64>) {
    let (mut precs, mut starts, mut qmz, mut qit) =
        (Vec::new(), vec![0u32], Vec::new(), Vec::new());
    for k in 0..140 {
        let s = &targets[(k * 3) % 300];
        let src = if k >= 100 {
            &targets[(k * 7 + 1) % 300]
        } else {
            s
        };
        for (m, i) in src.mz.iter().zip(&src.it) {
            if m < &(s.prec - 4.0) {
                qmz.push(((m + (next(r) - 0.5) * 0.004) * 1e4).round() / 1e4);
                qit.push((i * (0.7 + next(r) * 0.6) * 100.0).round() / 100.0);
            }
        }
        precs.push(s.prec);
        starts.push(qmz.len() as u32);
    }
    (precs, starts, qmz, qit)
}

/// The native answer, from the MSP text the way the importer builds it.
fn native_answer(msp: &str, queries: &(Vec<f64>, Vec<u32>, Vec<f64>, Vec<f64>)) -> String {
    let (precs, starts, qmz, qit) = queries;
    let parsed: Rc<RefCell<Vec<Spectrum>>> = Rc::new(RefCell::new(Vec::new()));
    let p2 = parsed.clone();
    let mut p = Parser::new(Format::Msp, move |s: Spectrum| p2.borrow_mut().push(s));
    p.push(msp.as_bytes());
    p.end();
    let (bytes, _) = build_in_memory(&parsed.borrow(), BuildOpts::default()).unwrap();
    let lib = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    let qs: Vec<Option<Query>> = (0..precs.len())
        .map(|k| {
            let (a, b) = (starts[k] as usize, starts[k + 1] as usize);
            Query::new(precs[k], 1, &qmz[a..b], &qit[a..b])
        })
        .collect();
    let o = IdentityOpts {
        prec_tol: Tol::Ppm(10.0),
        frag_da: 0.01,
        pol: 1,
        top: 20,
    };
    search_fdr_json(&search_fdr(&lib, &qs, &o, DecoyKind::Fragments).unwrap())
}

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("OUTDIR"));
    std::fs::create_dir_all(&dir).unwrap();
    let mut r = 42u64;
    let targets = make_targets(&mut r);
    let msp = to_msp(&with_decoys(&targets, DecoyKind::Fragments, DEFAULT_SEED));
    std::fs::write(dir.join("lib.msp"), &msp).unwrap();
    let queries = make_queries(&targets, &mut r);
    let (precs, starts, qmz, qit) = &queries;
    let q = serde_json::json!({"precs": precs, "pols": vec![1; precs.len()], "starts": starts, "mz": qmz, "it": qit});
    std::fs::write(dir.join("queries.json"), q.to_string()).unwrap();
    std::fs::write(dir.join("native.json"), native_answer(&msp, &queries)).unwrap();
    println!(
        "{} targets + decoys, {} queries",
        targets.len(),
        precs.len()
    );
}
