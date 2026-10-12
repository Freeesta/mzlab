//! Stability of the chosen shape under resampling: leave one point out (no replicates) or bootstrap (replicates). The proof counts only if the
//! same shape wins in at least 80 % of the fits. The random generator is `SplitMix64`: the same seed gives the same bytes everywhere.

use super::forms::{Form, FormBank};

#[derive(Clone, Copy, Debug)]
pub enum Resample {
    LeaveOneOut,
    Bootstrap { draws: u32, seed: u64 },
}

/// Shape of the full fit and how often the resampled fits agree with it.
#[derive(Clone, Copy, Debug)]
pub struct Stability {
    pub best: Form,
    pub n: u32,
    pub same: u32,
    pub counts: [u32; 4],
}

impl Stability {
    pub fn share(&self) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            self.same as f64 / self.n as f64
        }
    }

    pub fn stable(&self) -> bool {
        self.n > 0 && self.same as f64 >= 0.8 * self.n as f64
    }
}

pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl FormBank {
    /// Shape of the full fit and of the resampled ones. `None` under 4 points (a point left out would leave fewer than 3).
    pub fn stability(&self, y: &[f64], mode: Resample) -> Option<Stability> {
        let m = y.len();
        if m < 4 || m != self.n_times() {
            return None;
        }
        let all: Vec<usize> = (0..m).collect();
        let best = self.fit_cols(y, &all).first()?.form;
        let mut st = Stability {
            best,
            n: 0,
            same: 0,
            counts: [0; 4],
        };
        let mut rng = SplitMix64(match mode {
            Resample::Bootstrap { seed, .. } => seed,
            Resample::LeaveOneOut => 0,
        });
        let runs = match mode {
            Resample::LeaveOneOut => m as u32,
            Resample::Bootstrap { draws, .. } => draws,
        };
        for r in 0..runs as usize {
            let cols: Vec<usize> = match mode {
                Resample::LeaveOneOut => all.iter().copied().filter(|&j| j != r).collect(),
                Resample::Bootstrap { .. } => {
                    (0..m).map(|_| (rng.next() % m as u64) as usize).collect()
                }
            };
            if let Some(f) = self.fit_cols(y, &cols).first() {
                st.n += 1;
                st.counts[f.form as usize] += 1;
                st.same += (f.form == best) as u32;
            }
        }
        Some(st)
    }
}
