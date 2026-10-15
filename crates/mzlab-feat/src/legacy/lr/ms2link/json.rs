//! `ms2_links/1`, as `docs/studi/TABUI_MS2_LR.md` §5 writes it.

use mzlab_core::json::{round, Out};

use super::{Links, PREC_TOL};

fn ids(o: &mut Out, v: &[i64]) {
    o.raw("[");
    for (i, x) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(*x);
    }
    o.raw("]");
}

fn index(o: &mut Out, v: Option<usize>) {
    match v {
        Some(i) => o.int(i as i64),
        None => o.raw("null"),
    };
}

/// The object `ms2_links/1`.
pub fn json(o: &mut Out, l: &Links) {
    o.raw("{\"schema\":\"ms2_links/1\",\"prec_tol\":")
        .num(PREC_TOL)
        .raw(",\"rt_tol\":")
        .num(l.rt_tol);
    o.raw(",\"experiments\":[");
    for (i, e) in l.exps.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"file\":")
            .str(&e.file);
        o.raw(",\"time\":")
            .opt(e.time)
            .raw(",\"polarity\":")
            .int(e.polarity as i64);
        o.raw(",\"precursor\":")
            .num(e.precursor)
            .raw(",\"ce\":")
            .opt(e.ce);
        o.raw(",\"scans\":")
            .int(e.scans.len() as i64)
            .raw(",\"rt\":");
        match (e.rt.first(), e.rt.last()) {
            (Some(a), Some(b)) => o
                .raw("[")
                .num(round(*a, 3))
                .raw(",")
                .num(round(*b, 3))
                .raw("]"),
            _ => o.raw("null"),
        };
        o.raw(",\"drift\":")
            .num(round(e.drift, 3))
            .raw(",\"peaks\":[");
        for (j, p) in e.peaks.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" })
                .raw("{\"apex\":")
                .num(round(p.apex, 3));
            o.raw(",\"lo\":")
                .num(round(p.lo, 3))
                .raw(",\"hi\":")
                .num(round(p.hi, 3));
            o.raw(",\"sn\":")
                .num(round(p.sn, 1))
                .raw(",\"scans\":")
                .int(p.scans as i64)
                .raw("}");
        }
        o.raw("]}");
    }
    o.raw("],\"links\":[");
    for (i, k) in l.links.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"exp\":")
            .int(k.exp as i64);
        o.raw(",\"peak\":")
            .int(k.peak as i64)
            .raw(",\"row\":")
            .int(k.row);
        o.raw(",\"state\":")
            .str(k.state.name())
            .raw(",\"dmz\":")
            .num(k.dmz);
        o.raw(",\"drt\":").num(k.drt).raw(",\"alternatives\":");
        ids(o, &k.alternatives);
        o.raw(",\"why\":").str(&k.why).raw("}");
    }
    o.raw("],\"unmatched\":[");
    for (i, u) in l.unmatched.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"exp\":");
        index(o, u.exp);
        o.raw(",\"peak\":");
        index(o, u.peak);
        o.raw(",\"reason\":").str(u.reason).raw(",\"rows\":");
        ids(o, &u.rows);
        o.raw(",\"why\":").str(&u.why).raw("}");
    }
    o.raw("],\"parent\":{\"has_ms2\":")
        .raw(if l.parent_has_ms2 { "true" } else { "false" })
        .raw("}}");
}
