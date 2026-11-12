//! Parameters of a file estimated from a few scans (at most 64 equally spaced ones, read and dropped): the mode (counts or continuous), the unit of one
//! count, the step of the grid and the width of a peak (median FWHM of isolated peaks with S/N >= 20, looked for at the fallback width and, for continuous data, at widths of 8-64 points; below 8 peaks the fallback width is used and `peaks` is 0).

use super::{smooth, Buf, Mode, Params};

/// The parameters and what they rest on.
#[derive(Clone, Debug)]
pub struct Estimate {
    pub params: Params,
    /// The points are on a regular grid.
    pub regular: bool,
    /// Isolated peaks that gave the width (0 = the fallback width was used).
    pub peaks: usize,
    /// Centroids of a conversion flagged `MERGED` and `SATURATED` (0 for an estimate alone).
    pub merged: usize,
    pub saturated: usize,
}

/// Value of the highest frequency among `v` rounded to 6 significant digits (the smallest on a tie).
fn mode_of(v: &mut [f64]) -> f64 {
    crate::sortc::by(v, f64::total_cmp);
    let key = |x: f64| format!("{x:.5e}");
    let (mut best, mut best_n, mut i) = (v[0], 0, 0);
    while i < v.len() {
        let mut j = i;
        while j < v.len() && key(v[j]) == key(v[i]) {
            j += 1;
        }
        if j - i > best_n {
            best_n = j - i;
            best = v[i];
        }
        i = j;
    }
    best
}

/// Unit and mode: u = most frequent smallest positive intensity of a scan; counts when 95 % of the points are whole multiples of u (within 0.01).
fn unit_and_mode(scans: &[(&[f64], &[f64])]) -> (f64, Mode) {
    let mut mins: Vec<f64> = scans
        .iter()
        .filter_map(|(_, y)| {
            y.iter()
                .copied()
                .filter(|&v| v > 0.0)
                .fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.min(v))))
        })
        .collect();
    if mins.is_empty() {
        return (1.0, Mode::Continuous);
    }
    let u = mode_of(&mut mins);
    let (mut all, mut ok) = (0usize, 0usize);
    for (_, y) in scans {
        for &v in y.iter().filter(|&&v| v > 0.0) {
            all += 1;
            let q = v / u;
            if q.round() >= 1.0 && (q - q.round()).abs() <= 0.01 {
                ok += 1;
            }
        }
    }
    (
        u,
        if all > 0 && ok as f64 >= 0.95 * all as f64 {
            Mode::Counts
        } else {
            Mode::Continuous
        },
    )
}

/// Step: the smallest distance between two points; regular when every distance is a whole multiple of it (within 1 %). Irregular: the median distance.
fn step_and_grid(scans: &[(&[f64], &[f64])]) -> (f64, bool) {
    let mut d: Vec<f64> = scans
        .iter()
        .flat_map(|(mz, _)| mz.windows(2).map(|w| w[1] - w[0]))
        .filter(|&x| x > 0.0)
        .collect();
    if d.is_empty() {
        return (1.0, false);
    }
    crate::sortc::by(&mut d, f64::total_cmp);
    let min = d[0];
    let regular = d.iter().all(|&x| {
        let q = x / min;
        (q - q.round()).abs() <= 0.01 * q.round().max(1.0)
    });
    (if regular { min } else { d[(d.len() - 1) / 2] }, regular)
}

/// FWHM of the isolated peaks of a smoothed scan, by linear interpolation at half height.
fn fwhms(buf: &Buf, p: &Params, sigma: f64, out: &mut Vec<(f64, f64)>) {
    let n = buf.s.len();
    let wb = p.wb();
    let u = if p.mode == Mode::Counts { p.unit } else { 1.0 };
    for j in 1..n.saturating_sub(1) {
        let s = buf.s[j];
        if !(s >= buf.s[j - 1] && s > buf.s[j + 1]) {
            continue;
        }
        let h = s - buf.b[j];
        let sn = if p.mode == Mode::Counts {
            h / u / (s / u).max(1.0).sqrt()
        } else {
            h / sigma
        };
        if sn < 20.0 {
            continue;
        }
        let (lo, hi) = (j.saturating_sub(3 * wb), (j + 3 * wb).min(n - 1));
        let isolated = (lo..=hi).all(|i| {
            i == j
                || !(buf.s[i] >= buf.s[i.saturating_sub(1)] && buf.s[i] > buf.s[(i + 1).min(n - 1)])
                || buf.s[i] <= 0.2 * s
        });
        if !isolated {
            continue;
        }
        let half = buf.b[j] + 0.5 * h;
        let (mut l, mut r) = (j, j);
        while l > 0 && buf.s[l] > half {
            l -= 1;
        }
        while r + 1 < n && buf.s[r] > half {
            r += 1;
        }
        if buf.s[l] > half || buf.s[r] > half {
            continue;
        }
        let xl =
            buf.x[l] + (half - buf.s[l]) / (buf.s[l + 1] - buf.s[l]) * (buf.x[l + 1] - buf.x[l]);
        let xr = buf.x[r - 1]
            + (buf.s[r - 1] - half) / (buf.s[r - 1] - buf.s[r]) * (buf.x[r] - buf.x[r - 1]);
        out.push((buf.x[j], xr - xl));
    }
}

/// Median of the widths of `(m/z, width)` pairs.
fn median_w(pairs: &[(f64, f64)]) -> f64 {
    median(&mut pairs.iter().map(|&(_, w)| w).collect::<Vec<_>>())
}

/// `(a, b)` of FWHM = a * m^b by least squares on the logarithms of the isolated peaks, when they span at least 1.3 x in m/z and give
/// 0 <= b <= 2 (a quadrupole is flat, a TOF ~1, an Orbitrap ~1.5); `None` keeps one width for every m/z.
fn width_model(pairs: &[(f64, f64)]) -> Option<(f64, f64)> {
    let pts: Vec<(f64, f64)> = pairs
        .iter()
        .filter(|&&(m, w)| m > 0.0 && w > 0.0)
        .map(|&(m, w)| (libm::log(m), libm::log(w)))
        .collect();
    let n = pts.len() as f64;
    let (lo, hi) = pts
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), &(x, _)| (l.min(x), h.max(x)));
    if pts.len() < 8 || hi - lo < libm::log(1.3) {
        return None;
    }
    let (mx, my) = (
        pts.iter().map(|p| p.0).sum::<f64>() / n,
        pts.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let sxx: f64 = pts.iter().map(|&(x, _)| (x - mx) * (x - mx)).sum();
    let sxy: f64 = pts.iter().map(|&(x, y)| (x - mx) * (y - my)).sum();
    let b = sxy / sxx;
    (0.0..=2.0)
        .contains(&b)
        .then(|| (libm::exp(my - b * mx), b))
}

fn median(v: &mut [f64]) -> f64 {
    crate::sortc::by(v, f64::total_cmp);
    v[(v.len() - 1) / 2]
}

/// FWHMs of the isolated peaks of `scans` measured with the background and the isolation test of the width `p.w`.
fn widths_at(scans: &[(&[f64], &[f64])], p: &Params) -> Vec<(f64, f64)> {
    let mut buf = Buf::default();
    let mut widths = Vec::new();
    for (mz, y) in scans {
        super::grid::fill(mz, y, p, &mut buf);
        if buf.v.len() < 5 {
            continue;
        }
        smooth::smooth(&mut buf, 5);
        smooth::background(&mut buf, p.wb(), p.mode == Mode::Counts);
        let sigma = if p.mode == Mode::Continuous {
            smooth::noise(&mut buf, p.wb())
        } else {
            1.0
        };
        fwhms(&buf, p, sigma, &mut widths);
    }
    widths
}

/// Estimates the parameters; `fallback_w` is used when fewer than 8 isolated peaks are found (continuous data are searched again at widths of 8, 24 and 64 points first) (0.7 for a unit-resolution quadrupole, 0.25 for a linear ion trap).
pub fn estimate(scans: &[(&[f64], &[f64])], fallback_w: f64) -> Estimate {
    let (unit, mode) = unit_and_mode(scans);
    let (step, regular) = step_and_grid(scans);
    let vmax = scans
        .iter()
        .flat_map(|(_, y)| y.iter().copied())
        .fold(0.0, f64::max);
    let mut p = Params {
        mode,
        unit,
        step,
        w: fallback_w.max(3.0 * step),
        k: 3.0,
        vmax,
        area: true,
        wm: None,
    };
    let mut widths = widths_at(scans, &p);
    // Continuous data whose peaks are much narrower than the fallback (high-resolution profile): at the fallback width
    // nothing looks isolated and the background swallows the peaks, so the width is looked for again on the scale of
    // the point spacing, then measured once more at the width found.
    if widths.len() < 8 && mode == Mode::Continuous {
        for points in [8.0, 24.0, 64.0] {
            let q = Params {
                w: points * step,
                ..p.clone()
            };
            if q.w >= p.w {
                break;
            }
            let found = widths_at(scans, &q);
            if found.len() >= 8 {
                let again = Params {
                    w: median_w(&found).max(3.0 * step),
                    ..p.clone()
                };
                let more = widths_at(scans, &again);
                widths = if more.len() >= 8 { more } else { found };
                break;
            }
        }
    }
    let peaks = if widths.len() >= 8 { widths.len() } else { 0 };
    if peaks > 0 {
        p.w = median_w(&widths).max(3.0 * step);
    }
    p.area = !p.hr();
    if peaks > 0 && p.hr() {
        p.wm = width_model(&widths);
    }
    Estimate {
        params: p,
        regular,
        peaks,
        merged: 0,
        saturated: 0,
    }
}
