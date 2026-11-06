//! The bin of an intensity in the log10 histograms of `params` without a logarithm per value: the bin is `floor((log10(v) - lo) / bin)`
//! clamped to the histogram, and its edges are found once by bisection on that same formula (the `libm` log10), so a value is placed by
//! comparing it with the edges. A value within `BAND` (relative) of an edge takes the formula itself: the log10 of `libm` is within a few ulp,
//! far below the band, so every value lands in the bin the formula gives (the same counts, the same bytes, native and `.wasm`).

/// Relative distance from an edge under which the bin is computed with the logarithm.
const BAND: f64 = 1e-9;

/// The bin of `v` (> 0) by the formula: `floor((log10(v) - lo) / bin)` clamped to `0..bins`.
pub(crate) fn by_log(v: f64, lo: f64, bin: f64, bins: usize) -> usize {
    let i = ((libm::log10(v) - lo) / bin).floor();
    (i.max(0.0) as usize).min(bins - 1)
}

/// The edges of a histogram: `edges[k]` is the smallest positive value whose bin by `by_log` is at least `k + 1`.
pub(crate) struct Edges {
    edges: Vec<f64>,
    lo: f64,
    bin: f64,
    bins: usize,
}

impl Edges {
    pub(crate) fn new(lo: f64, bin: f64, bins: usize) -> Edges {
        // fixed size: one edge per bin after the first
        let mut edges = Vec::with_capacity(bins.saturating_sub(1));
        for k in 1..bins {
            // positive doubles are ordered as their bits: bisect on the bits for the first value of bin >= k
            let (mut a, mut b) = (0u64, f64::MAX.to_bits());
            while b - a > 1 {
                let m = a + (b - a) / 2;
                if by_log(f64::from_bits(m), lo, bin, bins) >= k {
                    b = m;
                } else {
                    a = m;
                }
            }
            edges.push(f64::from_bits(b));
        }
        Edges {
            edges,
            lo,
            bin,
            bins,
        }
    }

    /// The bin of `v` (> 0, not NaN): the number of edges at or below it, or the formula next to an edge.
    pub(crate) fn bin(&self, v: f64) -> usize {
        let k = self.edges.partition_point(|&e| e <= v);
        let near = |e: f64| (v - e).abs() <= e * BAND;
        let close =
            (k > 0 && near(self.edges[k - 1])) || self.edges.get(k).is_some_and(|&e| near(e));
        if close || !v.is_finite() {
            return by_log(v, self.lo, self.bin, self.bins);
        }
        k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_value_takes_the_bin_of_the_formula() {
        let e = Edges::new(0.0, 0.1, 100);
        let mut x = 0x2545_f491_4f6c_dd1du64;
        let mut seen = 0;
        for _ in 0..2_000_000 {
            // xorshift over the whole range of positive doubles, and values around the edges
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let v = f64::from_bits(x >> 1);
            if v > 0.0 && !v.is_nan() {
                assert_eq!(e.bin(v), by_log(v, 0.0, 0.1, 100), "{v:e}");
                seen += 1;
            }
            let k = (x % 99) as usize;
            let edge = e.edges[k].to_bits();
            for d in [-3i64, -2, -1, 0, 1, 2, 3, (x % 4096) as i64 - 2048] {
                let w = f64::from_bits(edge.wrapping_add_signed(d));
                assert_eq!(e.bin(w), by_log(w, 0.0, 0.1, 100), "{w:e}");
            }
        }
        assert!(seen > 1_000_000);
        for v in [
            f64::MIN_POSITIVE,
            5e-324,
            1.0,
            10.0,
            1e10,
            3.0e38,
            f64::MAX,
            f64::INFINITY,
        ] {
            assert_eq!(e.bin(v), by_log(v, 0.0, 0.1, 100), "{v:e}");
        }
    }

    #[test]
    fn the_edges_are_the_decimal_steps() {
        let e = Edges::new(0.0, 0.1, 100);
        assert_eq!(e.edges.len(), 99);
        for (k, &v) in e.edges.iter().enumerate() {
            let want = libm::pow(10.0, 0.1 * (k + 1) as f64);
            assert!((v / want - 1.0).abs() < 1e-12, "{k}: {v} {want}");
        }
    }
}
