//! Small helpers of the target search: noise estimate, m/z windows, adduct element counts, isotope fallback peak.

use super::*;

/// Median of `buf` (reordered): by selection, with the full sort kept for the NaN case (the order of NaN there is the sort's).
pub(super) fn median_of(buf: &mut [f64]) -> f64 {
    if let Some(m) = mzlab_core::sortc::select_median(buf) {
        return m;
    }
    mzlab_core::sortc::by(buf, |a, b| a.total_cmp(b));
    let mid = buf.len() / 2;
    if buf.len().is_multiple_of(2) && buf.len() > 1 {
        (buf[mid - 1] + buf[mid]) * 0.5
    } else {
        buf[mid]
    }
}

/// Estimate chromatographic baseline noise using Median Absolute Deviation (MAD); `buf` is scratch space reused between calls.
/// A trace that is flat for more than half of its points has MAD 0: the noise is then the fixed 1.0 (intensity units), which makes S/N equal to the raw height.
pub(super) fn estimate_noise(y: &[f64], buf: &mut Vec<f64>) -> f64 {
    if y.is_empty() {
        return 1.0;
    }
    buf.clear();
    buf.extend_from_slice(y);
    let med = median_of(buf);
    buf.clear();
    buf.extend(y.iter().map(|v| (v - med).abs()));
    let n = median_of(buf) * 1.4826;
    if n > 1e-6 {
        n
    } else {
        1.0
    }
}

/// Helper to add an element count to the Counts vector.
pub(super) fn add_count(counts: &mut Counts, el: &str, delta: i64) {
    if let Some(entry) = counts.iter_mut().find(|(e, _)| e == el) {
        entry.1 += delta;
    } else {
        counts.push((el.to_string(), delta));
    }
}

/// Element counts of the adduct ion of a neutral formula (`None` when the formula does not parse); built once per entry and adduct.
pub(super) fn adduct_counts(neutral: &Counts, adduct: AdductType) -> Counts {
    let mut counts = neutral.clone();
    match adduct {
        AdductType::MH => add_count(&mut counts, "H", 1),
        AdductType::MNa => add_count(&mut counts, "Na", 1),
        AdductType::MNH4 => {
            add_count(&mut counts, "N", 1);
            add_count(&mut counts, "H", 4);
        }
    }
    counts
}

/// Peaks of `spec` with m/z in [lo, hi], in file order. The window starts at a bisection when the m/z array is ascending (`sorted`), and the same filter then keeps only the peaks inside it.
pub(super) fn window<'a>(
    spec: &'a Spectrum,
    sorted: bool,
    lo: f64,
    hi: f64,
) -> impl Iterator<Item = (f64, f32)> + 'a {
    let n = spec.mz.len().min(spec.intensity.len());
    // the window holds few peaks: after the bisection for its start, its end is found by walking
    let (a, b) = if sorted {
        let a = spec.mz[..n].partition_point(|&m| m < lo);
        let len = spec.mz[a..n].iter().position(|&m| m > hi);
        (a, len.map_or(n, |k| a + k))
    } else {
        (0, n)
    };
    spec.mz[a..b]
        .iter()
        .zip(&spec.intensity[a..b])
        .filter(move |(&m, _)| m >= lo && m <= hi)
        .map(|(&m, &i)| (m, i))
}

/// True when the m/z array can be bisected: ascending (ties allowed) and free of NaN.
pub(super) fn is_ascending(spec: &Spectrum) -> bool {
    spec.mz.windows(2).all(|w| w[0] <= w[1])
}

/// Of the peaks accepted by `ok`, the most intense one; on equal intensity the closest to `theo`, then the first.
pub(super) fn strongest(
    obs: &[(f64, f64)],
    theo: f64,
    ok: impl Fn(f64) -> bool,
) -> Option<(f64, f64)> {
    let mut best: Option<(f64, f64)> = None;
    for &(m, i) in obs.iter().filter(|(m, _)| ok(*m)) {
        let better = match best {
            None => true,
            Some((bm, bi)) => i > bi || (i == bi && (m - theo).abs() < (bm - theo).abs()),
        };
        if better {
            best = Some((m, i));
        }
    }
    best
}
