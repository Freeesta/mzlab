//! The verdict of `raw xic` (`rawxic`, `docs/agenti/raw-access.md`): the count statistics of a window against its sides (Currie levels, the one-ion
//! floor, an upper limit) and the rule that turns them into `peak`, `trace`, `nothing` or `no_data`. The numbers of the integrator are never changed here.

use crate::ionstat::{rd, QUANTUM_TOL};
use crate::rawxic::{tallest, Hit, Thresholds, SECONDS, SMOOTH_HALF, VALLEY_PCT};

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

/// The verdict of a window whose intensities are not ion counts: the S/N and the points of the integrator decide, against the same thresholds.
fn pipeline(hit: Option<Hit>, flags: Vec<&'static str>, t: &Thresholds) -> Judged {
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
    Judged {
        verdict,
        rule,
        hit,
        counts: None,
        flags,
    }
}

/// The rule that decided, with every value next to its threshold; the tests of a `peak` are all met, those of the others are shown with the ones of `trace`.
fn rule_of(c: &Counts, t: &Thresholds, peak: bool, verdict: &str) -> String {
    let ok_sn = c.sn_floor.is_some_and(|s| s >= t.min_sn);
    let sn = c.sn_floor.map_or("n/a (no peak found)".to_string(), |s| {
        format!("{} {} min_sn {}", rd(s), cmp(ok_sn), rd(t.min_sn))
    });
    let mut rule = format!(
        "N {} {} L_D {}; sn_floor {sn}; run {} {} min_points {}",
        rd(c.net),
        cmp(c.net >= c.l_d),
        rd(c.l_d),
        c.run,
        cmp(c.run >= t.min_points),
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
    rule + &format!(" -> {verdict}")
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
        return pipeline(hit, flags, t);
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
    let rule = rule_of(&c, t, peak, verdict);
    Judged {
        verdict,
        rule,
        hit,
        counts: Some(c),
        flags,
    }
}

/// The median step of the scan times, seconds (0 for fewer than two scans).
pub(crate) fn step_seconds(rt: &[f64]) -> f64 {
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
