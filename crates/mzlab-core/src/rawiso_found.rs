//! The counting of `raw isotopes` (`rawiso`): the region of M, the net counts of its three bins against the sides, the observed ratios, the verdicts against the
//! expected pattern of a formula, the carbon range and the M+2 hint, the co-elution correlation. The question and the route are `rawiso`; the JSON is `rawiso_json`.

use crate::error::Result;
use crate::ionpattern::{ion_counts, unit_pattern};
use crate::json::round;
use crate::rawiso::{Ask, Params};
use crate::rawxic::{self, judge, tallest, SMOOTH_HALF, VALLEY_PCT};

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

/// The region of M: the peak the integrator finds in the window, else the window itself; its scans, those of the sides and the flags it raises.
fn region_of(
    ask: &Ask,
    rt: &[f64],
    m: &[f64],
) -> (Region, Vec<usize>, Vec<usize>, Vec<&'static str>) {
    let hit = tallest(rt, m, ask.rt.0, ask.rt.1, SMOOTH_HALF, VALLEY_PCT).0;
    let span = hit.as_ref().map_or(ask.rt, |h| (h.start_rt, h.end_rt));
    let mut flags = Vec::new();
    if hit.is_none() {
        flags.push("no_peak_region");
    }
    let (inside, sides) = scans_of(rt, span, ask.rt, &ask.p.x);
    if sides.is_empty() {
        flags.push("no_background");
    }
    let left = sides.iter().filter(|&&i| rt[i] < ask.rt.0).count();
    let region = Region {
        source: if hit.is_some() { "peak" } else { "window" },
        lo: span.0,
        hi: span.1,
        apex: hit.as_ref().map(|h| h.rt),
        n: inside.len(),
        left,
        right: sides.len() - left,
    };
    (region, inside, sides, flags)
}

/// With a formula: its expected pattern and a verdict per ratio (a formula whose ion is not at `--mz` is flagged).
fn against_formula(ask: &Ask, f: &mut Found, fm: &str, ad: &str, has_quantum: bool) -> Result<()> {
    f.expected = expected_of(fm, ad)?;
    match &f.expected {
        Some(e) => {
            let (n0, n1, n2) = (f.ions[0].net, f.ions[1].net, f.ions[2].net);
            f.tests = Some((
                test(f.r1, e.e1, n1, n0, has_quantum, &ask.p),
                test(f.r2, e.e2, n2, n0, has_quantum, &ask.p),
            ));
            if (e.mz - ask.mz).abs() > 0.5 {
                f.flags.push("formula_mz_mismatch");
            }
        }
        None => f.flags.push("no_expected_m"),
    }
    Ok(())
}

/// Without a formula: the carbon range from R1 and the M+2 hint, evidence only.
fn without_formula(p: &Params, f: &mut Found, r1: Ratio) {
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

/// Everything one sample answered: the region, the three bins, the ratios and what they are compared with.
pub(crate) fn found_of(
    ask: &Ask,
    rt: &[f64],
    y: &[Vec<f64>],
    centres: [f64; 3],
    q: Option<f64>,
) -> Result<Found> {
    let j = judge(rt, &y[0], ask.rt, q, &ask.p.x);
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
    let (region, inside, sides, flags) = region_of(ask, rt, &y[0]);
    f.region = Some(region);
    f.flags.extend(flags);
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
        against_formula(ask, &mut f, fm, ad, q.is_some())?;
    } else if let Some(r1) = f.r1 {
        without_formula(&ask.p, &mut f, r1);
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
