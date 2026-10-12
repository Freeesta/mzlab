//! Candidates (local maxima of the smoothed signal above the background), their regions and the merging of adjacent ones.

use super::{Buf, Mode, Params};

/// A peak before the threshold: its apex and its region (indices of the grid, both included); `members` = candidates merged into it.
#[derive(Clone, Debug)]
pub(super) struct Group {
    pub apex: usize,
    pub lo: usize,
    pub hi: usize,
    pub members: usize,
}

/// Local maxima of the smoothed signal: s[j] >= s[j-1], s[j] > s[j+1], s[j] > background (the last point of a plateau).
fn candidates(buf: &Buf) -> Vec<usize> {
    let n = buf.s.len();
    (0..n)
        .filter(|&j| {
            let l = if j > 0 {
                buf.s[j - 1]
            } else {
                f64::NEG_INFINITY
            };
            let r = if j + 1 < n {
                buf.s[j + 1]
            } else {
                f64::NEG_INFINITY
            };
            buf.s[j] >= l && buf.s[j] > r && buf.s[j] > buf.b[j] && buf.s[j] > 0.0
        })
        .collect()
}

/// Region of an apex: down each side while the smoothed signal keeps falling and stays above the background, not through two raw zeros in a row,
/// at most 1.5 w from the apex.
fn bounds(buf: &Buf, apex: usize, wb: usize) -> (usize, usize) {
    let ext = (1.5 * wb as f64).ceil() as usize;
    let n = buf.s.len();
    let mut lo = apex;
    while lo > 0
        && apex - lo < ext
        && buf.s[lo - 1] <= buf.s[lo]
        && buf.s[lo - 1] > buf.b[lo - 1]
        && !(buf.v[lo - 1] == 0.0 && buf.v[lo] == 0.0 && lo != apex)
    {
        lo -= 1;
    }
    let mut hi = apex;
    while hi + 1 < n
        && hi - apex < ext
        && buf.s[hi + 1] <= buf.s[hi]
        && buf.s[hi + 1] > buf.b[hi + 1]
        && !(buf.v[hi + 1] == 0.0 && buf.v[hi] == 0.0 && hi != apex)
    {
        hi += 1;
    }
    (lo, hi)
}

/// Index of the first minimum of the smoothed signal in [a, b].
fn valley(buf: &Buf, a: usize, b: usize) -> usize {
    let mut v = a;
    for j in a..=b {
        if buf.s[j] < buf.s[v] {
            v = j;
        }
    }
    v
}

/// Sum of the raw intensities within `h` bins of `c`.
fn raw_near(buf: &Buf, c: usize, h: usize) -> f64 {
    let (lo, hi) = (c.saturating_sub(h), (c + h).min(buf.v.len() - 1));
    buf.v[lo..=hi].iter().sum()
}

/// True when the valley between two apexes is not significant: counts (a - v) / sqrt(a + v) < k on 0.5 w around the lower apex and around the valley;
/// continuous: the lower apex stands less than k sigma over the valley.
fn insignificant(buf: &Buf, p: &Params, sigma: f64, a: usize, c: usize) -> bool {
    let v = valley(buf, a, c);
    let lower_apex = if buf.s[c] < buf.s[a] { c } else { a };
    match p.mode {
        Mode::Counts => {
            let h = ((0.25 * p.wb() as f64).ceil() as usize).max(1);
            let (ca, cv) = (raw_near(buf, lower_apex, h), raw_near(buf, v, h));
            ca + cv <= 0.0 || (ca - cv) / (ca + cv).sqrt() < p.k
        }
        Mode::Continuous => buf.s[lower_apex] - buf.s[v] < p.k * sigma,
    }
}

/// The peaks of a scan: candidates merged into the highest apex when they are closer than 0.5 w or the valley between them is not significant.
pub(super) fn groups(buf: &Buf, p: &Params, sigma: f64) -> Vec<Group> {
    let wb = p.wb();
    let mut out: Vec<Group> = Vec::new();
    for c in candidates(buf) {
        let (lo, hi) = bounds(buf, c, wb);
        let g = Group {
            apex: c,
            lo,
            hi,
            members: 1,
        };
        let merge = match out.last() {
            Some(last) => {
                ((c - last.apex) as f64) < 0.5 * wb as f64
                    || insignificant(buf, p, sigma, last.apex, c)
            }
            None => false,
        };
        if merge {
            let last = out.last_mut().unwrap();
            if buf.s[c] > buf.s[last.apex] {
                last.apex = c;
            }
            last.lo = last.lo.min(lo);
            last.hi = last.hi.max(hi);
            last.members += 1;
        } else {
            out.push(g);
        }
    }
    for i in 1..out.len() {
        let prev = out[i - 1].hi;
        if out[i].lo <= prev {
            out[i].lo = (prev + 1).min(out[i].apex);
        }
    }
    out
}
