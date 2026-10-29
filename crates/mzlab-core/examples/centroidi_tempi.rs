//! Times the profile centroider on made-up counts scans of N points on the 0.06 Da grid and prints the checksum of the output (identical before and after a change).
//! `cargo run --release --example centroidi_tempi -p mzlab-core -- [POINTS] [SCANS] [PEAKS]`
use std::time::Instant;

use mzlab_core::centroidi::{centroids, Buf, Mode, Params};

fn main() {
    let a: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    let (n, scans, peaks) = (
        *a.first().unwrap_or(&9000),
        *a.get(1).unwrap_or(&50),
        *a.get(2).unwrap_or(&250),
    );
    let mut seed = 11u64;
    let mut r = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let p = Params {
        mode: Mode::Counts,
        unit: 1.0,
        step: 0.06,
        w: 0.7,
        k: 3.0,
        vmax: 0.0,
        area: true,
    };
    let centres: Vec<(f64, f64)> = (0..peaks)
        .map(|_| (r() * n as f64 * 0.06, 5.0 + 400.0 * r() * r()))
        .collect();
    let (mut buf, mut sum, mut count) = (Buf::default(), 0u64, 0usize);
    let mut spent = 0.0;
    for _ in 0..scans {
        let (mut mz, mut y) = (vec![], vec![]);
        for b in 0..n {
            let x = 100.0 + b as f64 * 0.06;
            let lam = 0.4
                + centres
                    .iter()
                    .map(|&(c, h)| h * (-(x - 100.0 - c).powi(2) / (2.0 * 0.297f64.powi(2))).exp())
                    .sum::<f64>();
            let k = (lam + lam.sqrt() * (r() + r() + r() - 1.5) * 2.0)
                .round()
                .max(0.0);
            if k > 0.0 {
                mz.push(x);
                y.push(k);
            }
        }
        let t0 = Instant::now();
        let out = centroids(&mz, &y, &p, &mut buf);
        spent += t0.elapsed().as_secs_f64();
        count += out.len();
        for c in &out {
            sum = sum.wrapping_mul(1099511628211)
                ^ c.mz.to_bits()
                ^ c.value.to_bits().rotate_left(7)
                ^ c.height.to_bits().rotate_left(13)
                ^ c.flags as u64;
        }
    }
    println!(
        "{n} points: {:.3} ms/scan, {count} centroids, checksum {sum:016x}",
        spent * 1e3 / scans as f64
    );
}
