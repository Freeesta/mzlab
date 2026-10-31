//! The synthetic traces of the benchmark: cases with their true peaks, the noise and the true areas and limits.

pub const DT: f64 = 0.01;
pub const N: usize = 600;
pub const SEEDS: u64 = 30;

/// One true peak: centre (min), sigma (min), height of the Gaussian, exponential tail tau (min, 0 = none).
#[derive(Clone, Copy)]
pub struct True {
    pub c: f64,
    pub s: f64,
    pub h: f64,
    pub tau: f64,
}

pub struct Case {
    pub name: &'static str,
    pub peaks: Vec<True>,
    pub noise: f64,
    /// baseline: offset + slope * t + curve * t^2, or (-1, centre, height) for a broad Gaussian hump of sigma 0.6 min on 1e3
    pub base: (f64, f64, f64),
}

pub fn g(c: f64, s: f64, h: f64) -> True {
    True { c, s, h, tau: 0.0 }
}

fn case(name: &'static str, peaks: Vec<True>, base: (f64, f64, f64)) -> Case {
    Case {
        name,
        peaks,
        noise: 1.0e3,
        base,
    }
}

pub fn cases() -> Vec<Case> {
    let s = 0.05;
    let flat = (1.0e3, 0.0, 0.0);
    let tail = True {
        c: 3.0,
        s,
        h: 1.0e5,
        tau: 0.08,
    };
    let five = vec![
        g(0.8, s, 1.0e5),
        g(1.8, s, 3.0e4),
        g(2.8, s, 1.0e4),
        g(3.8, 0.06, 2.0e4),
        g(4.8, s, 5.0e3),
    ];
    vec![
        case("single, S/N 100", vec![g(3.0, s, 1.0e5)], flat),
        case("noisy, S/N 10", vec![g(3.0, s, 1.0e4)], flat),
        case("low S/N 5", vec![g(3.0, s, 5.0e3)], flat),
        case("tailing tau/sigma 1.6", vec![tail], flat),
        case(
            "pair Rs 1.5, 1:0.6",
            vec![g(2.85, s, 1.0e5), g(3.15, s, 6.0e4)],
            flat,
        ),
        case(
            "pair Rs 0.8, 1:0.6",
            vec![g(2.92, s, 1.0e5), g(3.08, s, 6.0e4)],
            flat,
        ),
        case(
            "shoulder Rs 0.5, 1:0.4",
            vec![g(2.95, s, 1.0e5), g(3.05, s, 4.0e4)],
            flat,
        ),
        case(
            "drifting baseline",
            vec![g(3.0, s, 1.0e5)],
            (2.0e3, 1.0e4, 1.5e3),
        ),
        case(
            "broad hump under the peak",
            vec![g(3.0, s, 1.0e5)],
            (-1.0, 3.2, 2.0e4),
        ),
        case("five peaks 1e5..5e3", five, flat),
        case("no peak", vec![], flat),
    ]
}

/// Deterministic standard normal noise (LCG + Box-Muller).
pub struct Rng(pub u64);
impl Rng {
    fn u(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn n(&mut self) -> f64 {
        (-2.0 * self.u().ln()).sqrt() * (2.0 * std::f64::consts::PI * self.u()).cos()
    }
}

/// The clean shape of one peak on the grid: a Gaussian, convolved with a unit-area exponential when tau > 0 (the area is kept).
pub fn shape(rt: &[f64], p: &True) -> Vec<f64> {
    let gauss: Vec<f64> = rt
        .iter()
        .map(|t| p.h * (-((t - p.c) / p.s).powi(2) / 2.0).exp())
        .collect();
    if p.tau <= 0.0 {
        return gauss;
    }
    let k: Vec<f64> = (0..rt.len())
        .map(|i| (-(i as f64) * DT / p.tau).exp() * DT / p.tau)
        .collect();
    let ks: f64 = k.iter().sum();
    (0..rt.len())
        .map(|i| (0..=i).map(|j| gauss[j] * k[i - j]).sum::<f64>() / ks)
        .collect()
}

/// The trace, the clean shape of each peak and the true area of each (counts x min).
pub fn draw(c: &Case, seed: u64) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let rt: Vec<f64> = (0..N).map(|i| i as f64 * DT).collect();
    let shapes: Vec<Vec<f64>> = c.peaks.iter().map(|p| shape(&rt, p)).collect();
    let mut r = Rng(seed * 7919 + 13);
    let y = (0..N)
        .map(|i| {
            let t = rt[i];
            let sig: f64 = shapes.iter().map(|s| s[i]).sum();
            // additive detector noise plus 3 % ion-statistics noise on the signal
            let bl = if c.base.0 < 0.0 {
                1.0e3 + c.base.2 * (-((t - c.base.1) / 0.6).powi(2) / 2.0).exp()
            } else {
                c.base.0 + c.base.1 * t / 6.0 + c.base.2 * (t / 6.0).powi(2)
            };
            bl + sig * (1.0 + 0.03 * r.n()) + c.noise * r.n()
        })
        .collect();
    (rt, y, shapes)
}

pub fn true_area(p: &True) -> f64 {
    p.h * p.s * (2.0 * std::f64::consts::PI).sqrt()
}

/// Limits of the clean shape: where it stays above 1 % of its maximum.
pub fn true_limits(s: &[f64]) -> (usize, usize, usize) {
    let (mut apex, mut mx) = (0, f64::MIN);
    for (i, &v) in s.iter().enumerate() {
        if v > mx {
            mx = v;
            apex = i;
        }
    }
    let lo = s.iter().position(|&v| v >= 0.01 * mx).unwrap_or(0);
    let hi = s
        .iter()
        .rposition(|&v| v >= 0.01 * mx)
        .unwrap_or(s.len() - 1);
    (lo, apex, hi)
}
