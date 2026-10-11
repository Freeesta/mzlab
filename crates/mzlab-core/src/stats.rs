//! Small statistics of `mzlab/ionfamily.py` (`rankdata`, `wpearson`, `wspearman`, `kendall_tau`, `theil_sen`, `chi2_sf`, `perm_pvalue`),
//! with the same rules, ties and thresholds. The permutation p-value draws its permutations from the numpy generator
//! (`np.random.default_rng(seed)`: SeedSequence + PCG64 + `shuffle`), so that the Monte Carlo value is the same as Python's.

/// Average ranks (ties share the mean rank), 1-based.
pub fn rankdata(a: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..a.len()).collect();
    order.sort_by(|&i, &j| a[i].total_cmp(&a[j]));
    let mut r = vec![0.0; a.len()];
    let mut i = 0;
    while i < a.len() {
        let mut j = i;
        while j + 1 < a.len() && a[order[j + 1]] == a[order[i]] {
            j += 1;
        }
        for &o in &order[i..=j] {
            r[o] = (i + j) as f64 / 2.0 + 1.0;
        }
        i = j + 1;
    }
    r
}

/// `ndarray.sum()` of a contiguous float64 array: numpy adds pairwise (blocks of 8 accumulators), so a constant vector gives the same
/// exact zero after centring here as in Python.
pub(crate) fn psum(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut r = 0.0;
        for v in a {
            r += v;
        }
        return r;
    }
    if n <= 128 {
        let mut r: [f64; 8] = a[..8].try_into().unwrap();
        let mut i = 8;
        while i + 8 <= n - n % 8 {
            for k in 0..8 {
                r[k] += a[i + k];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        while i < n {
            res += a[i];
            i += 1;
        }
        return res;
    }
    let mut n2 = n / 2;
    n2 -= n2 % 8;
    psum(&a[..n2]) + psum(&a[n2..])
}

/// Weighted Pearson correlation (NaN if a trace is constant).
pub fn wpearson(x: &[f64], y: &[f64], w: Option<&[f64]>) -> f64 {
    let n = x.len();
    if n < 3 {
        return f64::NAN;
    }
    let ones;
    let w = match w {
        Some(w) => w,
        None => {
            ones = vec![1.0; n];
            &ones
        }
    };
    let sw = psum(w);
    if sw <= 0.0 {
        return f64::NAN;
    }
    let col = |f: &dyn Fn(usize) -> f64| psum(&(0..n).map(f).collect::<Vec<_>>());
    let mx = col(&|i| w[i] * x[i]) / sw;
    let my = col(&|i| w[i] * y[i]) / sw;
    let vx = col(&|i| w[i] * (x[i] - mx) * (x[i] - mx));
    let vy = col(&|i| w[i] * (y[i] - my) * (y[i] - my));
    if vx <= 1e-300 || vy <= 1e-300 {
        return f64::NAN;
    }
    col(&|i| w[i] * (x[i] - mx) * (y[i] - my)) / (vx * vy).sqrt()
}

pub fn wspearman(x: &[f64], y: &[f64], w: Option<&[f64]>) -> f64 {
    wpearson(&rankdata(x), &rankdata(y), w)
}

/// Kendall tau-b (ties corrected).
pub fn kendall_tau(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len();
    if n < 2 {
        return f64::NAN;
    }
    let sg = |v: f64| {
        if v > 0.0 {
            1.0
        } else if v < 0.0 {
            -1.0
        } else {
            0.0
        }
    };
    let (mut c, mut tx, mut ty) = (0.0, 0.0, 0.0);
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (sg(x[i] - x[j]), sg(y[i] - y[j]));
            c += a * b;
            tx += (a != 0.0) as u8 as f64;
            ty += (b != 0.0) as u8 as f64;
        }
    }
    let d = (tx * ty).sqrt();
    if d > 0.0 {
        c / d
    } else {
        f64::NAN
    }
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Theil-Sen line (median of pairwise slopes, median intercept): (slope, intercept). Thinned deterministically above `max_pts`.
pub fn theil_sen(x: &[f64], y: &[f64], max_pts: usize) -> (f64, f64) {
    let (mut x, mut y) = (x.to_vec(), y.to_vec());
    if x.len() > max_pts {
        // np.linspace(0, n - 1, max_pts).astype(int)
        let step = (x.len() - 1) as f64 / (max_pts - 1) as f64;
        let idx: Vec<usize> = (0..max_pts)
            .map(|i| {
                if i == max_pts - 1 {
                    x.len() - 1
                } else {
                    (i as f64 * step) as usize
                }
            })
            .collect();
        x = idx.iter().map(|&i| x[i]).collect();
        y = idx.iter().map(|&i| y[i]).collect();
    }
    let amax = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
    let tol = 1e-12 * if x.is_empty() { 1.0 } else { amax.max(1.0) };
    let mut slopes = vec![];
    for i in 0..x.len() {
        for j in i + 1..x.len() {
            let dx = x[i] - x[j];
            if dx.abs() > tol {
                slopes.push((y[i] - y[j]) / dx);
            }
        }
    }
    if slopes.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    let s = median(&mut slopes);
    let mut r: Vec<f64> = x.iter().zip(&y).map(|(a, b)| b - s * a).collect();
    (s, median(&mut r))
}

/// ln Gamma (Lanczos, g = 7): about 1e-15 relative.
fn lgamma(x: f64) -> f64 {
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin().abs()).ln()
            - lgamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = C[0];
    let t = x + 7.5;
    for (i, c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Regularised upper incomplete gamma Q(a, x); the chi-square survival is Q(df / 2, chi2 / 2).
fn gammainc_q(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    if x < a + 1.0 {
        let (mut ap, mut s, mut d) = (a, 1.0 / a, 1.0 / a);
        for _ in 0..500 {
            ap += 1.0;
            d *= x / ap;
            s += d;
            if d.abs() < s.abs() * 1e-14 {
                break;
            }
        }
        return (1.0 - s * (-x + a * x.ln() - lgamma(a)).exp()).max(0.0);
    }
    let mut b = x + 1.0 - a;
    let mut c = 1e300;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..500 {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        c = b + an / c;
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1.0 / d;
        let de = d * c;
        h *= de;
        if (de - 1.0).abs() < 1e-14 {
            break;
        }
    }
    ((-x + a * x.ln() - lgamma(a)).exp() * h).min(1.0)
}

pub fn chi2_sf(x: f64, df: u32) -> f64 {
    gammainc_q(df as f64 / 2.0, x / 2.0)
}

/// numpy's `SeedSequence(seed).generate_state(4, uint64)` for a non-negative integer seed below 2^32.
fn seed_state(seed: u32) -> [u64; 4] {
    const MULT_A: u32 = 0x931e8875;
    const MULT_B: u32 = 0x58f38ded;
    const MIX_L: u32 = 0xca01f9dd;
    const MIX_R: u32 = 0x4973f715;
    let hashmix = |v: u32, h: &mut u32| {
        let mut v = v ^ *h;
        *h = h.wrapping_mul(MULT_A);
        v = v.wrapping_mul(*h);
        v ^ (v >> 16)
    };
    let mix = |x: u32, y: u32| {
        let r = MIX_L.wrapping_mul(x).wrapping_sub(MIX_R.wrapping_mul(y));
        r ^ (r >> 16)
    };
    let mut h = 0x43b0d7e5u32;
    let mut pool = [0u32; 4];
    for (i, p) in pool.iter_mut().enumerate() {
        *p = hashmix(if i == 0 { seed } else { 0 }, &mut h);
    }
    for s in 0..4 {
        for d in 0..4 {
            if s != d {
                let hs = hashmix(pool[s], &mut h);
                pool[d] = mix(pool[d], hs);
            }
        }
    }
    let mut hb = 0x8b51f9ddu32;
    let mut w = [0u32; 8];
    for (i, o) in w.iter_mut().enumerate() {
        let mut v = pool[i % 4] ^ hb;
        hb = hb.wrapping_mul(MULT_B);
        v = v.wrapping_mul(hb);
        *o = v ^ (v >> 16);
    }
    [0, 1, 2, 3].map(|i| w[2 * i] as u64 | (w[2 * i + 1] as u64) << 32)
}

/// numpy's PCG64 (XSL-RR 128/64) with its buffered 32-bit output.
struct Pcg64 {
    state: u128,
    inc: u128,
    half: Option<u32>,
}

impl Pcg64 {
    const MULT: u128 = 0x2360ED051FC65DA44385DF649FCCF645;
    fn new(seed: u32) -> Pcg64 {
        let v = seed_state(seed);
        let init = (v[0] as u128) << 64 | v[1] as u128;
        let seq = (v[2] as u128) << 64 | v[3] as u128;
        let mut g = Pcg64 {
            state: 0,
            inc: seq << 1 | 1,
            half: None,
        };
        g.step();
        g.state = g.state.wrapping_add(init);
        g.step();
        g
    }
    fn step(&mut self) {
        self.state = self.state.wrapping_mul(Self::MULT).wrapping_add(self.inc);
    }
    fn next64(&mut self) -> u64 {
        self.step();
        let x = ((self.state >> 64) as u64) ^ (self.state as u64);
        x.rotate_right((self.state >> 122) as u32)
    }
    fn next32(&mut self) -> u32 {
        if let Some(h) = self.half.take() {
            return h;
        }
        let n = self.next64();
        self.half = Some((n >> 32) as u32);
        n as u32
    }
    /// `random_interval(max)`: a uniform integer in `0..=max` by masked rejection.
    fn interval(&mut self, max: u32) -> u32 {
        if max == 0 {
            return 0;
        }
        let mask = u32::MAX >> max.leading_zeros();
        loop {
            let v = self.next32() & mask;
            if v <= max {
                return v;
            }
        }
    }
    /// `rng.permutation(n)`.
    fn permutation(&mut self, n: usize) -> Vec<usize> {
        let mut a: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = self.interval(i as u32) as usize;
            a.swap(i, j);
        }
        a
    }
}

#[derive(Clone, Copy)]
pub enum Stat {
    Pearson,
    Spearman,
    Kendall,
}

impl Stat {
    fn eval(self, x: &[f64], y: &[f64]) -> f64 {
        match self {
            Stat::Pearson => wpearson(x, y, None),
            Stat::Spearman => wspearman(x, y, None),
            Stat::Kendall => kendall_tau(x, y),
        }
    }
}

fn heap_permutations(n: usize) -> Vec<Vec<usize>> {
    // itertools.permutations order (lexicographic); the order does not change a count
    let mut out = vec![];
    let mut cur: Vec<usize> = (0..n).collect();
    loop {
        out.push(cur.clone());
        let Some(i) = (0..n.saturating_sub(1))
            .rev()
            .find(|&i| cur[i] < cur[i + 1])
        else {
            break;
        };
        let j = (i + 1..n).rev().find(|&j| cur[j] > cur[i]).unwrap();
        cur.swap(i, j);
        cur[i + 1..].reverse();
    }
    out
}

/// Two-sided permutation p-value of an association statistic; exact for n <= 7, Monte Carlo above.
pub fn perm_pvalue(stat: Stat, x: &[f64], y: &[f64], n_perm: usize, seed: u32) -> f64 {
    let n = x.len();
    let obs = stat.eval(x, y).abs();
    if !obs.is_finite() {
        return f64::NAN;
    }
    let perms = if n <= 7 {
        heap_permutations(n)
    } else {
        let mut g = Pcg64::new(seed);
        (0..n_perm).map(|_| g.permutation(n)).collect()
    };
    let hit = perms
        .iter()
        .filter(|p| {
            let yp: Vec<f64> = p.iter().map(|&i| y[i]).collect();
            stat.eval(x, &yp).abs() >= obs - 1e-12
        })
        .count();
    if n <= 7 {
        hit as f64 / perms.len() as f64
    } else {
        (hit + 1) as f64 / (perms.len() + 1) as f64
    }
}
