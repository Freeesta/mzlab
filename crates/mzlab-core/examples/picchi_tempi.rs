//! Times the peak picker with its extras (`tracestat::detect`) on a made-up trace of many scans and many peaks.
//! `cargo run --release --example picchi_tempi -p mzlab-core -- [SCANS] [PEAKS] [REPEATS]`
use std::time::Instant;

use mzlab_core::tracestat::detect;

fn main() {
    let a: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    let (n, peaks, reps) = (
        *a.first().unwrap_or(&20000),
        *a.get(1).unwrap_or(&150),
        *a.get(2).unwrap_or(&5),
    );
    let rt: Vec<f64> = (0..n)
        .map(|k| 0.002 * k as f64 + 1e-5 * (k % 3) as f64)
        .collect();
    let mut seed = 7u64;
    let mut r = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let centers: Vec<(f64, f64)> = (0..peaks)
        .map(|_| (r() * n as f64, 2e5 + 3e6 * r()))
        .collect();
    let y: Vec<f64> = (0..n)
        .map(|k| {
            let k = k as f64;
            centers
                .iter()
                .map(|(c, h)| h * (-(k - c).powi(2) / 40.0).exp())
                .sum::<f64>()
                + 3e3 * r()
        })
        .collect();
    let mut found = 0;
    let t = Instant::now();
    let mut check = 0f64;
    for _ in 0..reps {
        let d = detect(&rt, &y);
        found = d.len();
        check = d
            .iter()
            .map(|p| p.p.area + p.e.apex_rt + p.e.apex_sigma + p.e.fwhm_min)
            .sum();
    }
    println!(
        "{n} scans, {found} peaks: {:.3} s per trace, check {check:.9e}",
        t.elapsed().as_secs_f64() / reps as f64
    );
}
