//! Optional figure columns of `manint::table_text` (`columns` of the windows document): `sn` (S/N against the noise zone the user picked), `fig`
//! (width at half height, plates, USP tailing) and `quality` (Gaussian similarity, asymmetry at 10 %, noise score). Port of `chromPeak` / `noiseSD` of
//! the page's former dialog, so the numbers of the one table equal it; the figures are measured on the window's own baseline.

use super::*;
use crate::peakq;
use crate::peakq_lr::noise_sd;
use crate::peaks::noise_level;

/// The column groups asked for.
#[derive(Clone, Copy, Default)]
pub(super) struct Cols {
    pub sn: bool,
    pub fig: bool,
    pub quality: bool,
}

/// The header cells of each group, in the order the groups are written.
const SN_HEAD: &str = ",sn_zone";
const FIG_HEAD: &str = ",w_half_min,plates,tailing";
const QUALITY_HEAD: &str = ",gaussian,asymmetry,noise_score";

/// The windows alone (an array), or `{"windows": [...], "columns": ["sn", "fig", "quality"]}` to add the optional figure columns.
pub(super) fn split(doc: &Value) -> Result<(&Value, Cols)> {
    match doc {
        Value::Obj(_) => Ok((
            doc.get("windows").unwrap_or(&Value::Null),
            Cols::of(doc.get("columns"))?,
        )),
        _ => Ok((doc, Cols::default())),
    }
}

impl Cols {
    /// The groups named in `columns` (a JSON array of `sn`, `fig`, `quality`); an unknown name is an error, never ignored.
    pub(super) fn of(v: Option<&Value>) -> Result<Cols> {
        let mut c = Cols::default();
        for n in v.map_or(&[][..], Value::arr) {
            match n.str() {
                Some("sn") => c.sn = true,
                Some("fig") => c.fig = true,
                Some("quality") => c.quality = true,
                other => {
                    return Err(Error::new("err.manint.columns").with("column", other.unwrap_or("")))
                }
            }
        }
        Ok(c)
    }
    /// How many cells the groups add to a row.
    pub(super) fn count(&self) -> usize {
        3 * usize::from(self.quality) + 3 * usize::from(self.fig) + usize::from(self.sn)
    }
    pub(super) fn any(&self) -> bool {
        self.count() > 0
    }
    /// The header cells to append (with the leading commas).
    pub(super) fn head(&self) -> String {
        let mut s = String::new();
        if self.sn {
            s += SN_HEAD;
        }
        if self.fig {
            s += FIG_HEAD;
        }
        if self.quality {
            s += QUALITY_HEAD;
        }
        s
    }
    /// The cells of one row (with the leading commas): empty where the figure cannot be measured.
    pub(super) fn cells(&self, t: &Trace, r: &Row) -> String {
        let mut s = String::new();
        let num = |v: Option<f64>, d: usize| {
            v.filter(|x| x.is_finite())
                .map_or(String::new(), |x| format!("{x:.d$}"))
        };
        if self.sn {
            let sd = r
                .zone
                .and_then(|z| noise_sd(&t.t, &t.y, z[0], z[1]))
                .filter(|&v| v > 0.0);
            s += &format!(",{}", num(sd.map(|v| r.height / v), 1));
        }
        if self.fig {
            let f = chrom_peak(&t.t, &t.y, r.a, r.b, r.bl);
            s += &format!(
                ",{},{},{}",
                num(f.and_then(|f| f.w50), 3),
                num(f.and_then(|f| f.plates).map(f64::round), 0),
                num(f.and_then(|f| f.tailing), 2)
            );
        }
        if self.quality {
            let (lo, hi) = (near_idx(&t.t, r.a), near_idx(&t.t, r.b));
            let n = t.t.len().min(t.y.len());
            let q = (lo < hi && hi < n)
                .then(|| peakq::quality(&t.t[..n], &t.y[..n], lo, hi, noise_level(&t.y[..n])));
            s += &format!(
                ",{},{},{}",
                num(q.as_ref().map(|q| q.gaussian), 3),
                num(q.as_ref().and_then(|q| q.asymmetry), 2),
                num(q.as_ref().map(|q| q.noise_score), 2)
            );
        }
        s
    }
}

/// What the figure columns need of a window: its baseline as `[t0, y0, t1, y1]` (`None`: `edges`) and the noise stretch it names.
pub(super) fn inputs(base: Baseline, w: &Value) -> (Option<[f64; 4]>, Option<[f64; 2]>) {
    let bl = match base {
        Baseline::Zero => Some([0.0, 0.0, 1.0, 0.0]),
        Baseline::Line { t0, y0, t1, y1 } => Some([t0, y0, t1, y1]),
        Baseline::Edges => None,
    };
    let zone = match nums(w.get("noise"))[..] {
        [a, b] if a.is_finite() && b.is_finite() => Some([a, b]),
        _ => None,
    };
    (bl, zone)
}

/// The figures of one peak.
#[derive(Clone, Copy)]
pub(super) struct Fig {
    pub w50: Option<f64>,
    pub plates: Option<f64>,
    pub tailing: Option<f64>,
}

/// Where the corrected signal `c` (index from `i0`) crosses `level` going away from the apex `ap`: `dir` -1 (front) or +1 (back); `None` if never inside [i0, i1].
fn cross(
    t: &[f64],
    c: &[f64],
    i0: usize,
    ap: usize,
    i1: usize,
    level: f64,
    dir: isize,
) -> Option<f64> {
    let mut j = ap as isize;
    while if dir < 0 {
        j > i0 as isize
    } else {
        j < i1 as isize
    } {
        let k = j + dir;
        let (cj, ck) = (c[j as usize - i0], c[k as usize - i0]);
        if ck <= level {
            let d = cj - ck;
            let f = (cj - level) / if d == 0.0 { 1.0 } else { d };
            return Some(t[j as usize] + (t[k as usize] - t[j as usize]) * f);
        }
        j += dir;
    }
    None
}

/// Width at half height (min), plates N = 5.54 (tR / w) squared and USP tailing at 5 % of the height of the peak between the edges `a` and `b`, over the
/// baseline `bl` = `[t0, y0, t1, y1]` (`None`: the line between the 3-point means of the edges). `None` for a peak of under 3 points or no height.
pub(super) fn chrom_peak(
    t: &[f64],
    y: &[f64],
    a: f64,
    b: f64,
    bl: Option<[f64; 4]>,
) -> Option<Fig> {
    let n = t.len().min(y.len());
    if n < 2 {
        return None;
    }
    let (i0, i1) = (near_idx(&t[..n], a.min(b)), near_idx(&t[..n], a.max(b)));
    if i1 < i0 + 2 {
        return None;
    }
    let on = |p: [f64; 4], x: f64| {
        p[1] + (p[3] - p[1]) * (x - p[0]) / if p[2] == p[0] { 1.0 } else { p[2] - p[0] }
    };
    let (ya, yb) = match bl {
        Some(p) => (on(p, t[i0]), on(p, t[i1])),
        None => (edge3(&y[..n], i0), edge3(&y[..n], i1)),
    };
    let span = if t[i1] == t[i0] { 1.0 } else { t[i1] - t[i0] };
    let c: Vec<f64> = (i0..=i1)
        .map(|j| y[j] - (ya + (yb - ya) * (t[j] - t[i0]) / span))
        .collect();
    let ap = (i0..=i1).fold(i0, |m, j| if c[j - i0] > c[m - i0] { j } else { m });
    let h = c[ap - i0];
    if h.is_nan() || h <= 0.0 {
        return None;
    }
    let at = |f: f64| {
        (
            cross(t, &c, i0, ap, i1, h * f, -1),
            cross(t, &c, i0, ap, i1, h * f, 1),
        )
    };
    let ((l50, r50), (l05, r05), tr) = (at(0.5), at(0.05), t[ap]);
    let w50 = l50.zip(r50).map(|(l, r)| r - l);
    Some(Fig {
        w50,
        plates: w50.filter(|&w| w != 0.0).map(|w| 5.54 * (tr / w).powi(2)),
        tailing: l05
            .zip(r05)
            .filter(|&(l, _)| tr > l)
            .map(|(l, r)| (r - l) / (2.0 * (tr - l))),
    })
}

#[cfg(test)]
#[path = "manint_fig_tests.rs"]
mod tests;
