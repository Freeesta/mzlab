//! `raw xic` (`/api/rawxic`, `docs/agenti/raw-access.md`): at this mass and time, is there a peak? Per m/z the trace of the window goes through the
//! `quant targets` integrator (`tallest`: the same area, height, S/N, shape), then through the ion counts of the file (`ionstat::quantum`, Currie
//! levels, the one-ion floor) to a verdict `peak`, `trace`, `nothing` or `no_data` that carries its rule. The verdict never changes a number.
//! The CLI, the app (`Routes::raw_xic_route`) and the site (the lazy `.wasm` `raw`) answer through here, so the bytes are one.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::ionstat::{rd, QUANTUM_TOL};
use crate::json::{round, Out};
use crate::mzcal::Spec;
use crate::peakq::quality;
use crate::peakq_lrauto::{lr_auto_integrate, LrAutoParams, Split};
use crate::routes::{Query, Routes};

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
const SECONDS: f64 = 60.0;

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

/// The one-sided normal quantile of `1 - alpha` (Acklam's rational approximation, relative error 1e-9).
pub fn z_of(alpha: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    let p = 1.0 - alpha;
    if p > 0.02425 && p < 1.0 - 0.02425 {
        let q = p - 0.5;
        let r = q * q;
        return (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0);
    }
    // the upper tail (alpha < 0.02425): the lower-tail formula of alpha, negated
    let q = (-2.0 * alpha.ln()).sqrt();
    -((((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
        / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0))
}

/// The count statistics of one region (every value in counts, `None` without an ion quantum).
#[derive(Clone, Debug, PartialEq)]
pub struct Counts {
    pub gross: f64,
    pub net: f64,
    pub sigma: f64,
    pub l_c: f64,
    pub l_d: f64,
    pub z: f64,
    /// Mean counts per scan of the sides.
    pub background: f64,
    pub n_region: usize,
    pub n_side: usize,
    /// The longest run of consecutive scans of the region with at least one ion.
    pub run: usize,
    pub ul95: f64,
    pub sn_floor: Option<f64>,
}

/// One judged window.
#[derive(Clone, Debug, PartialEq)]
pub struct Judged {
    pub verdict: &'static str,
    pub rule: String,
    pub hit: Option<Hit>,
    pub counts: Option<Counts>,
    pub flags: Vec<&'static str>,
}

fn cmp(ok: bool) -> &'static str {
    if ok {
        ">="
    } else {
        "<"
    }
}

/// The counts of the scans `region` (indices) against the sides, from the intensities `y` and the ion quantum `q` (cps per ion).
fn count_stats(
    y: &[f64],
    region: &[usize],
    sides: &[usize],
    q: f64,
    hit: Option<&Hit>,
    t: &Thresholds,
) -> Counts {
    let z = z_of(t.alpha);
    let c = |i: usize| y[i] / q;
    let n_p = region.len() as f64;
    let n_b = sides.len() as f64;
    let gross: f64 = region.iter().map(|&i| c(i)).sum();
    let lambda = if sides.is_empty() {
        0.0
    } else {
        sides.iter().map(|&i| c(i)).sum::<f64>() / n_b
    };
    let net = gross - lambda * n_p;
    let var = gross
        + if sides.is_empty() {
            0.0
        } else {
            lambda * n_p * n_p / n_b
        };
    let l_c = if sides.is_empty() {
        0.0
    } else {
        z * (lambda * n_p * (1.0 + n_p / n_b)).sqrt()
    };
    // never under the exact Poisson limit of zero background (3.0 counts at alpha 0.05)
    let l_d = (z * z + 2.0 * l_c).max(-t.alpha.ln());
    let sd = var.max(1.0).sqrt();
    let (mut run, mut cur) = (0usize, 0usize);
    let mut prev: Option<usize> = None;
    for &i in region {
        let ion = y[i] >= q * (1.0 - QUANTUM_TOL);
        cur = if ion && prev.is_some_and(|p| p + 1 == i) {
            cur + 1
        } else {
            usize::from(ion)
        };
        run = run.max(cur);
        prev = Some(i);
    }
    Counts {
        gross,
        net,
        sigma: var.sqrt(),
        l_c,
        l_d,
        z: net / sd,
        background: lambda,
        n_region: region.len(),
        n_side: sides.len(),
        run,
        ul95: l_d.max(net.max(0.0) + z * sd),
        sn_floor: hit.map(|h| h.height / (q * lambda.sqrt().max(1.0))),
    }
}

/// The verdict of one window `[a, b]` of the trace (`rt`, `y` rounded like `trace xic`) with the ion quantum `q` of the file, `None` when its
/// intensities are not counts. The numbers of `hit` are those of `quant targets`; the counts and the rule only decide the word.
pub fn judge(rt: &[f64], y: &[f64], win: (f64, f64), q: Option<f64>, t: &Thresholds) -> Judged {
    let (a, b) = win;
    let none = |reason: String| Judged {
        verdict: "no_data",
        rule: reason,
        hit: None,
        counts: None,
        flags: Vec::new(),
    };
    if !rt.iter().any(|&x| x >= a && x <= b) {
        return none(format!(
            "no scan of the level between {} and {} min",
            rd(a),
            rd(b)
        ));
    }
    let (hit, _, _, edge) = tallest(rt, y, a, b, SMOOTH_HALF, VALLEY_PCT);
    let mut flags = Vec::new();
    if edge {
        flags.push("edge");
    }
    let Some(q) = q else {
        flags.push("no_quantum");
        let (verdict, rule) = match &hit {
            Some(h) => {
                let (ok_sn, ok_pt) = (h.sn >= t.min_sn, h.points >= t.min_points);
                let v = if ok_sn && ok_pt { "peak" } else { "trace" };
                (v, format!("no ion quantum: pipeline S/N {} {} min_sn {}; points {} {} min_points {} -> {v}", rd(h.sn), cmp(ok_sn), rd(t.min_sn), h.points, cmp(ok_pt), t.min_points))
            }
            None => (
                "nothing",
                "no ion quantum and no peak in the window -> nothing".to_string(),
            ),
        };
        return Judged {
            verdict,
            rule,
            hit,
            counts: None,
            flags,
        };
    };
    let (lo, hi) = hit.as_ref().map_or((a, b), |h| (h.start_rt, h.end_rt));
    let region: Vec<usize> = (0..rt.len())
        .filter(|&i| rt[i] >= lo && rt[i] <= hi)
        .collect();
    let w = (b - a).max(t.side_min);
    let side = |i: usize| {
        (rt[i] >= a - w && rt[i] <= a - t.gap_min) || (rt[i] >= b + t.gap_min && rt[i] <= b + w)
    };
    let sides: Vec<usize> = (0..rt.len())
        .filter(|&i| side(i) && !region.contains(&i))
        .collect();
    let left = sides.iter().filter(|&&i| rt[i] < a).count();
    if sides.is_empty() {
        flags.push("no_background");
    } else if left == 0 || left == sides.len() {
        flags.push("one_side");
    }
    let c = count_stats(y, &region, &sides, q, hit.as_ref(), t);
    let ok_n = c.net >= c.l_d;
    let ok_sn = c.sn_floor.is_some_and(|s| s >= t.min_sn);
    let ok_run = c.run >= t.min_points;
    let peak = hit.is_some() && ok_n && ok_sn && ok_run;
    let trace = !peak && c.gross >= 2.0 && c.net >= c.l_c;
    let verdict = if peak {
        "peak"
    } else if trace {
        "trace"
    } else {
        "nothing"
    };
    let sn = c.sn_floor.map_or("n/a (no peak found)".to_string(), |s| {
        format!("{} {} min_sn {}", rd(s), cmp(ok_sn), rd(t.min_sn))
    });
    let mut rule = format!(
        "N {} {} L_D {}; sn_floor {sn}; run {} {} min_points {}",
        rd(c.net),
        cmp(ok_n),
        rd(c.l_d),
        c.run,
        cmp(ok_run),
        t.min_points
    );
    if !peak {
        rule += &format!(
            "; gross {} {} 2; N {} {} L_C {}",
            rd(c.gross),
            cmp(c.gross >= 2.0),
            rd(c.net),
            cmp(c.net >= c.l_c),
            rd(c.l_c)
        );
    }
    rule += &format!(" -> {verdict}");
    Judged {
        verdict,
        rule,
        hit,
        counts: Some(c),
        flags,
    }
}

/// The median step of the scan times, seconds (0 for fewer than two scans).
fn step_seconds(rt: &[f64]) -> f64 {
    let mut d: Vec<f64> = rt.windows(2).map(|w| w[1] - w[0]).collect();
    if d.is_empty() {
        return 0.0;
    }
    crate::sortc::slice_by(&mut d, f64::total_cmp);
    let n = d.len();
    let m = if n % 2 == 1 {
        d[n / 2]
    } else {
        (d[n / 2 - 1] + d[n / 2]) / 2.0
    };
    m * SECONDS
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

fn opt3(o: &mut Out, v: Option<f64>) {
    o.opt(v.map(|x| round(x, 3)));
}

/// One row (JSON): the mass, its windows, the verdict with its rule and every number behind it.
fn row_json(
    mz: f64,
    da: f64,
    rt: (f64, f64),
    j: &Judged,
    dt_s: f64,
    q: Option<f64>,
    offset: bool,
) -> String {
    let mut o = Out::default();
    o.raw("{\"mz\":")
        .num(mz)
        .raw(",\"window\":")
        .rounded(&[mz - da, mz + da], 4);
    o.raw(",\"rt_window\":").floats(&[rt.0, rt.1]);
    o.raw(",\"verdict\":")
        .str(j.verdict)
        .raw(",\"rule\":")
        .str(&j.rule);
    let h = j.hit.as_ref();
    for (k, v) in [
        ("apex_rt", h.map(|h| h.rt)),
        ("height", h.map(|h| h.height)),
        ("area", h.map(|h| h.area)),
        ("sn", h.map(|h| h.sn)),
    ] {
        o.raw(",\"").raw(k).raw("\":").opt(v);
    }
    o.raw(",\"sn_estimator\":").str(SN_ESTIMATOR);
    let c = j.counts.as_ref();
    o.raw(",\"sn_floor\":");
    opt3(&mut o, c.and_then(|c| c.sn_floor));
    for (k, v) in [
        ("start_rt", h.map(|h| h.start_rt)),
        ("end_rt", h.map(|h| h.end_rt)),
    ] {
        o.raw(",\"").raw(k).raw("\":").opt(v);
    }
    o.raw(",\"points\":")
        .opt(h.map(|h| h.points as f64))
        .raw(",\"shape\":")
        .opt(h.map(|h| h.shape));
    for (k, v) in [
        ("counts_gross", c.map(|c| c.gross)),
        ("counts_net", c.map(|c| c.net)),
        ("counts_sigma", c.map(|c| c.sigma)),
        ("l_c", c.map(|c| c.l_c)),
        ("l_d", c.map(|c| c.l_d)),
        ("z_counts", c.map(|c| c.z)),
        ("background_per_scan", c.map(|c| c.background)),
    ] {
        o.raw(",\"").raw(k).raw("\":");
        opt3(&mut o, v);
    }
    // the upper limit says how much could hide under a verdict of nothing
    let ul = c.filter(|_| j.verdict == "nothing").map(|c| c.ul95);
    o.raw(",\"ul95_counts\":");
    opt3(&mut o, ul);
    o.raw(",\"ul95_area\":");
    opt3(&mut o, ul.zip(q).map(|(u, q)| u * q * dt_s));
    o.raw(",\"flags\":[");
    let mut flags: Vec<&str> = j.flags.clone();
    if offset {
        flags.push("offset_applied");
    }
    for (i, f) in flags.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(f);
    }
    o.raw("]}");
    o.0
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
        let mut o = Out::default();
        o.raw("\"ms_level\":")
            .int(i64::from(ask.level))
            .raw(",\"n_scans\":")
            .int(n as i64);
        o.raw(",\"scan_time_s\":").num(round(dt_s, 4));
        o.raw(",\"ion_quantum\":")
            .raw(&crate::rawinfo::quantum_json(Some(&probe)));
        o.raw(",\"mz_offset\":");
        match (&ask.offset, &off) {
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
        o.raw(",\"thresholds\":{\"min_sn\":")
            .num(t.min_sn)
            .raw(",\"min_points\":")
            .int(t.min_points as i64);
        o.raw(",\"alpha\":")
            .num(t.alpha)
            .raw(",\"z_alpha\":")
            .num(round(z_of(t.alpha), 4));
        o.raw(",\"side_min\":")
            .num(t.side_min)
            .raw(",\"gap_min\":")
            .num(t.gap_min);
        o.raw(",\"smooth_half\":")
            .int(SMOOTH_HALF as i64)
            .raw(",\"valley_pct\":")
            .num(VALLEY_PCT)
            .raw("}");
        Ok(XicOut {
            members: o.0,
            rows: rows.into_iter().flatten().collect(),
        })
    }
}

#[cfg(test)]
#[path = "rawxic_tests.rs"]
mod tests;
