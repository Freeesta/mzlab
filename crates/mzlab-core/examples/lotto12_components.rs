//! Times `components::estimate_components` on synthetic matrices and prints a checksum of every output bit, so that two builds compare byte for byte:
//! `cargo run --release --example lotto12_components -p mzlab-core`.
use std::time::Instant;

use mzlab_core::components::estimate_components;

fn matrix(n: usize, m: usize, comps: usize) -> Vec<f64> {
    let mut seed = 987654321u64;
    let mut rnd = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut d = vec![0.0; n * m];
    for c in 0..comps {
        let (centre, width) = (n as f64 * (0.3 + 0.2 * c as f64), n as f64 * 0.06);
        let spec: Vec<f64> = (0..m).map(|_| rnd() * rnd() * 1e4).collect();
        for i in 0..n {
            let z = (i as f64 - centre) / width;
            let e = (-0.5 * z * z).exp();
            for j in 0..m {
                d[i * m + j] += e * spec[j];
            }
        }
    }
    d.iter_mut().for_each(|x| *x += (rnd() - 0.5) * 50.0);
    d
}

fn main() {
    let mut sum = 0u64;
    for &(n, m, comps) in &[(60usize, 30usize, 2usize), (200, 80, 3), (400, 150, 3)] {
        let d = matrix(n, m, comps);
        let t = Instant::now();
        let r = estimate_components(&d, n, m, 5, 0.002);
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let mut h = r.k as u64;
        for v in r.sv.iter().chain(r.sigma.iter()).chain(r.edge.iter()) {
            h = h.wrapping_mul(1099511628211) ^ v.to_bits();
        }
        for &s in &r.efa_start {
            h = h.wrapping_mul(1099511628211) ^ s as u64;
        }
        sum ^= h;
        println!(
            "{n}x{m}: {ms:8.2} ms  k={} efa={:?} hash={h:016x}",
            r.k, r.efa_start
        );
    }
    println!("total hash {sum:016x}");
}
