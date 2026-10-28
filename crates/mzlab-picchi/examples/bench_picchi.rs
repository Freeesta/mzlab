// Timing and fingerprints of the peak picker pieces: AsLS on 10 000 points (downsampled to 160) and on 2 000 (solved at full size), detect_full on 10 000 points,
// Savitzky-Golay and Ricker CWT, noise_level, any_by on 10^6 items. The fingerprints must not change between versions.
// Run with `cargo run --release -p mzlab-picchi --example bench_picchi`.

use std::time::Instant;

use mzlab_picchi::peaks::{detect_full, Params};
use mzlab_picchi::signal::{asls_baseline, cwt_ricker, noise_level, savgol_d};
use mzlab_picchi::sortc::any_by;

fn fp(v: &[f64]) -> u64 {
    v.iter().fold(0xcbf29ce484222325u64, |h, x| {
        (h ^ x.to_bits()).wrapping_mul(0x100000001b3)
    })
}

fn time<T>(name: &str, reps: usize, mut f: impl FnMut() -> T, fingerprint: impl Fn(&T) -> u64) {
    let mut last = f();
    let t = Instant::now();
    for _ in 0..reps {
        last = f();
    }
    println!(
        "{name:<34} {:>9.3} ms  {:016x}",
        t.elapsed().as_secs_f64() * 1e3 / reps as f64,
        fingerprint(&last)
    );
}

fn trace(n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut s = 12345u64;
    let mut r = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 33) as f64 / (1u64 << 31) as f64
    };
    let centres: Vec<(f64, f64, f64)> = (0..12)
        .map(|_| (r() * n as f64, 1e3 + r() * 1e5, 4.0 + r() * 20.0))
        .collect();
    let rt: Vec<f64> = (0..n).map(|i| i as f64 * 0.003).collect();
    let y = (0..n)
        .map(|i| {
            let g: f64 = centres
                .iter()
                .map(|&(c, h, w)| h * (-((i as f64 - c) / w).powi(2) / 2.0).exp())
                .sum();
            200.0 + 0.05 * i as f64 + 80.0 * r() + g
        })
        .collect();
    (rt, y)
}

fn main() {
    let (rt, y) = trace(10_000);
    let (_, y2) = trace(2_000);
    time(
        "asls 10000 pts (max_n 160)",
        20,
        || asls_baseline(&y, 1e5, 0.01, 10, 160),
        |v| fp(v),
    );
    time(
        "asls 2000 pts (full size)",
        3,
        || asls_baseline(&y2, 1e5, 0.01, 10, usize::MAX / 4),
        |v| fp(v),
    );
    time(
        "asls 160 pts",
        200,
        || asls_baseline(&y[..160], 1e5, 0.01, 10, 160),
        |v| fp(v),
    );
    time(
        "detect_full 10000 pts",
        20,
        || detect_full(&rt, &y, Params::default()),
        |d| {
            fp(&d.yc)
                ^ fp(&d.ys)
                ^ d.noise.to_bits()
                ^ d.peaks.iter().fold(0u64, |h, p| {
                    h.rotate_left(5)
                        ^ p.area.to_bits()
                        ^ p.height.to_bits()
                        ^ p.lo as u64
                        ^ (p.hi as u64) << 20
                })
        },
    );
    time(
        "savgol_d 10000 pts w5",
        200,
        || savgol_d(&y, 5, 2, 0),
        |v| fp(v),
    );
    time(
        "savgol_d 10000 pts w21 d1",
        100,
        || savgol_d(&y, 21, 3, 1),
        |v| fp(v),
    );
    let widths = [1.0, 1.7, 2.9, 4.9, 8.3, 14.0];
    time(
        "cwt_ricker 10000 pts",
        10,
        || cwt_ricker(&y, &widths),
        |c| c.iter().fold(0, |h, v| h ^ fp(v)),
    );
    time(
        "noise_level 10000 pts",
        200,
        || noise_level(&y),
        |v| v.to_bits(),
    );
    let mut s = 99u64;
    let big: Vec<(u32, u32)> = (0..1_000_000u32)
        .map(|i| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 40) as u32 % 5000, i)
        })
        .collect();
    time(
        "any_by 10^6 items",
        3,
        || {
            let mut v = big.clone();
            any_by(&mut v, |a, b| a.0.cmp(&b.0));
            v
        },
        |v| {
            v.iter()
                .fold(0u64, |h, x| (h ^ x.1 as u64).wrapping_mul(0x100000001b3))
        },
    );
}
