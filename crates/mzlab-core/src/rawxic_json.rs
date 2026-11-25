//! One row of `raw xic` as JSON (`rawxic`): the mass, its windows, the verdict with its rule and every number behind it.

use crate::json::{round, Out};
use crate::mzcal::Offset;
use crate::rawinfo::{quantum_json, Probe};
use crate::rawxic::{z_of, Ask, Judged, SMOOTH_HALF, SN_ESTIMATOR, VALLEY_PCT};

fn opt3(o: &mut Out, v: Option<f64>) {
    o.opt(v.map(|x| round(x, 3)));
}

/// One row (JSON): the mass, its windows, the verdict with its rule and every number behind it.
pub(crate) fn row_json(
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

/// The members of one sample: the level and its scans, the ion quantum, the m/z offset with its evidence and the thresholds in force.
pub(crate) fn members_json(
    ask: &Ask,
    n: usize,
    dt_s: f64,
    probe: &Probe,
    off: Option<&Offset>,
) -> String {
    let mut o = Out::default();
    o.raw("\"ms_level\":")
        .int(i64::from(ask.level))
        .raw(",\"n_scans\":")
        .int(n as i64);
    o.raw(",\"scan_time_s\":").num(round(dt_s, 4));
    o.raw(",\"ion_quantum\":").raw(&quantum_json(Some(probe)));
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
    o.0
}
