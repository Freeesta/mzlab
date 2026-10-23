//! `cargo run --release -p mzlab-lib --example identity_bench -- [N_SPECTRA] [unit|hr]` -> timings of `search::identity` on a deterministic synthetic
//! library (xorshift, no data files). Precursors are spread over 100-900, 5-40 peaks each from a shared fragment pool; `unit` = window 0.5 Da / fragments 0.3 Da, `hr` = 10 ppm / 0.01 Da.
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::search::{identity, IdentityOpts, Query, Tol};
use mzlab_lib::spec::{Meta, Spectrum};
use std::time::Instant;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn spectrum(r: &mut Rng, hr: bool) -> Spectrum {
    let prec = 100.0 + r.next() * 800.0;
    let prec = if hr {
        prec
    } else {
        prec.round() + (r.next() - 0.5) * 0.2
    };
    let n = 5 + (r.next() * 35.0) as usize;
    // fragments come from a pool of 400 recurring values (unit: integer m/z) with a little jitter, so that spectra share peaks like real ones
    let mut mz: Vec<f64> = (0..n)
        .map(|_| {
            let base = 50.0 + (r.next() * 400.0).floor() * 2.0;
            let m = if hr {
                base + 0.0012345 * base.sqrt()
            } else {
                base
            };
            (m + (r.next() - 0.5) * if hr { 0.004 } else { 0.2 }).min(prec - 1.0)
        })
        .collect();
    mz.sort_by(|a, b| a.total_cmp(b));
    mz.dedup_by(|a, b| (*a - *b).abs() < 0.02);
    let it = mz.iter().map(|_| 1.0 + r.next() * 999.0).collect();
    Spectrum {
        prec,
        pol: 1,
        mz,
        it,
        meta: Meta::default(),
        rt: None,
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: usize = a.get(1).map_or(100_000, |s| s.parse().expect("N"));
    let hr = a.get(2).is_some_and(|s| s == "hr");
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let set: Vec<Spectrum> = (0..n).map(|_| spectrum(&mut r, hr)).collect();
    let t = Instant::now();
    let (b, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    println!(
        "build {n} spectra: {:.2} s, {} MB",
        t.elapsed().as_secs_f64(),
        b.len() >> 20
    );
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    let (tol, frag) = if hr {
        (Tol::Ppm(10.0), 0.01)
    } else {
        (Tol::Da(0.5), 0.3)
    };
    // queries are library spectra with perturbed intensities, so every window holds a true neighbour
    let queries: Vec<Query> = set
        .iter()
        .step_by((n / 200).max(1))
        .filter_map(|s| {
            let it: Vec<f64> = s.it.iter().map(|v| v * (0.7 + 0.6 * r.next())).collect();
            Query::new(s.prec, 1, &s.mz, &it)
        })
        .collect();
    let o = IdentityOpts {
        prec_tol: tol,
        frag_da: frag,
        pol: 1,
        top: 10,
    };
    // pass 1 reads cold chunks (decompression included); pass 2 repeats the queries on the warm cache (scoring only)
    for pass in ["cold", "warm"] {
        let reads0 = lib.block_reads.get();
        let t = Instant::now();
        let (mut hits, mut check) = (0usize, 0.0f64);
        for q in &queries {
            let h = identity(&lib, q, &o).unwrap();
            hits += h.len();
            check += h.iter().map(|x| x.entropy).sum::<f64>();
        }
        let s = t.elapsed().as_secs_f64();
        println!(
            "{pass}: {} queries ({}): {:.3} s, {:.2} ms/query, {} chunk reads, {hits} hits, checksum {check:.6}",
            queries.len(),
            if hr { "hr" } else { "unit" },
            s,
            s * 1e3 / queries.len() as f64,
            lib.block_reads.get() - reads0
        );
    }
}
