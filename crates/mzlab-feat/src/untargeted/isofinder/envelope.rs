//! Expected isotope envelope of an ion from its mass alone (the carbon model of the «Isotopic peaks finder» of mzmine 4.10.90).
//!
//! After `signal/CarbonEnvelopeModel.java` and `engine/IsotopeEnvelope.java` (MIT, (c) 2004-2022 The mzmine Development Team); rewritten, no code copied.
//! The number of carbons follows from the mass (carbons per dalton: typical for the expected shape, maximum for the upper bound), the 13C
//! count is Poisson (or binomial), and every other element of the list adds a stepped binomial of its dominant heavy isotope.

use mzlab_core::isoscore::isotopes_of;

const P_13C: f64 = 0.0107;
const HEAVY_MASS_PER_ATOM: f64 = 200.0;
const MAX_HEAVY_ATOMS: i64 = 8;
const CAP: usize = 30;
/// Offsets whose expected relative height is under this are not counted as support.
pub const SUPPORT_CUTOFF: f64 = 0.02;
pub const PROTON_MASS: f64 = 1.007276466;
/// Distance between 13C and 12C.
pub const C13_DISTANCE: f64 = 1.003354831;

/// Java `Math.round` for the values of this module: half up.
pub fn jround(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

/// Settings of the carbon model (mzmine defaults: 1/20, 1/14, 1/12 carbons per Da, minimum relative height 0.01, Poisson).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarbonModel {
    pub c_per_da_min: f64,
    pub c_per_da_typical: f64,
    pub c_per_da_max: f64,
    pub min_rel: f64,
    pub poisson: bool,
}

impl Default for CarbonModel {
    fn default() -> Self {
        CarbonModel {
            c_per_da_min: 1.0 / 20.0,
            c_per_da_typical: 1.0 / 14.0,
            c_per_da_max: 1.0 / 12.0,
            min_rel: 0.01,
            poisson: true,
        }
    }
}

/// Expected (`expected`) and upper-bound (`upper`) relative heights by offset in 13C steps, both starting at the monoisotopic offset 0.
#[derive(Clone, Debug, PartialEq)]
pub struct Envelope {
    pub expected: Vec<f64>,
    pub upper: Vec<f64>,
    pub spacing: f64,
    pub charge: i32,
}

impl Envelope {
    pub fn max_offset(&self) -> i64 {
        self.expected.len() as i64 - 1
    }
    pub fn expected_at(&self, o: i64) -> f64 {
        if o >= 0 && (o as usize) < self.expected.len() {
            self.expected[o as usize]
        } else {
            0.0
        }
    }
    pub fn upper_at(&self, o: i64) -> f64 {
        if o >= 0 && (o as usize) < self.upper.len() {
            self.upper[o as usize]
        } else {
            0.0
        }
    }
    /// Offset of the highest expected value (the first if equal).
    pub fn base_offset(&self) -> i64 {
        let mut idx = 0;
        let mut max = -1.0;
        for (i, v) in self.expected.iter().enumerate() {
            if *v > max {
                max = *v;
                idx = i;
            }
        }
        idx as i64
    }
}

/// Neutral mass of an ion of m/z `mz` and charge `z`; `sign` is +1 (positive), -1 (negative) or 0 (unknown). A non-positive result falls back to `mz * z`.
pub fn neutral_mass(mz: f64, z: i32, sign: i32) -> f64 {
    let m = mz * z as f64 - z as f64 * PROTON_MASS * sign as f64;
    if m > 0.0 {
        m
    } else {
        mz * z as f64
    }
}

/// Lower and upper bound of the M+1 / M ratio from the carbons per dalton.
pub fn m1_bounds(m: &CarbonModel, mz: f64, z: i32, sign: i32) -> [f64; 2] {
    let nm = neutral_mass(mz, z, sign);
    [
        jround(nm * m.c_per_da_min).max(0) as f64 * P_13C,
        jround(nm * m.c_per_da_max).max(0) as f64 * P_13C,
    ]
}

/// The heavy isotope that dominates the elements other than C and H: `(step in nominal masses, abundance relative to the sum)`.
/// Isotopes lighter than one nominal mass above the main one are skipped.
pub fn heavy_contribution(symbol: &str) -> Option<(usize, f64)> {
    if symbol == "C" || symbol == "H" {
        return None;
    }
    let iso = isotopes_of(symbol)?;
    let main = iso
        .iter()
        .fold(iso[0], |m, i| if i.1 > m.1 { *i } else { m });
    let mut best: Option<(f64, f64)> = None; // (delta mass, relative intensity)
    for i in iso.iter().filter(|i| i.0 != main.0) {
        let delta = (i.0 - main.0).abs();
        let rel = i.1 / main.1;
        if jround(delta) < 1 {
            continue;
        }
        if best.is_none_or(|b| rel > b.1) {
            best = Some((delta, rel));
        }
    }
    best.map(|(d, rel)| (jround(d) as usize, rel / (1.0 + rel)))
}

fn poisson(lambda: f64) -> Vec<f64> {
    let mut p = vec![0.0; CAP + 1];
    p[0] = (-lambda).exp();
    for k in 1..=CAP {
        p[k] = p[k - 1] * lambda / k as f64;
    }
    p
}

fn binomial(n: i64, prob: f64) -> Vec<f64> {
    let mut p = vec![0.0; CAP + 1];
    if n <= 0 || prob <= 0.0 {
        p[0] = 1.0;
        return p;
    }
    let q = 1.0 - prob;
    p[0] = q.powi(n as i32);
    for k in 1..=(n as usize).min(CAP) {
        p[k] = p[k - 1] * ((n - k as i64 + 1) as f64 / k as f64) * (prob / q);
    }
    p
}

fn stepped_binomial(n: i64, abundance: f64, step: usize) -> Vec<f64> {
    let bin = binomial(n, abundance);
    let mut dist = vec![0.0; CAP + 1];
    let mut k = 0usize;
    while k * step <= CAP && (k as i64) <= n {
        dist[k * step] += bin[k];
        k += 1;
    }
    dist
}

fn convolve(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut r = vec![0.0; CAP + 1];
    for (i, av) in a.iter().enumerate().take(CAP + 1) {
        if *av == 0.0 {
            continue;
        }
        for (j, bv) in b.iter().enumerate() {
            if i + j > CAP {
                break;
            }
            r[i + j] += av * bv;
        }
    }
    r
}

fn normalize_to_max(a: &[f64]) -> Vec<f64> {
    let max = a.iter().fold(0.0f64, |m, v| m.max(*v));
    if max <= 0.0 {
        return vec![1.0];
    }
    a.iter().map(|v| v / max).collect()
}

fn max_of(a: &[f64], b: &[f64]) -> Vec<f64> {
    (0..a.len().max(b.len()))
        .map(|i| {
            a.get(i)
                .copied()
                .unwrap_or(0.0)
                .max(b.get(i).copied().unwrap_or(0.0))
        })
        .collect()
}

/// The envelope is cut after the last offset where the expected or the upper value reaches `min_rel`.
fn trim(expected: &[f64], upper: &[f64], min_rel: f64, spacing: f64, charge: i32) -> Envelope {
    let len = expected.len().max(upper.len());
    let at = |v: &[f64], i: usize| v.get(i).copied().unwrap_or(0.0);
    let mut last = 0;
    for i in 0..len {
        if at(expected, i) >= min_rel || at(upper, i) >= min_rel {
            last = i;
        }
    }
    Envelope {
        expected: (0..=last).map(|i| at(expected, i)).collect(),
        upper: (0..=last).map(|i| at(upper, i)).collect(),
        spacing,
        charge,
    }
}

impl CarbonModel {
    fn carbons(&self, n: i64) -> Vec<f64> {
        if self.poisson {
            poisson(n as f64 * P_13C)
        } else {
            binomial(n, P_13C)
        }
    }

    /// Envelope of an ion. `user_heavies` are the symbols of the elements the user listed (their heavy isotope enters the upper bound with a crude
    /// atom count from the mass) and `include_user_heavies` switches that off; `detected` are `(symbol, count)` of elements found in the spectrum
    /// (they widen the upper bound, the largest candidate wins).
    pub fn build(
        &self,
        mz: f64,
        z: i32,
        sign: i32,
        user_heavies: &[&str],
        detected: Option<&[(&str, i64)]>,
        include_user_heavies: bool,
    ) -> Envelope {
        let nm = neutral_mass(mz, z, sign);
        let n_typ = jround(nm * self.c_per_da_typical).max(0);
        let n_max = jround(nm * self.c_per_da_max).max(0);
        let expected_c = self.carbons(n_typ);
        let mut upper_raw = self.carbons(n_max);
        if include_user_heavies {
            let crude = jround(nm / HEAVY_MASS_PER_ATOM).clamp(1, MAX_HEAVY_ATOMS);
            for sym in user_heavies {
                if let Some((step, ab)) = heavy_contribution(sym) {
                    upper_raw = convolve(&upper_raw, &stepped_binomial(crude, ab, step));
                }
            }
        }
        if let Some(found) = detected {
            let mut best: Option<Vec<f64>> = None;
            for (sym, count) in found.iter().filter(|(_, c)| *c > 0) {
                let Some((step, ab)) = heavy_contribution(sym) else {
                    continue;
                };
                let cand = convolve(&upper_raw, &stepped_binomial(*count, ab, step));
                best = Some(match best {
                    None => cand,
                    Some(b) => max_of(&b, &cand),
                });
            }
            if let Some(b) = best {
                upper_raw = b;
            }
        }
        let expected = normalize_to_max(&expected_c);
        let mut upper = normalize_to_max(&upper_raw);
        for i in 0..upper.len().min(expected.len()) {
            upper[i] = upper[i].max(expected[i]);
        }
        trim(&expected, &upper, self.min_rel, C13_DISTANCE / z as f64, z)
    }
}
