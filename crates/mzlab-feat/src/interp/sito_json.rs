//! The `site` and `drawing` keys of the block `interp/1` of a row (see `sito.rs` and `disegno.rs`).

use mzlab_core::json::{round, Out};

use super::build::RowInterp;
use super::disegno::Drawing;
use super::sito::SiteReport;

fn atoms(o: &mut Out, a: &[usize]) {
    o.raw("[");
    for (i, x) in a.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(*x as i64);
    }
    o.raw("]");
}

fn strs(o: &mut Out, a: &[&str]) {
    o.raw("[");
    for (i, x) in a.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(x);
    }
    o.raw("]");
}

fn votes(o: &mut Out, s: &SiteReport) {
    o.raw("[");
    for (i, v) in s.votes.iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .int(v.id as i64)
            .raw(",\"kind\":")
            .str(&v.kind)
            .raw(",\"status\":")
            .str(v.status);
        o.raw(",\"atoms\":");
        atoms(o, &v.atoms);
        o.raw(",\"group\":");
        match &v.group {
            Some(g) => o.str(g),
            None => o.raw("null"),
        };
        o.raw(",\"n_support\":")
            .int(v.n_support as i64)
            .raw(",\"n_against\":")
            .int(v.n_against as i64);
        o.raw(",\"support\":").num(round(v.support, 3)).raw("}");
    }
    o.raw("]");
}

fn site(o: &mut Out, s: &SiteReport) {
    o.raw("{\"transform\":")
        .str(&s.transform)
        .raw(",\"confidence\":")
        .str(s.confidence)
        .raw(",\"reasons\":");
    strs(o, &s.reasons);
    o.raw(",\"source\":")
        .str(s.source)
        .raw(",\"informative\":")
        .int(s.informative as i64);
    o.raw(",\"margin\":");
    match s.margin {
        Some(m) => o.num(round(m, 2)),
        None => o.raw("null"),
    };
    o.raw(",\"classes\":");
    votes(o, s);
    o.raw(",\"region\":");
    match &s.region {
        Some((n, a)) => {
            o.raw("{\"name\":").str(n).raw(",\"atoms\":");
            atoms(o, a);
            o.raw("}")
        }
        None => o.raw("null"),
    };
    o.raw(",\"retained\":");
    match &s.retained {
        Some((a, d)) => {
            o.raw("{\"atoms\":");
            atoms(o, a);
            o.raw(",\"text\":").str(d).raw("}")
        }
        None => o.raw("null"),
    };
    o.raw(",\"proposal\":{\"sublevel\":")
        .str(s.sublevel)
        .raw(",\"why\":")
        .str(s.why)
        .raw("}}");
}

fn drawing(o: &mut Out, d: &Drawing) {
    o.raw("{\"mode\":").str(d.mode).raw(",\"atoms\":");
    atoms(o, &d.atoms);
    o.raw(",\"ends\":");
    atoms(o, &d.ends);
    o.raw(",\"lost\":");
    atoms(o, &d.lost);
    o.raw(",\"added\":");
    match d.added {
        Some(e) => o.str(e),
        None => o.raw("null"),
    };
    o.raw(",\"kind\":");
    match &d.kind {
        Some(k) => o.str(k),
        None => o.raw("null"),
    };
    o.raw(",\"delta\":")
        .str(&d.delta)
        .raw(",\"label\":")
        .str(&d.label)
        .raw(",\"uncertainty\":")
        .str(d.uncertainty)
        .raw(",\"fallback\":");
    match d.fallback {
        Some(f) => o.str(f),
        None => o.raw("null"),
    };
    o.raw("}");
}

/// `,"site":{..},"drawing":{..}` of a row (both null when the row has none).
pub fn write(o: &mut Out, r: &RowInterp) {
    o.raw(",\"site\":");
    match &r.site {
        Some(s) => site(o, s),
        None => {
            o.raw("null");
        }
    };
    o.raw(",\"drawing\":");
    match &r.drawing {
        Some(d) => drawing(o, d),
        None => {
            o.raw("null");
        }
    };
}
