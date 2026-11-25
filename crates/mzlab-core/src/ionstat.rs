//! The noise of an ion-counting trace with a floor of one ion (rule R1 of docs/studies/TP_SCREENING_RULES.md): noise = max(1.4826 MAD, 0.5 SD,
//! one ion x sqrt(max(lambda, 1))), lambda = mean background counts per point. A pure MAD is 0 when most points are zero and the S/N explodes;
//! the Poisson term keeps it at one ion near zero and at the counting error above. One function for `mzlab-cli peak noise` and mzlab-wasm
//! `noise_floor` (the same JSON bytes); every term, the one that won and the check are in the output, nothing is hidden.

use crate::json::Value;
use crate::jsonb::{num, opt, text, write, Obj};

/// Scale of the MAD to a normal SD.
pub const MAD_K: f64 = 1.4826;
/// Default weight of the SD term.
pub const SD_WEIGHT: f64 = 0.5;
/// Default smallest S/N of the check (the `min_snr` of the Dig peak check).
pub const MIN_SN: f64 = 5.0;
/// A positive value is a multiple of the quantum when its ratio is within this of an integer.
pub const QUANTUM_TOL: f64 = 0.02;
/// Share of positive values that must be multiples of the quantum for it to be accepted.
pub const QUANTUM_SUPPORT: f64 = 0.9;
/// Fewest positive values behind an automatic quantum.
pub const QUANTUM_MIN_N: usize = 3;
/// Largest median of the positive values, in quanta, of an accepted quantum: a counting background sits at a few ions per point,
/// integer-valued analog or processed data (1000, 1001, 1003 ...) at hundreds or thousands of its unit step.
pub const QUANTUM_MAX_IONS: f64 = 100.0;

/// Relative difference under which two values are the same value for the quantum (rounding of f32 intensities and of their sums).
pub const JITTER: f64 = 1e-6;

/// Where the cps of one ion comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OneIon {
    /// The quantum of the trace (smallest positive value or step between values, accepted only with its support).
    Auto,
    /// A value the user set (cps per ion of the instrument and scan mode).
    Cps(f64),
    /// No ion term: noise = max(MAD, SD term).
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorParams {
    pub one_ion: OneIon,
    pub sd_weight: f64,
    pub min_sn: f64,
}

impl Default for FloorParams {
    fn default() -> Self {
        FloorParams {
            one_ion: OneIon::Auto,
            sd_weight: SD_WEIGHT,
            min_sn: MIN_SN,
        }
    }
}

impl FloorParams {
    /// The numbers of the wasm interface: one_ion (negative or NaN = auto, 0 = off, > 0 cps), sd_weight, min_sn; a missing, NaN or negative
    /// weight or threshold keeps its default.
    pub fn from_slice(p: &[f64]) -> FloorParams {
        let d = FloorParams::default();
        let g = |i: usize| p.get(i).copied().filter(|v| v.is_finite() && *v >= 0.0);
        FloorParams {
            one_ion: match p.first().copied() {
                Some(v) if v.is_finite() && v > 0.0 => OneIon::Cps(v),
                Some(0.0) => OneIon::Off,
                _ => OneIon::Auto,
            },
            sd_weight: g(1).unwrap_or(d.sd_weight),
            min_sn: g(2).unwrap_or(d.min_sn),
        }
    }
}

/// The quantum of a trace: the smallest positive value or step between distinct values, the share of positive values that are its integer
/// multiples and their median in quanta (`level`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quantum {
    pub q: f64,
    pub support: f64,
    pub n_positive: usize,
    pub level: f64,
    pub accepted: bool,
}

/// The quantum of `y` (`None` without positive values); accepted with at least `QUANTUM_MIN_N` positive values, `QUANTUM_SUPPORT` and a
/// `level` of at most `QUANTUM_MAX_IONS` (integer spacing alone is not evidence of counting).
pub fn quantum(y: &[f64]) -> Option<Quantum> {
    let pos: Vec<f64> = y
        .iter()
        .copied()
        .filter(|v| v.is_finite() && *v > 0.0)
        .collect();
    if pos.is_empty() {
        return None;
    }
    // with a high background no point may hold a single ion: the smallest step between distinct values is one too
    let mut d = pos.clone();
    let med = median(&mut d);
    // values within `JITTER` of each other are one value: a sum of f32 intensities (an XIC over a window) leaves steps of 1e-3 cps that are
    // rounding, not a step of the signal
    d.dedup_by(|b, a| *b - *a <= JITTER * *b);
    let step = d
        .windows(2)
        .map(|w| w[1] - w[0])
        .fold(f64::INFINITY, f64::min);
    let q = d[0].min(step);
    let on = pos
        .iter()
        .filter(|&&v| (v / q - (v / q).round()).abs() <= QUANTUM_TOL)
        .count();
    let support = on as f64 / pos.len() as f64;
    let level = med / q;
    Some(Quantum {
        q,
        support,
        n_positive: pos.len(),
        level,
        accepted: pos.len() >= QUANTUM_MIN_N
            && support >= QUANTUM_SUPPORT
            && level <= QUANTUM_MAX_IONS,
    })
}

fn median(v: &mut [f64]) -> f64 {
    crate::sortc::slice_by(v, f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// The noise of the background points and every term behind it.
#[derive(Clone, Debug, PartialEq)]
pub struct Floor {
    pub n: usize,
    pub median: f64,
    pub mean: f64,
    pub mad: f64,
    pub sd: f64,
    pub sd_term: f64,
    /// cps of one ion used (`None`: off, or no accepted quantum).
    pub one_ion: Option<f64>,
    /// `set`, `auto`, `off` or `auto_rejected` (a quantum was looked for and not accepted).
    pub one_ion_source: &'static str,
    pub quantum: Option<Quantum>,
    /// Mean background counts per point.
    pub lambda: Option<f64>,
    pub ion_term: Option<f64>,
    pub noise: f64,
    /// The term that gave the noise: `mad`, `sd`, `one_ion` (lambda <= 1) or `poisson`; ties go to the first of that order.
    pub estimator: &'static str,
}

/// The noise of `bg` (background points, at least one) with the floor of one ion.
pub fn floor(bg: &[f64], p: &FloorParams) -> Floor {
    let n = bg.len();
    let mut v = bg.to_vec();
    let med = median(&mut v);
    let mean = bg.iter().sum::<f64>() / n as f64;
    let mut dev: Vec<f64> = bg.iter().map(|x| (x - med).abs()).collect();
    let mad = MAD_K * median(&mut dev);
    let sd = if n > 1 {
        (bg.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt()
    } else {
        0.0
    };
    let sd_term = p.sd_weight * sd;
    let quant = quantum(bg);
    let (one_ion, source) = match p.one_ion {
        OneIon::Cps(c) => (Some(c), "set"),
        OneIon::Off => (None, "off"),
        OneIon::Auto => match quant {
            Some(q) if q.accepted => (Some(q.q), "auto"),
            _ => (None, "auto_rejected"),
        },
    };
    let lambda = one_ion.map(|c| mean.max(0.0) / c);
    let ion_term = one_ion.zip(lambda).map(|(c, l)| c * l.max(1.0).sqrt());
    let mut noise = mad;
    let mut estimator = "mad";
    if sd_term > noise {
        (noise, estimator) = (sd_term, "sd");
    }
    if let Some(t) = ion_term.filter(|&t| t > noise) {
        let poisson = lambda.is_some_and(|l| l > 1.0);
        (noise, estimator) = (t, if poisson { "poisson" } else { "one_ion" });
    }
    Floor {
        n,
        median: med,
        mean,
        mad,
        sd,
        sd_term,
        one_ion,
        one_ion_source: source,
        quantum: quant,
        lambda,
        ion_term,
        noise,
        estimator,
    }
}

/// What to measure: the stretch [from, to] of the trace (min; `None` = its end) and the peak window [a, b] left out of the background.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FloorAsk {
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub peak: Option<(f64, f64)>,
}

impl FloorAsk {
    /// The numbers of the wasm interface: from, to, peak start, peak end (min); NaN = not given (both peak ends are needed).
    pub fn from_slice(a: &[f64]) -> FloorAsk {
        let g = |i: usize| a.get(i).copied().filter(|v| v.is_finite());
        FloorAsk {
            from: g(0),
            to: g(1),
            peak: g(2).zip(g(3)),
        }
    }
}

/// The peak of the window: apex above the background median, its S/N on the floor and on the MAD alone (`None` when the MAD is 0).
#[derive(Clone, Debug, PartialEq)]
pub struct PeakSn {
    pub from: f64,
    pub to: f64,
    pub apex_rt: f64,
    pub height: f64,
    pub sn: Option<f64>,
    pub sn_mad: Option<f64>,
    pub ions: Option<f64>,
}

/// The check of the peak: `pass` (S/N on the floor >= min_sn), `fail`, `warn` (no ion term, or noise 0: the floor could not be applied) or
/// `n/a` (no finite point in the peak window).
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub status: &'static str,
    pub rule: String,
}

/// Rounded for the text of a rule.
pub(crate) fn rd(v: f64) -> String {
    let a = v.abs();
    let d = if a >= 100.0 {
        0
    } else if a >= 1.0 {
        1
    } else {
        3
    };
    format!("{v:.d$}")
}

fn check(f: &Floor, pk: &PeakSn, sd_weight: f64, min_sn: f64) -> Check {
    let terms = format!(
        "noise {} = {} (MAD {}, {} SD {}, ion term {})",
        rd(f.noise),
        f.estimator,
        rd(f.mad),
        rd(sd_weight),
        rd(f.sd_term),
        f.ion_term.map_or("none".to_string(), rd)
    );
    let Some(sn) = pk.sn else {
        return Check {
            status: "warn",
            rule: format!("{terms}; noise 0 and no ion term: S/N not defined, set one_ion"),
        };
    };
    let (status, cmp) = match (f.ion_term.is_some(), sn >= min_sn) {
        (false, _) => ("warn", if sn >= min_sn { ">=" } else { "<" }),
        (true, true) => ("pass", ">="),
        (true, false) => ("fail", "<"),
    };
    let tail = if f.ion_term.is_none() {
        " (no ion term: one_ion off or no accepted quantum)"
    } else {
        ""
    };
    Check {
        status,
        rule: format!(
            "{terms}; S/N {} {cmp} {} -> {status}{tail}",
            rd(sn),
            rd(min_sn)
        ),
    }
}

/// The result of one trace.
#[derive(Clone, Debug, PartialEq)]
pub struct FloorResult {
    pub params: FloorParams,
    pub ask: FloorAsk,
    pub floor: Floor,
    pub peak: Option<PeakSn>,
    pub check: Option<Check>,
}

/// The noise floor of the trace (rt in min, y in cps) and, with a peak window, its S/N and check. Err: no background point.
pub fn measure(
    rt: &[f64],
    y: &[f64],
    ask: FloorAsk,
    p: FloorParams,
) -> Result<FloorResult, String> {
    let n = rt.len().min(y.len());
    let lo = ask.from.unwrap_or(f64::NEG_INFINITY);
    let hi = ask.to.unwrap_or(f64::INFINITY);
    let inside = |i: usize| rt[i] >= lo && rt[i] <= hi && y[i].is_finite();
    let in_peak = |i: usize| ask.peak.is_some_and(|(a, b)| rt[i] >= a && rt[i] <= b);
    let bg: Vec<f64> = (0..n)
        .filter(|&i| inside(i) && !in_peak(i))
        .map(|i| y[i])
        .collect();
    if bg.is_empty() {
        return Err("err.noise.noBackground".into());
    }
    let f = floor(&bg, &p);
    let peak = ask.peak.and_then(|(a, b)| {
        let top = (0..n)
            .filter(|&i| in_peak(i) && y[i].is_finite())
            .max_by(|&i, &j| y[i].total_cmp(&y[j]))?;
        let height = y[top] - f.median;
        let div = |d: f64| (d > 0.0).then(|| height / d);
        Some(PeakSn {
            from: a,
            to: b,
            apex_rt: rt[top],
            height,
            sn: div(f.noise),
            sn_mad: div(f.mad),
            ions: f.one_ion.map(|c| height / c),
        })
    });
    let check = match (&peak, ask.peak) {
        (Some(pk), _) => Some(check(&f, pk, p.sd_weight, p.min_sn)),
        (None, Some((a, b))) => Some(Check {
            status: "n/a",
            rule: format!(
                "no finite point in the peak window {}-{} min: no S/N",
                rd(a),
                rd(b)
            ),
        }),
        (None, None) => None,
    };
    Ok(FloorResult {
        params: p,
        ask,
        floor: f,
        peak,
        check,
    })
}

fn quantum_json(q: &Option<Quantum>) -> Value {
    q.map_or(Value::Null, |q| {
        Obj::new()
            .n("q", q.q)
            .n("support", q.support)
            .i("n_positive", q.n_positive as i64)
            .n("level", q.level)
            .b("accepted", q.accepted)
            .done()
    })
}

/// The `noise-floor/1` JSON (the bytes of `mzlab-cli peak noise` and of mzlab-wasm `noise_floor`).
pub fn floor_json(r: &FloorResult) -> String {
    let f = &r.floor;
    let one_ion = match r.params.one_ion {
        OneIon::Auto => text("auto"),
        OneIon::Off => num(0.0),
        OneIon::Cps(c) => num(c),
    };
    let params = Obj::new()
        .put("one_ion", one_ion)
        .n("sd_weight", r.params.sd_weight)
        .n("min_sn", r.params.min_sn)
        .n("mad_k", MAD_K)
        .put("from", opt(r.ask.from))
        .put("to", opt(r.ask.to))
        .done();
    let bg = Obj::new()
        .i("n", f.n as i64)
        .n("median", f.median)
        .n("mean", f.mean)
        .done();
    let terms = Obj::new()
        .n("mad", f.mad)
        .n("sd", f.sd)
        .n("sd_term", f.sd_term)
        .put("one_ion", opt(f.one_ion))
        .put("one_ion_source", text(f.one_ion_source))
        .put("lambda", opt(f.lambda))
        .put("ion_term", opt(f.ion_term))
        .done();
    let peak = r.peak.as_ref().map_or(Value::Null, |p| {
        Obj::new()
            .n("from", p.from)
            .n("to", p.to)
            .n("apex_rt", p.apex_rt)
            .n("height", p.height)
            .put("sn", opt(p.sn))
            .put("sn_mad", opt(p.sn_mad))
            .put("ions", opt(p.ions))
            .done()
    });
    let check = r.check.as_ref().map_or(Value::Null, |c| {
        Obj::new()
            .put("id", text("noise_floor"))
            .put("status", text(c.status))
            .put("value", opt(r.peak.as_ref().and_then(|p| p.sn)))
            .n("threshold", r.params.min_sn)
            .put("rule", text(&c.rule))
            .done()
    });
    write(
        &Obj::new()
            .put("schema", text("noise-floor/1"))
            .put("params", params)
            .put("background", bg)
            .put("quantum", quantum_json(&f.quantum))
            .put("terms", terms)
            .n("noise", f.noise)
            .put("estimator", text(f.estimator))
            .put("peak", peak)
            .put("check", check)
            .done(),
    )
}

/// The one log line of a result (stderr of the CLI): what the floor chose and why.
pub fn floor_log(r: &FloorResult) -> String {
    let f = &r.floor;
    let base = format!(
        "noise floor: {} cps from {} ({} background points, one ion {} [{}])",
        rd(f.noise),
        f.estimator,
        f.n,
        f.one_ion.map_or("none".to_string(), rd),
        f.one_ion_source
    );
    match &r.check {
        Some(c) => format!("{base}; {}", c.rule),
        None => base,
    }
}

#[cfg(test)]
#[path = "ionstat_tests.rs"]
mod tests;
