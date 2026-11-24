//! `companions/1` (`docs/schemi/companions-1.json`): the parameters used, the counts, one entry per row (`id`, `companion_of`, `companion_via`), the edges
//! with their evidence and the `weak` candidates (a mass relation that fits but whose correlation is below `min_corr`, or was not tested).

use super::{Edge, Found, Kind, Params};
use crate::table::Aligned;
use mzlab_core::json::{round, Out};

/// Weak candidates listed at most, the strongest first; `weak_dropped` counts the others.
pub const WEAK_CAP: usize = 200;
/// 13C/12C of nature (1.08 %): carbons implied by an M+1/M ratio, an upper bound when N, S or Si add to it.
const C13: f64 = 0.0108;
/// M+2/M above which Cl, Br or S is a reading to look at.
const HEAVY_M2: f64 = 0.25;

fn opt(o: &mut Out, v: f64, nd: usize) {
    if v.is_finite() {
        o.num(round(v, nd));
    } else {
        o.raw("null");
    }
}

fn list(o: &mut Out, key: &str, v: impl Iterator<Item = f64>) {
    o.raw(",\"").raw(key).raw("\":[");
    for (k, d) in v.enumerate() {
        o.raw(if k > 0 { "," } else { "" }).num(d);
    }
    o.raw("]");
}

fn edge(o: &mut Out, t: &Aligned, t_e: &Edge) {
    let e = t_e;
    o.raw("{\"from\":").int(i64::from(t.id(e.a)));
    o.raw(",\"to\":").int(i64::from(t.id(e.b)));
    o.raw(",\"kind\":")
        .str(e.kind.name())
        .raw(",\"relations\":[");
    for (k, r) in e.relations.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(r);
    }
    o.raw("],\"delta_da\":");
    opt(o, t.mz[e.b] - t.mz[e.a], 3);
    o.raw(",\"parent\":");
    match e.parent {
        Some(p) => o.int(i64::from(t.id(p))).raw(",\"rule\":").str(e.rule),
        None => o.raw("null,\"rule\":null"),
    };
    o.raw(",\"r\":");
    opt(o, e.ev.r, 4);
    o.raw(",\"n_files\":").int(e.ev.n as i64);
    o.raw(",\"rt_offset\":");
    opt(o, e.ev.rt_offset, 4);
    o.raw(",\"apex_shift\":");
    opt(o, e.ev.apex_shift, 4);
    o.raw(",\"ratio\":{\"median\":");
    opt(o, e.ev.ratio_med, 4);
    o.raw(",\"lo\":");
    opt(o, e.ev.ratio_lo, 4);
    o.raw(",\"hi\":");
    opt(o, e.ev.ratio_hi, 4);
    o.raw("},\"carbons\":");
    let c = e.kind == Kind::Isotope && e.relations[0] == "isotope_p1" && e.ev.ratio_lo.is_finite();
    if c {
        let (lo, hi) = ((e.ev.ratio_lo / C13).floor(), (e.ev.ratio_hi / C13).ceil());
        o.raw("[").num(lo).raw(",").num(hi).raw("]");
    } else {
        o.raw("null");
    }
    o.raw(",\"hint\":");
    let heavy =
        e.kind == Kind::Isotope && e.relations[0] == "isotope_p2" && e.ev.ratio_med >= HEAVY_M2;
    o.raw(if heavy { "\"cl_br_s\"" } else { "null" }).raw("}");
}

fn edges(o: &mut Out, t: &Aligned, key: &str, v: &[Edge], cap: usize) {
    o.raw(",\"").raw(key).raw("\":[");
    for (k, e) in v.iter().take(cap).enumerate() {
        o.raw(if k > 0 { "," } else { "" });
        edge(o, t, e);
    }
    o.raw("]");
}

/// The `companions/1` text of `f` found on `t` under `p`.
pub fn to_json(t: &Aligned, p: &Params, f: &Found) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":\"companions/1\",\"rows\":")
        .int(t.len() as i64);
    o.raw(",\"files\":").int(t.n_files as i64);
    o.raw(",\"params\":{\"rt_tol\":").num(p.rt_tol);
    o.raw(",\"min_corr\":").num(p.min_corr);
    o.raw(",\"weak_corr\":").num(p.weak_corr);
    o.raw(",\"min_corr_samples\":")
        .int(p.min_corr_samples as i64);
    o.raw(",\"da_tol\":").num(p.da_tol);
    o.raw(",\"max_edges\":").int(p.max_edges as i64);
    list(&mut o, "adducts", p.adducts.iter().map(|r| r.delta));
    list(&mut o, "losses", p.losses.iter().map(|r| r.delta));
    let c = &f.counts;
    o.raw("},\"counts\":{\"pairs\":").int(c.pairs as i64);
    o.raw(",\"isotope\":").int(c.isotope as i64);
    o.raw(",\"relation\":").int(c.relation as i64);
    o.raw(",\"coelution\":").int(c.coelution as i64);
    o.raw(",\"weak\":").int(c.weak as i64);
    o.raw(",\"untested\":").int(c.untested as i64);
    o.raw(",\"implied\":").int(c.implied as i64);
    o.raw(",\"dropped\":").int(c.dropped as i64);
    o.raw("},\"id\":[");
    for r in 0..t.len() {
        o.raw(if r > 0 { "," } else { "" }).int(i64::from(t.id(r)));
    }
    o.raw("],\"companion_of\":[");
    for (r, c) in f.companion_of.iter().enumerate() {
        o.raw(if r > 0 { "," } else { "" });
        match c {
            Some((par, _)) => o.int(i64::from(t.id(*par))),
            None => o.raw("null"),
        };
    }
    o.raw("],\"companion_via\":[");
    for (r, c) in f.companion_of.iter().enumerate() {
        o.raw(if r > 0 { "," } else { "" });
        match c {
            Some((_, k)) => o.str(k.name()),
            None => o.raw("null"),
        };
    }
    o.raw("]");
    edges(&mut o, t, "edges", &f.edges, usize::MAX);
    edges(&mut o, t, "weak", &f.weak, WEAK_CAP);
    o.raw(",\"weak_dropped\":")
        .int(f.weak.len().saturating_sub(WEAK_CAP) as i64);
    o.raw(",\"weak_truncated\":")
        .raw(if f.weak.len() > WEAK_CAP {
            "true"
        } else {
            "false"
        })
        .raw("}");
    o.0
}
