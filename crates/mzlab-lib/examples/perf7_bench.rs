//! `cargo run --release -p mzlab-lib --example perf7_bench -- [N_SPECTRA] [N_QUERIES]` -> timings of the library hot paths on a deterministic synthetic library
//! (xorshift, no data files): `Builder::add` / `finish`, open + `light()`, Flash searches (open, neutral loss, hybrid), MGF parsing and `lib_fdr`.
//! Prints the SHA-256 of the built library and a checksum of the scores, so two builds of the code can be compared.
use mzlab_lib::build::{BuildOpts, Builder};
use mzlab_lib::decoy::{lib_fdr, DecoyKind};
use mzlab_lib::flash::{flash_top, FlashAcc, FlashMode};
use mzlab_lib::parse::{parse_all, Format};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::search::Query;
use mzlab_lib::sha256::sha256;
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

fn spectrum(r: &mut Rng, i: usize) -> Spectrum {
    let prec = 100.0 + r.next() * 800.0;
    let n = 5 + (r.next() * 35.0) as usize;
    let mut mz: Vec<f64> = (0..n)
        .map(|_| {
            (50.0 + (r.next() * 400.0).floor() * 2.0 + (r.next() - 0.5) * 0.004).min(prec - 1.0)
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
        meta: Meta {
            name: format!("Compound {i}"),
            adduct: "[M+H]+".into(),
            inchikey: format!("{:014X}-ABCDEFGHIJ-N", i % 30_000),
            accession: format!("ACC{i:07}"),
            ..Meta::default()
        },
        rt: None,
        ri: None,
        ei: false,
    }
}

fn mgf_of(set: &[Spectrum]) -> String {
    let mut s = String::new();
    for sp in set {
        s.push_str(&format!(
            "BEGIN IONS\nTITLE={}\nPEPMASS={:.4}\nCHARGE=1+\nRTINSECONDS=12,5\n",
            sp.meta.name, sp.prec
        ));
        for (m, i) in sp.mz.iter().zip(&sp.it) {
            s.push_str(&format!("{m:.4} {i:.2}\n"));
        }
        s.push_str("END IONS\n\n");
    }
    s
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: usize = a.get(1).map_or(100_000, |s| s.parse().expect("N"));
    let nq: usize = a.get(2).map_or(1000, |s| s.parse().expect("queries"));
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let set: Vec<Spectrum> = (0..n).map(|i| spectrum(&mut r, i)).collect();

    let t = Instant::now();
    let mut b = Builder::new(Vec::<u8>::new(), BuildOpts::default());
    for s in &set {
        b.add(s).unwrap();
    }
    let t_add = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let mut out = Vec::new();
    b.finish(&mut out).unwrap();
    let t_fin = t.elapsed().as_secs_f64();
    let sha: String = sha256(&[&out[..]])
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect();
    println!(
        "build {n}: add {t_add:.3} s, finish {t_fin:.3} s, {} MB, sha256 {sha}",
        out.len() >> 20
    );

    let reps: usize = std::env::var("BENCH_REPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let mut best = f64::MAX;
    for _ in 0..reps {
        let l = Library::open(&out[..], CACHE_BYTES).unwrap();
        let t = Instant::now();
        l.light().unwrap();
        best = best.min(t.elapsed().as_secs_f64());
    }
    println!("light() best of {reps}: {:.1} ms", best * 1e3);
    let lib = Library::open(&out[..], CACHE_BYTES).unwrap();
    lib.light().unwrap();

    let queries: Vec<Query> = set
        .iter()
        .step_by((n / nq).max(1))
        .take(nq)
        .filter_map(|s| {
            let it: Vec<f64> = s.it.iter().map(|v| v * (0.7 + 0.6 * r.next())).collect();
            Query::new(s.prec + 14.0, 1, &s.mz, &it)
        })
        .collect();
    let ranges = lib.ranges(1, 0.0, 2000.0).unwrap();
    for (name, mode) in [
        ("open", FlashMode::Open),
        ("nloss", FlashMode::NeutralLoss),
        ("hybrid", FlashMode::Hybrid),
    ] {
        let mut acc = FlashAcc::new();
        let mut best = f64::MAX;
        let (mut cnt, mut check, mut h) = (0usize, 0.0f64, 0xcbf2_9ce4_8422_2325u64);
        for rep in 0..reps {
            let t = Instant::now();
            cnt = 0;
            check = 0.0;
            h = 0xcbf2_9ce4_8422_2325u64;
            for q in &queries {
                let v =
                    flash_top(&lib, q, mode, 0.01, &ranges, &mut acc, Some(10), |_| true).unwrap();
                cnt += v.len();
                check += v
                    .iter()
                    .map(|x| x.score + x.pairs as f64 + x.index as f64)
                    .sum::<f64>();
                for x in &v {
                    for w in [x.score.to_bits(), x.index as u64, x.pairs as u64] {
                        h = (h ^ w).wrapping_mul(0x0100_0000_01b3);
                    }
                }
            }
            if rep > 0 || reps == 1 {
                best = best.min(t.elapsed().as_secs_f64());
            }
        }
        println!(
            "{name}: {} queries, best of {reps}: {best:.3} s = {:.2} ms/query, {cnt} hits, checksum {check:.9}, bits {h:016x}",
            queries.len(),
            best * 1e3 / queries.len() as f64
        );
    }

    let text = mgf_of(&set[..n.min(50_000)]);
    let mut best = f64::MAX;
    let mut nrec = 0;
    for _ in 0..reps {
        let t = Instant::now();
        let (v, _) = parse_all(Format::Mgf, text.as_bytes());
        best = best.min(t.elapsed().as_secs_f64());
        nrec = v.len();
    }
    println!(
        "parse MGF {} MB: best {best:.3} s, {nrec} records",
        text.len() >> 20
    );

    let mut ts = Vec::new();
    let mut ds = Vec::new();
    for _ in 0..200_000 {
        ts.push(((r.next() * 400.0).floor() / 400.0) as f32);
        ds.push(((r.next() * r.next() * 400.0).floor() / 400.0) as f32);
    }
    let mut best = f64::MAX;
    let mut chk = 0.0;
    for _ in 0..reps {
        let t = Instant::now();
        let f = lib_fdr(&ts, &ds, DecoyKind::Precursor);
        best = best.min(t.elapsed().as_secs_f64());
        chk = f
            .iter()
            .map(|x| x.q_value as f64 + x.n_decoy_ge as f64)
            .sum::<f64>();
    }
    println!(
        "lib_fdr 200k/200k: best {:.1} ms, checksum {chk:.6}",
        best * 1e3
    );
}
