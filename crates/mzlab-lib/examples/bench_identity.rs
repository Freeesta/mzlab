//! `cargo run --release -p mzlab-lib --example bench_identity` -> time of `search::identity` on a synthetic library, by precursor window.
//! Synthetic data only (a deterministic LCG); used to compare the identity search before and after a change.
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::search::{identity, IdentityOpts, Query, Tol};
use mzlab_lib::spec::{Meta, Spectrum};

fn next(s: &mut u64) -> f64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*s >> 11) as f64 / (1u64 << 53) as f64
}

fn main() {
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000);
    let mut s = 42u64;
    let mut set = Vec::with_capacity(n);
    for k in 0..n {
        let prec = 150.0 + next(&mut s) * 450.0;
        let np = 10 + (next(&mut s) * 40.0) as usize;
        let mz: Vec<f64> = (0..np)
            .map(|_| 50.0 + next(&mut s) * (prec - 55.0))
            .collect();
        let it: Vec<f64> = (0..np).map(|_| 1.0 + next(&mut s) * 99.0).collect();
        set.push(Spectrum {
            prec,
            pol: 1,
            mz,
            it,
            meta: Meta {
                name: format!("s{k}"),
                ..Default::default()
            },
            rt: None,
        });
    }
    let (bytes, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    let queries: Vec<Query> = (0..200)
        .map(|k| {
            let sp = &set[(k * 487) % n];
            Query::new(sp.prec, 1, &sp.mz, &sp.it).unwrap()
        })
        .collect();
    // Decoding alone, over the same windows: what is left once the scoring is taken out.
    for (label, w) in [("0.5 Da", 0.5), ("5 Da", 5.0)] {
        let t = std::time::Instant::now();
        let mut peaks = 0usize;
        for q in &queries {
            for r in lib.ranges(1, q.prec - w, q.prec + w).unwrap() {
                for i in r {
                    peaks += lib.peaks(i).unwrap().0.len();
                }
            }
        }
        println!(
            "decode only {label}: {:.1} ms per query ({peaks} peaks)",
            t.elapsed().as_secs_f64() * 1e3 / queries.len() as f64
        );
    }
    for (label, tol) in [
        ("10 ppm", Tol::Ppm(10.0)),
        ("0.5 Da", Tol::Da(0.5)),
        ("5 Da", Tol::Da(5.0)),
        ("20 Da", Tol::Da(20.0)),
    ] {
        let o = IdentityOpts {
            prec_tol: tol,
            frag_da: 0.01,
            pol: 1,
            top: 5,
        };
        let t = std::time::Instant::now();
        let mut checksum = 0.0;
        for q in &queries {
            for h in identity(&lib, q, &o).unwrap() {
                checksum += h.entropy + h.index as f64 * 1e-9;
            }
        }
        println!(
            "{label}: {:.1} ms per query, checksum {checksum:.9}",
            t.elapsed().as_secs_f64() * 1e3 / queries.len() as f64
        );
    }
}
