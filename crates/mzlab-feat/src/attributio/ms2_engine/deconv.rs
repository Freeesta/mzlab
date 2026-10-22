//! Library-assisted deconvolution of a chimeric MS2 spectrum: the spectrum is explained as a non-negative mix of the library spectra whose precursor lies in
//! its isolation window (non-negative least squares by coordinate descent, deterministic). Used by the executor of `spectra.chimeric`; the spectrum itself is
//! never changed: the shares and the unexplained part are evidence for the chemist.

/// A library spectrum that may have been co-isolated.
pub struct Candidate<'a> {
    pub mz: &'a [f64],
    pub it: &'a [f64],
}

/// What the mix explains: the share of the query intensity of each candidate (0 for those the mix does not use) and the share left unexplained.
#[derive(Debug, Clone, PartialEq)]
pub struct Deconv {
    pub shares: Vec<f64>,
    pub residual: f64,
}

const SWEEPS: usize = 500;

/// Intensity of `cand` within `tol` Da of `mz`, the largest one.
fn at(cand: &Candidate, mz: f64, tol: f64) -> f64 {
    let top = |(m, i): (&f64, &f64)| ((m - mz).abs() <= tol).then_some(*i);
    cand.mz
        .iter()
        .zip(cand.it)
        .filter_map(top)
        .fold(0.0, f64::max)
}

/// `None` when the query has no intensity. Query and candidates are scaled to a maximum of 1 first.
pub fn deconvolve(mz: &[f64], it: &[f64], cands: &[Candidate], tol: f64) -> Option<Deconv> {
    let top = it.iter().copied().fold(0.0, f64::max);
    if mz.is_empty() || top <= 0.0 {
        return None;
    }
    let q: Vec<f64> = it.iter().map(|v| v / top).collect();
    let total: f64 = q.iter().sum();
    let cols: Vec<Vec<f64>> = cands
        .iter()
        .map(|c| {
            let hi =
                c.it.iter()
                    .copied()
                    .fold(0.0, f64::max)
                    .max(f64::MIN_POSITIVE);
            mz.iter().map(|&m| at(c, m, tol) / hi).collect()
        })
        .collect();
    let norm: Vec<f64> = cols.iter().map(|c| c.iter().map(|v| v * v).sum()).collect();
    let mut w = vec![0.0; cols.len()];
    let mut fit = vec![0.0; q.len()];
    for _ in 0..SWEEPS {
        let mut moved = 0.0f64;
        for (j, col) in cols.iter().enumerate().filter(|(j, _)| norm[*j] > 0.0) {
            let grad: f64 = col
                .iter()
                .zip(q.iter().zip(&fit))
                .map(|(a, (y, f))| a * (y - f))
                .sum();
            let next = (w[j] + grad / norm[j]).max(0.0);
            let d = next - w[j];
            for (f, a) in fit.iter_mut().zip(col) {
                *f += d * a;
            }
            w[j] = next;
            moved = moved.max(d.abs());
        }
        if moved < 1e-10 {
            break;
        }
    }
    let shares = cols
        .iter()
        .zip(&w)
        .map(|(c, w)| w * c.iter().map(|v| v.min(1.0)).sum::<f64>() / total)
        .collect();
    let left: f64 = q.iter().zip(&fit).map(|(y, f)| (y - f).max(0.0)).sum();
    Some(Deconv {
        shares,
        residual: left / total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mix_of_two_library_spectra_is_split_in_its_two_parts() {
        let a = Candidate {
            mz: &[100.0, 150.0],
            it: &[1.0, 0.5],
        };
        let b = Candidate {
            mz: &[120.0, 180.0],
            it: &[1.0, 1.0],
        };
        let c = Candidate {
            mz: &[400.0],
            it: &[1.0],
        };
        // 2 parts of a, 1 part of b
        let mz = [100.0, 120.0, 150.0, 180.0];
        let it = [2.0, 1.0, 1.0, 1.0];
        let d = deconvolve(&mz, &it, &[a, b, c], 0.02).unwrap();
        assert!(d.residual < 1e-6, "{d:?}");
        assert!(d.shares[2] == 0.0);
        assert!(
            (d.shares[0] - 0.6).abs() < 1e-6 && (d.shares[1] - 0.4).abs() < 1e-6,
            "{d:?}"
        );
    }

    #[test]
    fn a_peak_no_candidate_explains_stays_in_the_residual() {
        let a = Candidate {
            mz: &[100.0],
            it: &[1.0],
        };
        let d = deconvolve(&[100.0, 250.0], &[1.0, 1.0], &[a], 0.02).unwrap();
        assert!((d.residual - 0.5).abs() < 1e-6, "{d:?}");
        assert!(deconvolve(&[], &[], &[], 0.02).is_none());
    }
}
