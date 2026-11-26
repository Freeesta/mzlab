//! The JSON of `raw isotopes` (`rawiso`): the members of one sample (level, ion quantum, offset, thresholds) and its `result` object. Every key is always
//! present (a value that does not apply is null), so a reader never meets a missing key.

use crate::json::{round, Out};
use crate::rawiso::{Ask, Expected, Found, Ion, Ratio, Test};

/// `r` of a ratio at 4 decimals, null when there is none.
fn opt4(o: &mut Out, v: Option<f64>) {
    o.opt(v.map(|x| round(x, 4)));
}

/// The members of one sample: the level and its scans, the ion quantum, the m/z offset with its evidence and the thresholds in force.
pub(crate) fn members_json(
    ask: &Ask,
    n: usize,
    probe: &crate::rawinfo::Probe,
    off: Option<&crate::mzcal::Offset>,
    counts: bool,
) -> String {
    let mut o = Out::default();
    o.raw("\"ms_level\":")
        .int(i64::from(ask.level))
        .raw(",\"n_level_scans\":")
        .int(n as i64);
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
    let p = &ask.p;
    o.raw(",\"thresholds\":{");
    for (i, (k, v)) in [
        ("min_counts", p.min_counts),
        ("z_ok", p.z_ok),
        ("z_bad", p.z_bad),
        ("c13_pct", p.c13_pct),
        ("carbon_k", p.carbon_k),
        ("m2_ratio", p.m2_ratio),
        ("m2_k", p.m2_k),
        ("min_r", p.min_r),
        ("min_sn", p.x.min_sn),
        ("min_points", p.x.min_points as f64),
        ("alpha", p.x.alpha),
        ("side_min", p.x.side_min),
        ("gap_min", p.x.gap_min),
    ]
    .iter()
    .enumerate()
    {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").num(*v);
    }
    o.raw("}");
    o.0
}

fn ion_json(o: &mut Out, i: &Ion) {
    let r3 = |v: f64| round(v, 3);
    o.raw("{\"offset\":")
        .int(i64::from(i.off))
        .raw(",\"mz_bin\":")
        .num(i.mz)
        .raw(",\"window\":")
        .floats(&[i.mz - 0.5, i.mz + 0.5]);
    o.raw(",\"gross\":")
        .num(r3(i.gross))
        .raw(",\"net\":")
        .num(r3(i.net))
        .raw(",\"sigma\":")
        .opt(i.sigma.map(r3));
    o.raw(",\"background_per_scan\":")
        .num(r3(i.background))
        .raw("}");
}

fn ratio_json(o: &mut Out, r: Option<Ratio>) {
    match r {
        Some(r) => {
            o.raw("{\"r\":").num(round(r.r, 4)).raw(",\"sigma\":");
            opt4(o, r.sigma);
            o.raw("}")
        }
        None => o.raw("null"),
    };
}

fn expected_json(o: &mut Out, e: &Expected) {
    o.raw("{\"formula\":")
        .str(&e.formula)
        .raw(",\"adduct\":")
        .str(&e.adduct)
        .raw(",\"mz\":")
        .num(round(e.mz, 4));
    o.raw(",\"r1\":").num(round(e.e1, 4)).raw(",\"r2\":");
    o.num(round(e.e2, 4)).raw(",\"pattern\":[");
    for (i, (off, mz, rel)) in e.pattern.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"off\":")
            .int(i64::from(*off))
            .raw(",\"mz\":")
            .num(round(*mz, 4))
            .raw(",\"abundance_pct\":")
            .num(round(*rel, 4))
            .raw("}");
    }
    o.raw("]}");
}

fn test_json(o: &mut Out, t: &Test) {
    o.raw("{\"z\":").opt(t.z.map(|z| round(z, 3)));
    o.raw(",\"verdict\":")
        .str(t.verdict)
        .raw(",\"reason\":")
        .str(t.reason)
        .raw("}");
}

/// The `result` object of one sample.
pub(crate) fn result_json(ask: &Ask, f: &Found) -> String {
    let mut o = Out::default();
    o.raw("{\"mz\":")
        .num(ask.mz)
        .raw(",\"rt_window\":")
        .floats(&[ask.rt.0, ask.rt.1]);
    o.raw(",\"m\":{\"verdict\":")
        .str(f.m_verdict)
        .raw(",\"rule\":")
        .str(&f.m_rule)
        .raw("},\"region\":");
    match &f.region {
        Some(r) => {
            o.raw("{\"source\":")
                .str(r.source)
                .raw(",\"start_rt\":")
                .num(r.lo)
                .raw(",\"end_rt\":")
                .num(r.hi);
            o.raw(",\"apex_rt\":").opt(r.apex);
            o.raw(",\"n_scans\":")
                .int(r.n as i64)
                .raw(",\"n_left_scans\":")
                .int(r.left as i64)
                .raw(",\"n_right_scans\":")
                .int(r.right as i64)
                .raw("}")
        }
        None => o.raw("null"),
    };
    o.raw(",\"ions\":[");
    for (i, x) in f.ions.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        ion_json(&mut o, x);
    }
    o.raw("],\"observed\":");
    if f.ions.is_empty() {
        o.raw("null");
    } else {
        o.raw("{\"r1\":");
        ratio_json(&mut o, f.r1);
        o.raw(",\"r2\":");
        ratio_json(&mut o, f.r2);
        o.raw("}");
    }
    o.raw(",\"expected\":");
    match &f.expected {
        Some(e) => expected_json(&mut o, e),
        None => {
            o.raw("null");
        }
    }
    o.raw(",\"tests\":");
    match &f.tests {
        Some((a, b)) => {
            o.raw("{\"r1\":");
            test_json(&mut o, a);
            o.raw(",\"r2\":");
            test_json(&mut o, b);
            o.raw("}")
        }
        None => o.raw("null"),
    };
    o.raw(",\"carbon_range\":");
    match f.carbon {
        Some((best, lo, hi)) => o
            .raw("{\"best\":")
            .num(best)
            .raw(",\"min\":")
            .num(lo)
            .raw(",\"max\":")
            .num(hi)
            .raw(",\"c13_pct\":")
            .num(ask.p.c13_pct)
            .raw(",\"k_sigma\":")
            .num(ask.p.carbon_k)
            .raw("}"),
        None => o.raw("null"),
    };
    o.raw(",\"m2_hint\":");
    match &f.m2_hint {
        Some((h, rule)) => o
            .raw("{\"hint\":")
            .str(h)
            .raw(",\"rule\":")
            .str(rule)
            .raw("}"),
        None => o.raw("null"),
    };
    o.raw(",\"interference\":");
    match f.interference {
        Some((r1, r2, n)) => {
            o.raw("{\"r_m1\":");
            opt4(&mut o, r1);
            o.raw(",\"r_m2\":");
            opt4(&mut o, r2);
            o.raw(",\"n_points\":").int(n as i64).raw(",\"min_r\":");
            o.num(ask.p.min_r).raw(",\"possible\":");
            match r1 {
                Some(r) => o.raw(if r < ask.p.min_r { "true" } else { "false" }),
                None => o.raw("null"),
            };
            o.raw("}")
        }
        None => o.raw("null"),
    };
    o.raw(",\"flags\":[");
    for (i, fl) in f.flags.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(fl);
    }
    o.raw("]}");
    o.0
}
