//! Bank of model curves on a fixed vector of times and the least-squares search over it (port of `kinetics.fit_chain`).
//! Every row is one parameter set of one model; the curve is stored as its square root (the variance-stabilising scale of the fit).

/// Parameter grid of the rate constants, 1/min: `np.geomspace(0.003, 3.0, 28)`.
pub fn grid() -> Vec<f64> {
    const N: usize = 28;
    let (a, b) = (libm::log10(0.003), libm::log10(3.0));
    (0..N)
        .map(|i| match i {
            0 => 0.003,
            _ if i == N - 1 => 3.0,
            _ => libm::pow(10.0, a + (b - a) * i as f64 / (N - 1) as f64),
        })
        .collect()
}

/// `AICc` of a fit with sum of squares `sse` on `n` points and `k` parameters (the scale included).
pub fn aicc(sse: f64, n: usize, k: usize) -> f64 {
    let (n, k) = (n as f64, k as f64);
    n * libm::log(sse.max(1e-300) / n) + 2.0 * k + 2.0 * k * (k + 1.0) / (n - k - 1.0).max(1.0)
}

/// One parameter set: `group` is the model it belongs to, `p` the first `np` rate constants.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub group: u8,
    pub p: [f64; 3],
}

/// Best row of a model on a set of columns: its index, the sum of squares and the scale (in the normalised square-root space).
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub row: usize,
    pub sse: f64,
    pub scale: f64,
}

pub struct Bank {
    pub rows: Vec<Row>,
    s: Vec<f64>,
    t: usize,
}

impl Bank {
    /// `curve(row, time)` is the concentration of the product in the model of that row.
    pub fn new(rows: Vec<Row>, t: usize, curve: impl Fn(&Row, usize) -> f64) -> Bank {
        let mut s = Vec::with_capacity(rows.len() * t);
        for r in &rows {
            s.extend((0..t).map(|j| curve(r, j).max(0.0).sqrt()));
        }
        Bank { rows, s, t }
    }

    pub fn n_times(&self) -> usize {
        self.t
    }

    /// Square-root normalised data on the columns `cols`: `sqrt(y / max y)` and the maximum.
    pub fn normalise(y: &[f64], cols: &[usize]) -> (Vec<f64>, f64) {
        let ymax = cols.iter().fold(0.0f64, |m, &j| m.max(y[j])).max(1e-300);
        (
            cols.iter()
                .map(|&j| (y[j] / ymax).max(0.0).sqrt())
                .collect(),
            ymax,
        )
    }

    /// For every group the row with the smallest sum of squares on `cols` (first on ties, like `np.argmin`); the scale is solved in closed form.
    pub fn fit_groups(&self, cols: &[usize], yt: &[f64], groups: usize) -> Vec<Option<Hit>> {
        let yy: f64 = yt.iter().map(|v| v * v).sum();
        let whole = cols.len() == self.t && cols.iter().enumerate().all(|(k, &j)| k == j);
        let mut best: Vec<Option<Hit>> = vec![None; groups];
        for (i, r) in self.rows.iter().enumerate() {
            let s = &self.s[i * self.t..(i + 1) * self.t];
            let (mut smax, mut ss, mut sy) = (0.0f64, 0.0f64, 0.0f64);
            if whole {
                for (&v, &w) in s.iter().zip(yt) {
                    smax = smax.max(v);
                    ss += v * v;
                    sy += v * w;
                }
            } else {
                for (k, &j) in cols.iter().enumerate() {
                    smax = smax.max(s[j]);
                    ss += s[j] * s[j];
                    sy += s[j] * yt[k];
                }
            }
            let hit = if smax > 0.0 {
                let gg = (ss / (smax * smax)).max(1e-300);
                let gy = sy / smax;
                Hit {
                    row: i,
                    sse: yy - gy * gy / gg,
                    scale: gy / gg,
                }
            } else {
                Hit {
                    row: i,
                    sse: yy,
                    scale: 0.0,
                }
            };
            let slot = &mut best[r.group as usize];
            if slot.map_or(true, |b| hit.sse < b.sse) {
                *slot = Some(hit);
            }
        }
        best
    }
}
