//! `raw scan` (`/api/rawscan`, `docs/agenti/raw-access.md`): what is in the mean spectrum of a time window that is not in the background beside it?
//! Per bin the mean of the window's scans minus the mean of the scans on its two sides, with the ratio, the net ion counts, a z and a verdict. The CLI,
//! the app (`rawinfo::handle`) and the site (the lazy `.wasm` `rawscan`) answer through here, so the bytes are one.

use std::collections::BTreeMap;

use crate::channels::Famiglia;
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::json::{round, Out};
use crate::mzcal::Spec;
use crate::rawxic::{z_of, PROBE};
use crate::routes::{Query, Routes};

/// Defaults of the verdict (`--set`): the z of the net counts and the net ions per scan of a `peak`, the one-sided error of a `trace`, the shortest
/// side window (min) and the gap between the window and a side (min).
pub const MIN_Z: f64 = 3.0;
pub const MIN_NET: f64 = 1.0;
pub const ALPHA: f64 = 0.05;
pub const SIDE_MIN: f64 = 0.3;
pub const GAP_MIN: f64 = 0.05;
/// Rows kept by default; 0 keeps every bin.
pub const TOP: usize = 20;

/// The width of a bin: a whole mass or a tenth of a dalton, centred on its value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bin {
    Nominal,
    Tenth,
}

impl Bin {
    fn name(self) -> &'static str {
        match self {
            Bin::Nominal => "nominal",
            Bin::Tenth => "0.1",
        }
    }
    fn of(self, mz: f64) -> i64 {
        match self {
            Bin::Nominal => mz.round() as i64,
            Bin::Tenth => (mz * 10.0).round() as i64,
        }
    }
    fn centre(self, k: i64) -> f64 {
        match self {
            Bin::Nominal => k as f64,
            Bin::Tenth => round(k as f64 / 10.0, 1),
        }
    }
}

/// The thresholds of the verdict and the geometry of the sides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thresholds {
    pub min_z: f64,
    pub min_net: f64,
    pub alpha: f64,
    pub side_min: f64,
    pub gap_min: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            min_z: MIN_Z,
            min_net: MIN_NET,
            alpha: ALPHA,
            side_min: SIDE_MIN,
            gap_min: GAP_MIN,
        }
    }
}

fn bad(name: &str, v: &str) -> Error {
    Error::new("err.raw.scan.param")
        .with("name", name.to_string())
        .with("value", v.to_string())
}

/// The question of `raw scan` as the route reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub rt: (f64, f64),
    pub level: u8,
    pub channel: Option<String>,
    pub bin: Bin,
    pub range: Option<(f64, f64)>,
    pub top: usize,
    pub offset: Option<Spec>,
    pub t: Thresholds,
}

/// `a,b` as two finite numbers, `a < b`.
fn pair(k: &str, v: &str) -> Result<(f64, f64)> {
    let w: Vec<f64> = v
        .split(',')
        .filter_map(|t| t.trim().parse::<f64>().ok().filter(|x| x.is_finite()))
        .collect();
    match w[..] {
        [a, b] if a < b && v.split(',').count() == 2 => Ok((a, b)),
        _ => Err(bad(k, v)),
    }
}

impl Ask {
    /// `rt=A,B[&bin=nominal|0.1][&mz_range=LO,HI][&top=N][&ms_level=N][&channel=ID][&mz_offset=TEXT][&min_z=..&min_net=..&alpha=..&side_min=..&gap_min=..]`;
    /// a value out of its range is refused. `top=0` keeps every bin.
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
        let range = |k: &str, d: f64, lo: f64, hi: f64| match num(k)? {
            None => Ok(d),
            Some(x) if x >= lo && x <= hi => Ok(x),
            Some(x) => Err(bad(k, &x.to_string())),
        };
        let whole = |k: &str, d: usize, lo: f64, hi: f64| match num(k)? {
            None => Ok(d),
            Some(x) if x >= lo && x <= hi && x.fract() == 0.0 => Ok(x as usize),
            Some(x) => Err(bad(k, &x.to_string())),
        };
        let rt = pair("rt", q.some("rt").ok_or_else(|| bad("rt", ""))?)?;
        let bin = match q.some("bin") {
            None | Some("nominal") => Bin::Nominal,
            Some("0.1") => Bin::Tenth,
            Some(v) => return Err(bad("bin", v)),
        };
        let d = Thresholds::default();
        Ok(Ask {
            rt,
            level: whole("ms_level", 1, 1.0, 10.0)? as u8,
            channel: q.some("channel").map(str::to_string),
            bin,
            range: q
                .some("mz_range")
                .map(|v| pair("mz_range", v))
                .transpose()?,
            top: whole("top", TOP, 0.0, 1e6)?,
            offset: q.some("mz_offset").map(Spec::parse).transpose()?,
            t: Thresholds {
                min_z: range("min_z", d.min_z, 0.0, 1e6)?,
                min_net: range("min_net", d.min_net, 0.0, 1e6)?,
                alpha: range("alpha", d.alpha, 1e-6, 0.5)?,
                side_min: range("side_min", d.side_min, 0.0, 1e4)?,
                gap_min: range("gap_min", d.gap_min, 0.0, 1e4)?,
            },
        })
    }
}

/// What one sample answered: the members (no braces), the `summary` object and the rows, each a JSON object.
pub struct ScanOut {
    pub members: String,
    pub summary: String,
    pub rows: Vec<String>,
    /// Bins with a signal in the window or on its sides, before `top` cut the rows.
    pub n_bins: usize,
}

/// The sums of one bin over the scans of the window and of the sides (intensity units).
#[derive(Clone, Copy, Default)]
struct Sums {
    window: f64,
    side: f64,
}

/// One bin judged: the means, the net, and with an ion quantum the net counts, their z and the verdict.
struct Cell {
    bin: i64,
    mean: f64,
    background: f64,
    net: f64,
    counts_net: Option<f64>,
    z: Option<f64>,
    verdict: Option<&'static str>,
}

fn judge(k: i64, s: Sums, n_p: f64, n_b: f64, q: Option<f64>, t: &Thresholds) -> Cell {
    let mean = s.window / n_p;
    let background = if n_b > 0.0 { s.side / n_b } else { 0.0 };
    let mut c = Cell {
        bin: k,
        mean,
        background,
        net: mean - background,
        counts_net: None,
        z: None,
        verdict: None,
    };
    if let Some(q) = q {
        // the counts of `raw xic`: gross over the window, the side mean per scan times the window's scans, variance with the one-ion floor
        let gross = s.window / q;
        let lambda = if n_b > 0.0 { s.side / q / n_b } else { 0.0 };
        let net = gross - lambda * n_p;
        let var = gross
            + if n_b > 0.0 {
                lambda * n_p * n_p / n_b
            } else {
                0.0
            };
        let z = net / var.max(1.0).sqrt();
        c.counts_net = Some(net);
        c.z = Some(z);
        c.verdict = Some(if z >= t.min_z && net / n_p >= t.min_net {
            "peak"
        } else if net > 0.0 && z >= z_of(t.alpha) {
            "trace"
        } else {
            "nothing"
        });
    }
    c
}

fn row_json(c: &Cell, bin: Bin) -> String {
    let mut o = Out::default();
    let r3 = |v: f64| round(v, 3);
    o.raw("{\"bin\":")
        .int(c.bin)
        .raw(",\"mz_center\":")
        .num(bin.centre(c.bin));
    o.raw(",\"mean\":")
        .num(r3(c.mean))
        .raw(",\"background\":")
        .num(r3(c.background))
        .raw(",\"net\":")
        .num(r3(c.net));
    let zero = c.background == 0.0;
    o.raw(",\"ratio\":")
        .opt((!zero).then(|| r3(c.mean / c.background)));
    o.raw(",\"background_zero\":")
        .raw(if zero { "true" } else { "false" });
    o.raw(",\"counts_net\":").opt(c.counts_net.map(r3));
    o.raw(",\"z_counts\":").opt(c.z.map(r3));
    o.raw(",\"verdict\":");
    match c.verdict {
        Some(v) => o.str(v),
        None => o.raw("null"),
    };
    o.raw("}");
    o.0
}

fn mean_of(v: &[f64]) -> Option<f64> {
    (!v.is_empty()).then(|| round(v.iter().sum::<f64>() / v.len() as f64, 1))
}

impl<R: ReadAt> Routes<R> {
    /// `/api/rawscan?rt=A,B[&bin=..][&mz_range=..][&top=..][&ms_level=..][&channel=..][&mz_offset=..][&min_z=..]...`: the members of `raw-scan/1` for this
    /// sample, its `summary` and its rows.
    pub fn raw_scan_route(&mut self, q: &Query) -> Result<Option<String>> {
        let o = self.raw_scan(&Ask::of(q)?)?;
        Ok(Some(format!(
            "{{{},\"summary\":{},\"rows\":[{}]}}",
            o.members,
            o.summary,
            o.rows.join(",")
        )))
    }

    /// The level and the scan mask of the channel asked (`None` for every scan of the level).
    fn scan_channel(&mut self, ask: &Ask) -> Result<(u8, Option<Vec<bool>>)> {
        let Some(id) = &ask.channel else {
            return Ok((ask.level, None));
        };
        let c = self.canali();
        let i = c
            .find(id)
            .ok_or_else(|| Error::new("err.canali.unknown").with("id", id.as_str()))?;
        if !matches!(c.v[i].famiglia, Famiglia::Ms | Famiglia::Dd) {
            return Err(Error::new("err.raw.scan.channel").with("id", id.as_str()));
        }
        let mut mask = vec![false; self.idx.scans.len()];
        for k in c.scansioni(i) {
            mask[k as usize] = true;
        }
        Ok((c.v[i].livello, Some(mask)))
    }

    /// The rows of `ask` on this sample (its polarity `pol`): bins by net intensity, highest first, at most `top` of them.
    pub fn raw_scan(&mut self, ask: &Ask) -> Result<ScanOut> {
        let off = ask
            .offset
            .as_ref()
            .map(|s| s.resolve(&mut self.idx, self.pol))
            .transpose()?;
        let off_da = off.as_ref().map_or(0.0, |o| o.da);
        let (level, mask) = self.scan_channel(ask)?;
        let probe = self.probe(level, PROBE)?;
        let q = crate::rawinfo::quantum_of(Some(&probe))
            .filter(|q| q.accepted)
            .map(|q| q.q);
        let mut sums: BTreeMap<i64, Sums> = BTreeMap::new();
        let mut sc = Scans::default();
        let mut n_level = 0;
        if let Some(si) = self.idx.section_of(level, self.pol) {
            let arr = self.idx.arrays(si)?.clone();
            n_level = arr.rt.len();
            sc = Scans::of(&arr, mask.as_deref(), ask);
            if sc.n_win > 0 {
                let (lo, hi) = ask.range.unwrap_or((f64::MIN, f64::MAX));
                self.idx.each_peak_for(si, &sc.lut, |c, mz, y| {
                    let m = mz - off_da;
                    if m < lo || m > hi {
                        return;
                    }
                    let e = sums.entry(ask.bin.of(m)).or_default();
                    if c == 0 {
                        e.window += y;
                    } else {
                        e.side += y;
                    }
                })?;
            }
        }
        let (n_p, n_left, n_right) = (sc.n_win, sc.n_left, sc.n_right);
        let (tic_win, tic_side) = (sc.tic_win, sc.tic_side);
        let n_b = (n_left + n_right) as f64;
        let mut cells: Vec<Cell> = sums
            .into_iter()
            .filter(|(_, s)| s.window > 0.0 || s.side > 0.0)
            .map(|(k, s)| judge(k, s, n_p as f64, n_b, q, &ask.t))
            .collect();
        // highest net first; equal nets in mass order, so the answer is one
        crate::sortc::slice_by(&mut cells, |x, y| {
            y.net.total_cmp(&x.net).then(x.bin.cmp(&y.bin))
        });
        let count = |w: &str| cells.iter().filter(|c| c.verdict == Some(w)).count();
        let (peaks, traces) = (count("peak"), count("trace"));
        let n_bins = cells.len();
        let keep = if ask.top == 0 {
            n_bins
        } else {
            ask.top.min(n_bins)
        };
        let rows = cells[..keep].iter().map(|c| row_json(c, ask.bin)).collect();
        Ok(ScanOut {
            members: members_json(ask, level, n_level, &probe, off.as_ref(), q.is_some()),
            summary: summary_json(
                [n_p, n_left, n_right],
                [mean_of(&tic_win), mean_of(&tic_side)],
                [n_bins, peaks, traces],
            ),
            rows,
            n_bins,
        })
    }
}

/// The scans of one section sorted into the window (class 0), the sides (class 1) and the rest (not read), with their TICs.
#[derive(Default)]
struct Scans {
    lut: Vec<u32>,
    n_win: usize,
    n_left: usize,
    n_right: usize,
    tic_win: Vec<f64>,
    tic_side: Vec<f64>,
}

impl Scans {
    /// The sides are as wide as the window, at least `side_min`, and end `gap_min` away from it (as in `raw xic`).
    fn of(arr: &crate::idxread::ScanArrays, mask: Option<&[bool]>, ask: &Ask) -> Scans {
        let (a, b) = ask.rt;
        let w = (b - a).max(ask.t.side_min);
        let g = ask.t.gap_min;
        let mut s = Scans {
            lut: vec![u32::MAX; arr.rt.len()],
            ..Default::default()
        };
        for (i, &t) in arr.rt.iter().enumerate() {
            let kept = mask.is_none_or(|m| {
                arr.scan_ids
                    .get(i)
                    .and_then(|&k| m.get(k as usize))
                    .copied()
                    .unwrap_or(false)
            });
            if !kept {
                continue;
            }
            let y = arr.sumint.get(i).copied().unwrap_or(0.0);
            if t >= a && t <= b {
                (s.lut[i], s.n_win) = (0, s.n_win + 1);
                s.tic_win.push(y);
            } else if t >= a - w && t <= a - g {
                (s.lut[i], s.n_left) = (1, s.n_left + 1);
                s.tic_side.push(y);
            } else if t >= b + g && t <= b + w {
                (s.lut[i], s.n_right) = (1, s.n_right + 1);
                s.tic_side.push(y);
            }
        }
        s
    }
}

fn summary_json(n: [usize; 3], tic: [Option<f64>; 2], bins: [usize; 3]) -> String {
    let mut o = Out::default();
    o.raw("{\"n_scans\":")
        .int(n[0] as i64)
        .raw(",\"n_side_scans\":")
        .int((n[1] + n[2]) as i64);
    o.raw(",\"n_left_scans\":")
        .int(n[1] as i64)
        .raw(",\"n_right_scans\":")
        .int(n[2] as i64);
    o.raw(",\"tic_mean\":")
        .opt(tic[0])
        .raw(",\"tic_side\":")
        .opt(tic[1]);
    o.raw(",\"n_bins\":")
        .int(bins[0] as i64)
        .raw(",\"bins_with_peak\":")
        .int(bins[1] as i64)
        .raw(",\"bins_with_trace\":")
        .int(bins[2] as i64)
        .raw("}");
    o.0
}

/// The members of one sample: the level and its scans, the ion quantum, the m/z offset with its evidence and the thresholds in force.
fn members_json(
    ask: &Ask,
    level: u8,
    n: usize,
    probe: &crate::rawinfo::Probe,
    off: Option<&crate::mzcal::Offset>,
    counts: bool,
) -> String {
    let mut o = Out::default();
    o.raw("\"ms_level\":")
        .int(i64::from(level))
        .raw(",\"n_level_scans\":")
        .int(n as i64);
    o.raw(",\"channel\":");
    match &ask.channel {
        Some(c) => o.str(c),
        None => o.raw("null"),
    };
    o.raw(",\"bin\":").str(ask.bin.name());
    o.raw(",\"mz_range\":");
    match ask.range {
        Some((lo, hi)) => o.floats(&[lo, hi]),
        None => o.raw("null"),
    };
    o.raw(",\"ion_quantum\":")
        .raw(&crate::rawinfo::quantum_json(Some(probe)))
        .raw(",\"counts\":")
        .raw(if counts { "true" } else { "false" });
    o.raw(",\"mz_offset\":");
    match (&ask.offset, off) {
        (Some(s), Some(of)) => o
            .raw("{\"spec\":")
            .str(&s.to_string())
            .raw(",\"da\":")
            .num(of.da)
            .raw(",\"evidence\":")
            .raw(&of.evidence_json())
            .raw("}"),
        _ => o.raw("null"),
    };
    let t = &ask.t;
    o.raw(",\"thresholds\":{\"min_z\":")
        .num(t.min_z)
        .raw(",\"min_net\":")
        .num(t.min_net)
        .raw(",\"alpha\":")
        .num(t.alpha)
        .raw(",\"z_alpha\":")
        .num(round(z_of(t.alpha), 4));
    o.raw(",\"side_min\":")
        .num(t.side_min)
        .raw(",\"gap_min\":")
        .num(t.gap_min)
        .raw(",\"top\":")
        .int(ask.top as i64)
        .raw("}");
    o.0
}

#[cfg(test)]
#[path = "rawscan_tests.rs"]
mod tests;
