use super::*;

/// `limit_span` as it was before the pieces were pushed straight onto the stack.
fn limit_span_before(x: &[f64], runs: Vec<(usize, usize)>, max_span: f64) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(runs.len());
    let mut stack: Vec<(usize, usize, u32)> =
        runs.into_iter().rev().map(|(a, b)| (a, b, 0)).collect();
    while let Some((a, b, depth)) = stack.pop() {
        if depth >= 32 || b - a < 2 || x[b - 1] - x[a] <= max_span {
            out.push((a, b));
            continue;
        }
        let gap = |i: usize| x[i] - x[i - 1];
        let top = (a + 1..b).map(gap).fold(f64::MIN, f64::max);
        if top <= 0.0 {
            out.push((a, b));
            continue;
        }
        let mut cuts: Vec<usize> = (a + 1..b).filter(|&i| gap(i) == top).collect();
        cuts.push(b);
        let mut from = a;
        let mut pieces = Vec::with_capacity(cuts.len());
        for c in cuts {
            pieces.push((from, c, depth + 1));
            from = c;
        }
        stack.extend(pieces.into_iter().rev());
    }
    out
}

#[test]
fn limit_span_cuts_where_it_did_with_ties_and_several_runs() {
    let mut seed = 5u64;
    let mut r = |n: u64| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) % n
    };
    for case in 0..3000 {
        let n = r(40) as usize;
        // few distinct gaps, so that the largest one is often tied
        let mut x = Vec::with_capacity(n);
        let mut at = 0.0;
        for _ in 0..n {
            at += [0.0, 1.0, 1.0, 2.0, 2.0, 5.0][r(6) as usize];
            x.push(at);
        }
        let cut = if n > 0 { r(n as u64 + 1) as usize } else { 0 };
        let runs = if case % 2 == 0 {
            vec![(0, n)]
        } else {
            vec![(0, cut), (cut, n)]
        };
        let span = [0.5, 3.0, 6.0, 20.0][case % 4];
        assert_eq!(
            limit_span_before(&x, runs.clone(), span),
            limit_span(&x, runs, span),
            "case {case}"
        );
    }
}
