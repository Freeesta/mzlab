//! `raw xic` (`/api/rawxic`, `docs/agenti/raw-access.md`): at this mass and time, is there a peak? Per m/z the trace of the window goes through the
//! `quant targets` integrator (`tallest`: the same area, height, S/N, shape), then through the ion counts of the file (`ionstat::quantum`, Currie
//! levels, the one-ion floor) to a verdict `peak`, `trace`, `nothing` or `no_data` that carries its rule. The verdict never changes a number.
//! The CLI, the app (`Routes::raw_xic_route`) and the site (the lazy `.wasm` `raw`) answer through here, so the bytes are one.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::json::round;
use crate::mzcal::Spec;
use crate::peakq::quality;
use crate::peakq_lrauto::{lr_auto_integrate, LrAutoParams, Split};
use crate::rawxic_json::{members_json, row_json};
use crate::routes::{Query, Routes};

use crate::rawxic_judge::step_seconds;
pub use crate::rawxic_judge::{judge, z_of, Counts, Judged};

/// Defaults of the verdict thresholds (`--set`): the S/N on the one-ion floor, the run of scans with an ion, the one-sided error of the Currie
/// levels, the shortest side window (min) and the gap between the window and a side (min).
pub const MIN_SN: f64 = 3.0;
pub const MIN_POINTS: usize = 3;
pub const ALPHA: f64 = 0.05;
pub const SIDE_MIN: f64 = 0.3;
pub const GAP_MIN: f64 = 0.05;
/// Savitzky-Golay half window (0 = automatic) and valley depth (%) of the integrator: the defaults of `quant targets`.
pub const SMOOTH_HALF: usize = 0;
pub const VALLEY_PCT: f64 = 10.0;
/// Scans probed for the ion quantum of the level.
pub const PROBE: usize = 64;
/// The estimator behind `sn` (`peakq_lrauto`), named next to the number.
pub const SN_ESTIMATOR: &str = "smoothed height over MAD of first differences (RMS fallback)";
/// Seconds per minute: the integrator works in counts x min, the answers in counts x s.
pub(crate) const SECONDS: f64 = 60.0;

/// The tallest peak of one target in one file (raw trace, rounded like `trace xic`).
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub area: f64,
    pub height: f64,
    pub sn: f64,
    pub rt: f64,
    pub points: usize,
    pub shape: f64,
    pub start_rt: f64,
    pub end_rt: f64,
}

/// The tallest peak of a trace between `x0` and `x1`, how many peaks the window holds, the half window used and whether the apex sits on the first
/// or last scan of the window (the peak may go on outside it). The one function behind the `quant targets` numbers and the `raw xic` ones.
pub fn tallest(
    rt: &[f64],
    y: &[f64],
    x0: f64,
    x1: f64,
    smooth_half: usize,
    valley_pct: f64,
) -> (Option<Hit>, usize, usize, bool) {
    let lp = LrAutoParams {
        half: smooth_half,
        valley: valley_pct / 100.0,
        snr: 0.0,
        min_pts: 2,
        despike: true,
        split: Split::Drop,
    };
    let r = lr_auto_integrate(rt, y, x0, x1, lp);
    let mut best: Option<usize> = None;
    for (k, pk) in r.peaks.iter().enumerate() {
        if best.is_none_or(|b| pk.height > r.peaks[b].height) {
            best = Some(k);
        }
    }
    let Some(b) = best else {
        return (None, 0, r.half, false);
    };
    let pk = r.peaks[b];
    let inside: Vec<usize> = (0..rt.len())
        .filter(|&i| rt[i] >= x0 && rt[i] <= x1)
        .collect();
    let edge = inside.first() == Some(&pk.apex) || inside.last() == Some(&pk.apex);
    let q = quality(rt, y, pk.lo, pk.hi, r.noise);
    let hit = Hit {
        area: round(pk.area * SECONDS, 1),
        height: round(pk.height, 1),
        sn: round(pk.sn, 2),
        rt: rt[pk.apex],
        points: pk.hi - pk.lo + 1,
        shape: round(q.gaussian, 3),
        start_rt: rt[pk.lo],
        end_rt: rt[pk.hi],
    };
    (Some(hit), r.peaks.len(), r.half, edge)
}

/// The thresholds of the verdict and the error of the Currie levels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thresholds {
    pub min_sn: f64,
    pub min_points: usize,
    pub alpha: f64,
    pub side_min: f64,
    pub gap_min: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            min_sn: MIN_SN,
            min_points: MIN_POINTS,
            alpha: ALPHA,
            side_min: SIDE_MIN,
            gap_min: GAP_MIN,
        }
    }
}

fn bad(name: &str, v: &str) -> Error {
    Error::new("err.raw.xic.param")
        .with("name", name.to_string())
        .with("value", v.to_string())
}

/// The question of `raw xic` as the route reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub mz: Vec<f64>,
    pub da: f64,
    pub rt: (f64, f64),
    pub level: u8,
    pub offset: Option<Spec>,
    pub t: Thresholds,
}

impl Ask {
    /// `mz=LIST&rt=A,B[&da=D][&ms_level=N][&mz_offset=TEXT][&min_sn=..&min_points=..&alpha=..&side_min=..&gap_min=..]`; a value out of its range is refused.
    pub fn of(q: &Query) -> Result<Ask> {
        let num = |k: &str| -> Result<Option<f64>> {
            match q.some(k) {
                None => Ok(None),
                Some(v) => v
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|x| x.is_finite())
                    .map(Some)
                    .ok_or_else(|| bad(k, v)),
            }
        };
        let list = |k: &str| -> Result<Vec<f64>> {
            let v = q.some(k).ok_or_else(|| bad(k, ""))?;
            v.split(',')
                .map(|t| {
                    t.trim()
                        .parse::<f64>()
                        .ok()
                        .filter(|x| x.is_finite() && *x > 0.0)
                        .ok_or_else(|| bad(k, v))
                })
                .collect()
        };
        let mz = list("mz")?;
        let rt = q.some("rt").ok_or_else(|| bad("rt", ""))?;
        let w: Vec<f64> = rt
            .split(',')
            .filter_map(|t| t.trim().parse::<f64>().ok().filter(|x| x.is_finite()))
            .collect();
        let rt = match w[..] {
            [a, b] if a < b && rt.split(',').count() == 2 => (a, b),
            _ => return Err(bad("rt", rt)),
        };
        let d = Thresholds::default();
        let range = |k: &str, v: Option<f64>, d: f64, lo: f64, hi: f64| match v {
            None => Ok(d),
            Some(x) if x >= lo && x <= hi => Ok(x),
            Some(x) => Err(bad(k, &x.to_string())),
        };
        let count = |k: &str, v: Option<f64>, d: usize, hi: f64| match v {
            None => Ok(d),
            Some(x) if x >= 1.0 && x <= hi && x.fract() == 0.0 => Ok(x as usize),
            Some(x) => Err(bad(k, &x.to_string())),
        };
        Ok(Ask {
            mz,
            da: range("da", num("da")?, 0.5, 0.001, 5.0)?,
            rt,
            level: count("ms_level", num("ms_level")?, 1, 10.0)? as u8,
            offset: q.some("mz_offset").map(Spec::parse).transpose()?,
            t: Thresholds {
                min_sn: range("min_sn", num("min_sn")?, d.min_sn, 0.0, 1e6)?,
                min_points: count("min_points", num("min_points")?, d.min_points, 1e6)?,
                alpha: range("alpha", num("alpha")?, d.alpha, 1e-6, 0.5)?,
                side_min: range("side_min", num("side_min")?, d.side_min, 0.0, 1e4)?,
                gap_min: range("gap_min", num("gap_min")?, d.gap_min, 0.0, 1e4)?,
            },
        })
    }
}

/// What one sample answered: the members that are not rows (no braces) and the rows, each a JSON object.
pub struct XicOut {
    pub members: String,
    pub rows: Vec<String>,
}

impl<R: ReadAt> Routes<R> {
    /// `/api/rawxic?mz=LIST&rt=A,B[&da=..][&ms_level=..][&mz_offset=..][&min_sn=..]...`: the members of `raw-xic/1` for this sample and its rows.
    pub fn raw_xic_route(&mut self, q: &Query) -> Result<Option<String>> {
        let out = self.raw_xic(&Ask::of(q)?)?;
        Ok(Some(format!(
            "{{{},\"rows\":[{}]}}",
            out.members,
            out.rows.join(",")
        )))
    }

    /// The rows of `ask` on this sample (its polarity `pol`), one per mass in the order given, and the members that say how they were made.
    pub fn raw_xic(&mut self, ask: &Ask) -> Result<XicOut> {
        let keep = self.mz_off;
        let r = self.raw_xic_at(ask);
        self.mz_off = keep;
        r
    }

    fn raw_xic_at(&mut self, ask: &Ask) -> Result<XicOut> {
        let off = ask
            .offset
            .as_ref()
            .map(|s| s.resolve(&mut self.idx, self.pol))
            .transpose()?;
        if let Some(o) = &off {
            self.mz_off = o.da;
        }
        let applied = off.as_ref().is_some_and(|o| o.da != 0.0);
        let probe = self.probe(ask.level, PROBE)?;
        let quant = crate::rawinfo::quantum_of(Some(&probe));
        let q = quant.filter(|q| q.accepted).map(|q| q.q);
        let n = self.xic_len(ask.level);
        let mut rows: Vec<Option<String>> = vec![None; ask.mz.len()];
        let mut dt_s = 0.0;
        // the columns a sweep may hold within half the budget (`sweep_matrix` refuses more): the chunks change nothing in a column
        let per = (crate::budget::TARGET_BYTES / 2 / (8 * n.max(1))).max(1);
        for (k, chunk) in (0..ask.mz.len())
            .collect::<Vec<_>>()
            .chunks(per)
            .enumerate()
        {
            let centres: Vec<f64> = chunk.iter().map(|&i| ask.mz[i]).collect();
            self.forget_sweep();
            let m = if n == 0 {
                None
            } else {
                Some(self.sweep_matrix(&centres, &vec![ask.da; centres.len()], ask.level, None)?)
            };
            let rt: Vec<f64> = m
                .as_ref()
                .map_or(Vec::new(), |m| m.rt.iter().map(|&x| round(x, 4)).collect());
            if k == 0 {
                dt_s = step_seconds(&rt);
            }
            for (c, &i) in chunk.iter().enumerate() {
                let y: Vec<f64> = m.as_ref().map_or(Vec::new(), |m| {
                    m.y[c].iter().map(|&v| round(v, 1)).collect()
                });
                let j = if rt.is_empty() {
                    Judged {
                        verdict: "no_data",
                        rule: format!("no scan of MS level {} in the file", ask.level),
                        hit: None,
                        counts: None,
                        flags: Vec::new(),
                    }
                } else {
                    judge(&rt, &y, ask.rt, q, &ask.t)
                };
                rows[i] = Some(row_json(ask.mz[i], ask.da, ask.rt, &j, dt_s, q, applied));
            }
            drop(m);
            self.forget_sweep();
        }
        Ok(XicOut {
            members: members_json(ask, n, dt_s, &probe, off.as_ref()),
            rows: rows.into_iter().flatten().collect(),
        })
    }
}

#[cfg(test)]
#[path = "rawxic_tests.rs"]
mod tests;
