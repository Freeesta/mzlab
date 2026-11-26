//! `raw isotopes` (`/api/rawiso`, `docs/agenti/raw-access.md`): do M+1 and M+2 fit a formula, within counting error? Net ion counts of the bins M, M+1 and
//! M+2 over the peak region of M (the sides subtracted), the observed ratios with their delta-method sigma, the expected pattern of `ion pattern` with z
//! scores, or without a formula the carbon range and the M+2 hint. The CLI, the app (`rawinfo::handle`) and the site (the lazy `.wasm` `rawiso`) answer
//! through here, so the bytes are one.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::ionpattern::{ion_counts, unit_pattern};
use crate::json::round;
use crate::mzcal::{Offset, Spec};
use crate::rawxic::{self, judge, tallest, PROBE, SMOOTH_HALF, VALLEY_PCT};
use crate::routes::{Query, Routes};

/// Defaults of the verdicts (`--set`): the smallest net count that counts, the |z| under which a ratio is consistent and from which it is not, the
/// carbon-13 share per carbon (%), the half width of the carbon range in sigma, the M+2 hint (a share of R1 and sigma), the least correlation of M+1 with M.
pub const MIN_COUNTS: f64 = 10.0;
pub const Z_OK: f64 = 2.0;
pub const Z_BAD: f64 = 3.0;
pub const C13_PCT: f64 = 1.1;
pub const CARBON_K: f64 = 2.0;
pub const M2_RATIO: f64 = 0.2;
pub const M2_K: f64 = 3.0;
pub const MIN_R: f64 = 0.8;
/// The adduct taken when a formula comes without one.
pub const DEFAULT_ADDUCT: &str = "[M+H]+";

/// The thresholds of the verdicts and the ones of the judge of M (`raw xic`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub min_counts: f64,
    pub z_ok: f64,
    pub z_bad: f64,
    pub c13_pct: f64,
    pub carbon_k: f64,
    pub m2_ratio: f64,
    pub m2_k: f64,
    pub min_r: f64,
    pub x: rawxic::Thresholds,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            min_counts: MIN_COUNTS,
            z_ok: Z_OK,
            z_bad: Z_BAD,
            c13_pct: C13_PCT,
            carbon_k: CARBON_K,
            m2_ratio: M2_RATIO,
            m2_k: M2_K,
            min_r: MIN_R,
            x: rawxic::Thresholds::default(),
        }
    }
}

fn bad(name: &str, v: &str) -> Error {
    Error::new("err.raw.isotopes.param")
        .with("name", name.to_string())
        .with("value", v.to_string())
}

/// The question of `raw isotopes` as the route reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub mz: f64,
    pub rt: (f64, f64),
    pub level: u8,
    pub offset: Option<Spec>,
    pub formula: Option<String>,
    /// The adduct asked, else `DEFAULT_ADDUCT` when a formula is given.
    pub adduct: Option<String>,
    pub p: Params,
}

impl Ask {
    /// `mz=M&rt=A,B[&formula=F][&adduct=A][&bin=nominal][&ms_level=N][&mz_offset=TEXT][&min_counts=..&z_ok=..&z_bad=..&c13_pct=..&carbon_k=..&m2_ratio=..&m2_k=..
    /// &min_r=..&min_sn=..&min_points=..&alpha=..&side_min=..&gap_min=..]`; a value out of its range is refused, so is a formula the catalogue cannot read.
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
        let mz = match num("mz")? {
            Some(x) if x > 0.0 => x,
            _ => return Err(bad("mz", q.some("mz").unwrap_or(""))),
        };
        let rt = q.some("rt").ok_or_else(|| bad("rt", ""))?;
        let w: Vec<f64> = rt
            .split(',')
            .filter_map(|t| t.trim().parse::<f64>().ok().filter(|x| x.is_finite()))
            .collect();
        let rt = match w[..] {
            [a, b] if a < b && rt.split(',').count() == 2 => (a, b),
            _ => return Err(bad("rt", rt)),
        };
        if let Some(v) = q.some("bin").filter(|v| *v != "nominal") {
            return Err(bad("bin", v));
        }
        let level = match num("ms_level")? {
            None => 1,
            Some(x) if (1.0..=10.0).contains(&x) && x.fract() == 0.0 => x as u8,
            Some(x) => return Err(bad("ms_level", &x.to_string())),
        };
        let formula = q
            .some("formula")
            .filter(|f| !f.is_empty())
            .map(str::to_string);
        if formula.is_none() && q.some("adduct").is_some() {
            return Err(Error::new("err.raw.isotopes.adductNeedsFormula"));
        }
        let adduct = formula.as_ref().map(|_| {
            q.some("adduct")
                .filter(|a| !a.is_empty())
                .unwrap_or(DEFAULT_ADDUCT)
                .to_string()
        });
        if let (Some(f), Some(a)) = (&formula, &adduct) {
            // an unknown adduct or element is refused here, before any file is opened; a charge other than 1 has no M+1 at +1 Da
            let (_, z) = ion_counts(f, a)?;
            if z.abs() != 1 {
                return Err(Error::new("err.raw.isotopes.charge")
                    .with("adduct", a.as_str())
                    .with("z", i64::from(z)));
            }
        }
        let d = Params::default();
        let count = |k: &str, d: usize| match num(k)? {
            None => Ok(d),
            Some(x) if x >= 1.0 && x <= 1e6 && x.fract() == 0.0 => Ok(x as usize),
            Some(x) => Err(bad(k, &x.to_string())),
        };
        Ok(Ask {
            mz,
            rt,
            level,
            offset: q.some("mz_offset").map(Spec::parse).transpose()?,
            formula,
            adduct,
            p: Params {
                min_counts: range("min_counts", d.min_counts, 0.0, 1e9)?,
                z_ok: range("z_ok", d.z_ok, 0.0, 1e3)?,
                z_bad: range("z_bad", d.z_bad, 0.0, 1e3)?,
                c13_pct: range("c13_pct", d.c13_pct, 1e-3, 100.0)?,
                carbon_k: range("carbon_k", d.carbon_k, 0.0, 1e3)?,
                m2_ratio: range("m2_ratio", d.m2_ratio, 0.0, 1e3)?,
                m2_k: range("m2_k", d.m2_k, 0.0, 1e3)?,
                min_r: range("min_r", d.min_r, -1.0, 1.0)?,
                x: rawxic::Thresholds {
                    min_sn: range("min_sn", d.x.min_sn, 0.0, 1e6)?,
                    min_points: count("min_points", d.x.min_points)?,
                    alpha: range("alpha", d.x.alpha, 1e-6, 0.5)?,
                    side_min: range("side_min", d.x.side_min, 0.0, 1e4)?,
                    gap_min: range("gap_min", d.x.gap_min, 0.0, 1e4)?,
                },
            },
        })
    }

    /// `z_ok` must be under `z_bad`: a ratio cannot be consistent and inconsistent at once.
    fn check(&self) -> Result<()> {
        if self.p.z_ok > self.p.z_bad {
            return Err(bad("z_ok", &self.p.z_ok.to_string()));
        }
        Ok(())
    }
}

/// The counts of one bin over the peak region, the sides subtracted.
#[derive(Clone, Debug)]
pub(crate) struct Ion {
    pub off: i32,
    pub mz: f64,
    pub gross: f64,
    pub net: f64,
    /// Poisson sigma of the net counts; `None` when the intensities are not ion counts (no quantum).
    pub sigma: Option<f64>,
    /// Mean counts per scan of the sides.
    pub background: f64,
}

/// An observed ratio N_k / N_0 with its delta-method sigma.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ratio {
    pub r: f64,
    pub sigma: Option<f64>,
}

/// The expected pattern of the formula and adduct: R1, R2 and the groups M..M+2 as `ion pattern` gives them.
#[derive(Clone, Debug)]
pub(crate) struct Expected {
    pub formula: String,
    pub adduct: String,
    pub mz: f64,
    pub e1: f64,
    pub e2: f64,
    pub pattern: Vec<(i32, f64, f64)>,
}

/// The verdict of one ratio against its expected value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Test {
    pub z: Option<f64>,
    pub verdict: &'static str,
    pub reason: &'static str,
}

/// Where the region of M came from and which scans it holds.
pub(crate) struct Region {
    pub source: &'static str,
    pub lo: f64,
    pub hi: f64,
    pub apex: Option<f64>,
    pub n: usize,
    pub left: usize,
    pub right: usize,
}

/// Everything one sample answered, before it is written.
pub(crate) struct Found {
    pub m_verdict: &'static str,
    pub m_rule: String,
    pub region: Option<Region>,
    pub ions: Vec<Ion>,
    pub r1: Option<Ratio>,
    pub r2: Option<Ratio>,
    pub expected: Option<Expected>,
    pub tests: Option<(Test, Test)>,
    /// (best, min, max) carbon count from R1, without a formula.
    pub carbon: Option<(f64, f64, f64)>,
    pub m2_hint: Option<(&'static str, String)>,
    pub interference: Option<(Option<f64>, Option<f64>, usize)>,
    pub flags: Vec<&'static str>,
}

/// What one sample answered: the members (no braces) and the `result` object.
pub struct IsoOut {
    pub members: String,
    pub result: String,
}

/// Pearson's r of two columns over the scans `at`; `None` for fewer than 3 scans or a column without variance.
fn pearson(a: &[f64], b: &[f64], at: &[usize]) -> Option<f64> {
    let n = at.len() as f64;
    if at.len() < 3 {
        return None;
    }
    let (ma, mb) = (
        at.iter().map(|&i| a[i]).sum::<f64>() / n,
        at.iter().map(|&i| b[i]).sum::<f64>() / n,
    );
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for &i in at {
        let (da, db) = (a[i] - ma, b[i] - mb);
        sab += da * db;
        saa += da * da;
        sbb += db * db;
    }
    (saa > 0.0 && sbb > 0.0).then(|| sab / (saa * sbb).sqrt())
}

/// R = N_k / N_0 with sigma by the delta method: sqrt(s_k^2 / N_0^2 + N_k^2 s_0^2 / N_0^4); `None` when N_0 is not positive.
fn ratio(k: &Ion, m: &Ion) -> Option<Ratio> {
    (m.net > 0.0).then(|| Ratio {
        r: k.net / m.net,
        sigma: k.sigma.zip(m.sigma).map(|(sk, s0)| {
            (sk * sk / (m.net * m.net) + k.net * k.net * s0 * s0 / m.net.powi(4)).sqrt()
        }),
    })
}

/// The expected ratios of the ion: the groups of `ion pattern` at M, M+1 and M+2 (a group under the pattern's floor counts as 0).
fn expected_of(f: &str, a: &str) -> Result<Option<Expected>> {
    let (n, z) = ion_counts(f, a)?;
    let p = unit_pattern(&n, z)?;
    let at = |o: i32| p.iter().find(|x| x.off == o).map(|x| x.rel);
    let Some(m0) = at(0) else {
        return Ok(None);
    };
    Ok(Some(Expected {
        formula: f.to_string(),
        adduct: a.to_string(),
        mz: p.iter().find(|x| x.off == 0).map_or(0.0, |x| x.mz),
        e1: at(1).unwrap_or(0.0) / m0,
        e2: at(2).unwrap_or(0.0) / m0,
        pattern: p
            .iter()
            .filter(|x| (0..=2).contains(&x.off))
            .map(|x| (x.off, x.mz, x.rel))
            .collect(),
    }))
}

/// z = (observed - expected) / sigma and the word: `consistent` under `z_ok`, `inconsistent` from `z_bad`, else `undetermined`; also `undetermined`
/// with its reason when the counts cannot support a verdict.
fn test(obs: Option<Ratio>, e: f64, n_k: f64, n_0: f64, quantum: bool, p: &Params) -> Test {
    let no = |reason| Test {
        z: None,
        verdict: "undetermined",
        reason,
    };
    let Some(r) = obs else {
        return no("no_m");
    };
    let Some(s) = r.sigma.filter(|_| quantum) else {
        return no("no_quantum");
    };
    let z = (s > 0.0).then(|| (r.r - e) / s);
    if n_k < p.min_counts || n_0 < p.min_counts {
        return Test {
            z,
            ..no("too_few_counts")
        };
    }
    let (verdict, reason) = match z {
        Some(z) if z.abs() < p.z_ok => ("consistent", "within_counting_error"),
        Some(z) if z.abs() >= p.z_bad => ("inconsistent", "outside_counting_error"),
        Some(_) => ("undetermined", "between_limits"),
        None => ("undetermined", "no_sigma"),
    };
    Test { z, verdict, reason }
}

impl<R: ReadAt> Routes<R> {
    /// `/api/rawiso?mz=M&rt=A,B[&formula=..&adduct=..][&ms_level=..][&mz_offset=..][&min_counts=..]...`: the members of `raw-isotopes/1` for this sample and
    /// its `result`.
    pub fn raw_isotopes_route(&mut self, q: &Query) -> Result<Option<String>> {
        let o = self.raw_isotopes(&Ask::of(q)?)?;
        Ok(Some(format!("{{{},\"result\":{}}}", o.members, o.result)))
    }

    /// The answer of `ask` on this sample (its polarity `pol`).
    pub fn raw_isotopes(&mut self, ask: &Ask) -> Result<IsoOut> {
        ask.check()?;
        let keep = self.mz_off;
        let r = self.raw_isotopes_at(ask);
        self.mz_off = keep;
        r
    }

    fn raw_isotopes_at(&mut self, ask: &Ask) -> Result<IsoOut> {
        let off: Option<Offset> = ask
            .offset
            .as_ref()
            .map(|s| s.resolve(&mut self.idx, self.pol))
            .transpose()?;
        if let Some(o) = &off {
            self.mz_off = o.da;
        }
        let probe = self.probe(ask.level, PROBE)?;
        let quant = crate::rawinfo::quantum_of(Some(&probe)).filter(|q| q.accepted);
        let q = quant.map(|q| q.q);
        // the bins of `raw scan --bin nominal`: M is the whole mass nearest to --mz, M+1 and M+2 the next two
        let base = ask.mz.round();
        let centres = [base, base + 1.0, base + 2.0];
        let n = self.xic_len(ask.level);
        self.forget_sweep();
        let (rt, y): (Vec<f64>, Vec<Vec<f64>>) = if n == 0 {
            (Vec::new(), vec![Vec::new(); 3])
        } else {
            let m = self.sweep_matrix(&centres, &[0.5; 3], ask.level, None)?;
            (
                m.rt.iter().map(|&x| round(x, 4)).collect(),
                m.y.iter()
                    .map(|c| c.iter().map(|&v| round(v, 1)).collect())
                    .collect(),
            )
        };
        self.forget_sweep();
        let found = found_of(ask, &rt, &y, centres, q)?;
        Ok(IsoOut {
            members: crate::rawiso_json::members_json(ask, n, &probe, off.as_ref(), q.is_some()),
            result: crate::rawiso_json::result_json(ask, &found),
        })
    }
}

/// The scans of the region of M and of its two sides: the region is the peak the integrator finds in the window, else the window itself; the sides are as
/// wide as the window (at least `side_min`), `gap_min` away from it (the rule of `raw xic`).
fn scans_of(
    rt: &[f64],
    region: (f64, f64),
    win: (f64, f64),
    p: &rawxic::Thresholds,
) -> (Vec<usize>, Vec<usize>) {
    let inside: Vec<usize> = (0..rt.len())
        .filter(|&i| rt[i] >= region.0 && rt[i] <= region.1)
        .collect();
    let w = (win.1 - win.0).max(p.side_min);
    let side = |i: usize| {
        (rt[i] >= win.0 - w && rt[i] <= win.0 - p.gap_min)
            || (rt[i] >= win.1 + p.gap_min && rt[i] <= win.1 + w)
    };
    let sides = (0..rt.len())
        .filter(|&i| side(i) && !inside.contains(&i))
        .collect();
    (inside, sides)
}

/// The counts of the column `y` over the region against the sides, in ions when the quantum `q` is known, else in the intensity units.
fn ion_of(off: i32, mz: f64, y: &[f64], at: (&[usize], &[usize]), q: Option<f64>) -> Ion {
    let u = q.unwrap_or(1.0);
    let (n_p, n_b) = (at.0.len() as f64, at.1.len() as f64);
    let gross: f64 = at.0.iter().map(|&i| y[i] / u).sum();
    let lambda = if at.1.is_empty() {
        0.0
    } else {
        at.1.iter().map(|&i| y[i] / u).sum::<f64>() / n_b
    };
    let var = gross
        + if at.1.is_empty() {
            0.0
        } else {
            lambda * n_p * n_p / n_b
        };
    Ion {
        off,
        mz,
        gross,
        net: gross - lambda * n_p,
        sigma: q.map(|_| var.sqrt()),
        background: lambda,
    }
}

fn found_of(
    ask: &Ask,
    rt: &[f64],
    y: &[Vec<f64>],
    centres: [f64; 3],
    q: Option<f64>,
) -> Result<Found> {
    let p = &ask.p;
    let j = judge(rt, &y[0], ask.rt, q, &p.x);
    let mut f = Found {
        m_verdict: j.verdict,
        m_rule: j.rule.clone(),
        region: None,
        ions: Vec::new(),
        r1: None,
        r2: None,
        expected: None,
        tests: None,
        carbon: None,
        m2_hint: None,
        interference: None,
        flags: j.flags.clone(),
    };
    if j.verdict == "no_data" {
        return Ok(f);
    }
    let hit = tallest(rt, &y[0], ask.rt.0, ask.rt.1, SMOOTH_HALF, VALLEY_PCT).0;
    let span = hit.as_ref().map_or(ask.rt, |h| (h.start_rt, h.end_rt));
    if hit.is_none() {
        f.flags.push("no_peak_region");
    }
    let (inside, sides) = scans_of(rt, span, ask.rt, &p.x);
    if sides.is_empty() {
        f.flags.push("no_background");
    }
    let left = sides.iter().filter(|&&i| rt[i] < ask.rt.0).count();
    f.region = Some(Region {
        source: if hit.is_some() { "peak" } else { "window" },
        lo: span.0,
        hi: span.1,
        apex: hit.as_ref().map(|h| h.rt),
        n: inside.len(),
        left,
        right: sides.len() - left,
    });
    f.ions = (0..3)
        .map(|k| ion_of(k as i32, centres[k], &y[k], (&inside, &sides), q))
        .collect();
    f.r1 = ratio(&f.ions[1], &f.ions[0]);
    f.r2 = ratio(&f.ions[2], &f.ions[0]);
    f.interference = Some((
        pearson(&y[1], &y[0], &inside),
        pearson(&y[2], &y[0], &inside),
        inside.len(),
    ));
    if let (Some(fm), Some(ad)) = (&ask.formula, &ask.adduct) {
        f.expected = expected_of(fm, ad)?;
        match &f.expected {
            Some(e) => {
                let (n0, n1, n2) = (f.ions[0].net, f.ions[1].net, f.ions[2].net);
                let has = q.is_some();
                f.tests = Some((
                    test(f.r1, e.e1, n1, n0, has, p),
                    test(f.r2, e.e2, n2, n0, has, p),
                ));
                if (e.mz - ask.mz).abs() > 0.5 {
                    f.flags.push("formula_mz_mismatch");
                }
            }
            None => f.flags.push("no_expected_m"),
        }
    } else if let Some(r1) = f.r1 {
        let c = p.c13_pct / 100.0;
        let s = r1.sigma.map_or(0.0, |s| s * p.carbon_k);
        f.carbon = Some((
            (r1.r / c).round().max(0.0),
            ((r1.r - s) / c).floor().max(0.0),
            ((r1.r + s) / c).ceil().max(0.0),
        ));
        if f.ions[1].net < p.min_counts || f.ions[0].net < p.min_counts {
            f.flags.push("too_few_counts");
        }
        f.m2_hint = hint_of(f.r1, f.r2, p);
    }
    Ok(f)
}

/// `cl_br_s_possible` when R2 > m2_ratio x R1 + m2_k x sigma(R2), else `none`; evidence only, with the numbers of the rule.
fn hint_of(r1: Option<Ratio>, r2: Option<Ratio>, p: &Params) -> Option<(&'static str, String)> {
    let (r1, r2) = (r1?, r2?);
    let s = r2.sigma?;
    let limit = p.m2_ratio * r1.r + p.m2_k * s;
    let hit = r2.r > limit;
    Some((
        if hit { "cl_br_s_possible" } else { "none" },
        format!(
            "R2 {} {} {} x R1 + {} x sigma = {}",
            round(r2.r, 4),
            if hit { ">" } else { "<=" },
            p.m2_ratio,
            p.m2_k,
            round(limit, 4)
        ),
    ))
}

#[cfg(test)]
#[path = "rawiso_tests.rs"]
mod tests;
