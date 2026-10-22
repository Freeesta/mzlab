//! Speed bench of our Thermo `.raw` reader (criteria A5/A6 of docs/agenti/thermo-velocita.md): open, TIC/BPC, first scan, all scans, 100 random scans.
//! Writes one `banco/1` JSON line per file; read-only, uses the same functions as the `thermo` module. A tool of the bench: it never enters a .wasm.
//!   thermo_banco FILE.raw [--sha MAIN_SHA_40] [--reps 5] [--machine TEXT] [--profile TEXT] [--target native|wasm]
use std::time::Instant;

use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::rawlettore;
use serde_json::json;

fn fileset(name: &str, bytes: Vec<u8>) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: name.to_string(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }],
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn quantile(v: &mut [f64], q: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * q).round() as usize]
}

/// Times of one lap, in ms.
struct Lap {
    open: f64,
    tic_bpc: f64,
    first_scan: f64,
    read_all: f64,
    peaks: u64,
    random: Vec<f64>,
}

fn lap(name: &str, bytes: &[u8]) -> (Lap, usize) {
    let t0 = Instant::now();
    let mut l =
        (rawlettore::VOCE.apri)(fileset(name, bytes.to_vec()), &Limiti::desktop()).expect("open");
    let n = l.indice(0).expect("index").len();
    let open = ms(t0);

    // TIC and base peak come from the index and the headers, no peaks decoded
    let t = Instant::now();
    let idx = l.indice(0).expect("index");
    let (mut tic, mut bpc) = (0.0f64, 0.0f64);
    for v in idx {
        tic += v.tic;
        bpc += v.picco_base.unwrap_or(0.0);
    }
    std::hint::black_box((tic, bpc));
    let tic_bpc = ms(t);

    let t = Instant::now();
    std::hint::black_box(l.scansione(0, 0).expect("scan"));
    let first_scan = ms(t);

    let t = Instant::now();
    let mut peaks = 0u64;
    for k in 0..n {
        peaks += l.scansione(0, k).expect("scan").mz.len() as u64;
    }
    let read_all = ms(t);

    // 100 scans at fixed pseudo-random positions (same on every run and machine)
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut random = Vec::with_capacity(100);
    for _ in 0..100 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let k = (x % n as u64) as usize;
        let t = Instant::now();
        std::hint::black_box(l.scansione(0, k).expect("scan"));
        random.push(ms(t));
    }
    (
        Lap {
            open,
            tic_bpc,
            first_scan,
            read_all,
            peaks,
            random,
        },
        n,
    )
}

fn main() {
    std::panic::set_hook(Box::new(|i| {
        eprintln!("thermo_banco: {i}");
        std::process::exit(2);
    }));
    let a: Vec<String> = std::env::args().skip(1).collect();
    let opt = |f: &str| a.iter().position(|x| x == f).and_then(|i| a.get(i + 1));
    let path = a.first().expect("FILE.raw");
    let reps: usize = opt("--reps").map_or(5, |s| s.parse().expect("--reps"));
    let name = std::path::Path::new(path)
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());

    let t = Instant::now();
    let bytes = std::fs::read(path).expect("read");
    let file_read = ms(t);
    let mb = bytes.len() as f64 / 1e6;

    let mut laps = Vec::new();
    let mut n_scans = 0;
    for _ in 0..reps {
        let (l, n) = lap(&name, &bytes);
        n_scans = n;
        laps.push(l);
    }
    let med = |f: &dyn Fn(&Lap) -> f64| {
        let mut v: Vec<f64> = laps.iter().map(f).collect();
        quantile(&mut v, 0.5)
    };
    let mut random: Vec<f64> = laps.iter().flat_map(|l| l.random.iter().copied()).collect();
    let (r50, r95) = (
        quantile(&mut random.clone(), 0.5),
        quantile(&mut random, 0.95),
    );
    let read_all = med(&|l| l.read_all);
    let open = med(&|l| l.open);
    let first = &laps[0];
    println!(
        "{}",
        json!({
            "schema": "banco/1",
            "reader": "mzlab",
            "main_sha": opt("--sha"),
            "target": opt("--target").map_or("native", |s| s.as_str()),
            "machine": opt("--machine"),
            "profile": opt("--profile"),
            "file": name,
            "bytes": bytes.len(),
            "n_scans": n_scans,
            "n_peaks": first.peaks,
            "reps": reps,
            "file_read_ms": file_read,
            // first lap, with the cache as the run left it; the medians below are the warm laps
            "first_lap": {"open_ms": first.open, "tic_bpc_ms": first.tic_bpc, "first_scan_ms": first.first_scan, "read_all_ms": first.read_all},
            "median": {
                "open_ms": open,
                "tic_bpc_ms": med(&|l| l.tic_bpc),
                "first_scan_ms": med(&|l| l.first_scan),
                "read_all_ms": read_all,
                "random_scan_ms_median": r50,
                "random_scan_ms_p95": r95,
            },
            "open_mb_s": mb / (open / 1e3),
            "read_all_mb_s": mb / (read_all / 1e3),
            "read_all_scans_s": n_scans as f64 / (read_all / 1e3),
        })
    );
}
