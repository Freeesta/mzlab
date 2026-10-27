//! `cargo run --release -p mzlab-lib --example libset_bench -- [N_SPECTRA] [unit|hr] [LIBRARIES]` -> ms per query of the searches of a `LibSet` over a deterministic
//! synthetic library split into 1 or 3 files (same generator as `identity_bench`), and of the pair-collecting Flash search the accumulators replaced.
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::flash::FlashMode;
use mzlab_lib::frag_tol::FragTol;
use mzlab_lib::libset::LibSet;
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::search::{AnalogOpts, IdentityOpts, Query, Tol};
use mzlab_lib::spec::{Meta, Spectrum};
use std::time::Instant;

#[path = "../tests/common/flash_pairs.rs"]
mod flash_pairs;

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
        ri: None,
        ei: false,
    }
}

fn per_query(label: &str, n: usize, t: Instant, check: f64) {
    let s = t.elapsed().as_secs_f64();
    println!(
        "{label}: {:.3} ms/query ({n} queries, checksum {check:.4})",
        s * 1e3 / n as f64
    );
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: usize = a.get(1).map_or(100_000, |s| s.parse().expect("N"));
    let hr = a.get(2).is_some_and(|s| s == "hr");
    let libs: usize = a
        .get(3)
        .map_or(1, |s| s.parse().expect("LIBRARIES"))
        .clamp(1, 16);
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let set: Vec<Spectrum> = (0..n).map(|_| spectrum(&mut r, hr)).collect();
    let t = Instant::now();
    let per = n.div_ceil(libs);
    let built: Vec<Library<Vec<u8>>> = set
        .chunks(per)
        .map(|c| {
            Library::open(
                build_in_memory(c, BuildOpts::default()).unwrap().0,
                CACHE_BYTES,
            )
            .unwrap()
        })
        .collect();
    println!(
        "build {n} spectra in {} libraries: {:.2} s",
        built.len(),
        t.elapsed().as_secs_f64()
    );
    let ls = LibSet::new(built).unwrap();
    let (tol, frag) = if hr {
        (Tol::Ppm(10.0), 0.01)
    } else {
        (Tol::Da(0.5), 0.3)
    };
    let queries: Vec<Query> = set
        .iter()
        .step_by((n / 200).max(1))
        .filter_map(|s| {
            let it: Vec<f64> = s.it.iter().map(|v| v * (0.7 + 0.6 * r.next())).collect();
            Query::new(s.prec, 1, &s.mz, &it)
        })
        .collect();
    let open_q = &queries[..queries.len().min(50)];
    let ft = FragTol::from(frag);
    let io = IdentityOpts {
        prec_tol: tol,
        frag_da: frag,
        pol: 1,
        top: 10,
    };
    let ao = AnalogOpts {
        frag_da: frag,
        ..AnalogOpts::default()
    };
    bench_search(&ls, &queries, open_q, (&io, &ao), ft);
    bench_reference(&ls, open_q, frag);
}

/// The pair-vector search of the same queries, one library after the other, as the timing reference.
fn bench_reference(ls: &LibSet<Vec<u8>>, open_q: &[Query], frag: f64) {
    let t = Instant::now();
    let mut c = 0.0;
    for q in open_q {
        for k in 0..ls.len() {
            let lib = ls.lib(k).unwrap();
            let all = lib.ranges(1, f64::NEG_INFINITY, f64::INFINITY).unwrap();
            c += flash_pairs::reference(lib, q, FlashMode::Hybrid, frag, &all)
                .0
                .iter()
                .take(10)
                .map(|h| h.score)
                .sum::<f64>();
        }
    }
    per_query("open hybrid, pair vector (reference)", open_q.len(), t, c);
}

/// Times the identity, analog and open searches of the set.
fn bench_search(
    ls: &LibSet<Vec<u8>>,
    queries: &[Query],
    open_q: &[Query],
    (io, ao): (&IdentityOpts, &AnalogOpts),
    ft: FragTol,
) {
    // pass 1 reads cold chunks, pass 2 is warm
    for pass in ["cold", "warm"] {
        let t = Instant::now();
        let mut c = 0.0;
        for q in queries {
            c += ls
                .identity(q, io, ft, ls.all())
                .unwrap()
                .iter()
                .map(|h| h.hit.entropy)
                .sum::<f64>();
        }
        per_query(&format!("identity {pass}"), queries.len(), t, c);
    }
    let t = Instant::now();
    let mut c = 0.0;
    for q in open_q {
        c += ls
            .analogs(q, ao, ls.all())
            .unwrap()
            .iter()
            .map(|h| h.hit.mc.cosine)
            .sum::<f64>();
    }
    per_query(
        "analogs (hybrid +-200 Da, modified cosine)",
        open_q.len(),
        t,
        c,
    );
    for _ in 0..2 {
        let t = Instant::now();
        let mut c = 0.0;
        for q in open_q {
            c += ls
                .open(q, FlashMode::Hybrid, ft, 1, 10, ls.all())
                .unwrap()
                .iter()
                .map(|h| h.hit.score)
                .sum::<f64>();
        }
        per_query("open hybrid, flat accumulators", open_q.len(), t, c);
    }
}
