//! `rete/1` as JSON, and the words of an arc: the change of the ion formula and the reason, in English (Tabui is English only).

use mzlab_core::json::{round, Out};

use super::{Ms2Share, Rete};
use crate::hypo::Counts;

pub const SCHEMA: &str = "rete/1";

/// The change from `a` to `b` as `+O`, `−2H`, `+O−2H`: gains first, with the true minus (U+2212).
pub(super) fn delta_label(a: &Counts, b: &Counts) -> String {
    let get = |c: &Counts, e: &str| c.get(e).copied().unwrap_or(0);
    let mut elements: Vec<&String> = a.keys().chain(b.keys()).collect();
    elements.sort();
    elements.dedup();
    let diff: Vec<(&String, i64)> = elements
        .into_iter()
        .map(|e| (e, get(b, e) - get(a, e)))
        .filter(|d| d.1 != 0)
        .collect();
    let mut out = String::new();
    for (sign, gain) in [('+', true), ('\u{2212}', false)] {
        for (e, n) in diff.iter().filter(|d| (d.1 > 0) == gain) {
            out.push(sign);
            if n.abs() != 1 {
                out.push_str(&n.abs().to_string());
            }
            out.push_str(e);
        }
    }
    out
}

fn signed(v: f64) -> String {
    let t = format!("{:.3}", v.abs());
    format!("{}{t}", if v < 0.0 { '\u{2212}' } else { '+' })
}

/// The reason of an arc: the evidence of the MS2 when there is one, then the order in time.
pub(super) fn reason(
    delta: &str,
    rule: &str,
    kinetic: &str,
    ms2: Option<&Ms2Share>,
    dm: f64,
) -> String {
    let mut s = match ms2 {
        Some(m) if m.equal.len() + m.shifted.len() > 0 => {
            let total = m.equal.len() + m.shifted.len();
            format!(
                "shares {total} fragment{}, {} shifted by {}",
                if total == 1 { "" } else { "s" },
                m.shifted.len(),
                signed(dm)
            )
        }
        Some(_) => "shares no fragment with the parent".to_string(),
        None => format!("mass difference {delta} of rule {rule}"),
    };
    s.push_str(match kinetic {
        "consistent" => "; the source peaks first",
        "against" => "; the source peaks later than the product",
        _ => "; no time course to order them",
    });
    s
}

impl Rete {
    /// The network as `rete/1`. Numbers are rounded as in the other views (m/z 4 decimals, RT 2, ppm 2, Δm 5).
    pub fn to_json(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"schema\":\"rete/1\",\"params\":{\"ppm\":")
            .num(self.params.ppm)
            .raw(",\"min_fragments\":")
            .int(self.params.min_fragments as i64)
            .raw("},\"parent\":")
            .int(i64::from(self.parent))
            .raw(",\"columns\":")
            .int(i64::from(self.columns()))
            .raw(",\"nodes\":[");
        for (i, n) in self.nodes.iter().enumerate() {
            o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
                .int(i64::from(n.row.id));
            o.raw(",\"row\":").int(n.row.row as i64);
            o.raw(",\"mz\":")
                .num(round(n.row.mz, 4))
                .raw(",\"rt\":")
                .num(round(n.row.rt, 2));
            o.raw(",\"neutral_formula\":").str(&n.row.neutral_formula);
            o.raw(",\"ion_formula\":").str(&n.row.ion_formula);
            o.raw(",\"kinship\":").int(i64::from(n.kinship));
            o.raw(",\"step\":")
                .int(i64::from(n.step))
                .raw(",\"pos\":")
                .int(i64::from(n.pos))
                .raw("}");
        }
        o.raw("],\"arcs\":[");
        for (i, a) in self.arcs.iter().enumerate() {
            o.raw(if i > 0 { ",{\"from\":" } else { "{\"from\":" })
                .int(i64::from(a.from));
            o.raw(",\"to\":")
                .int(i64::from(a.to))
                .raw(",\"rule\":")
                .str(&a.rule)
                .raw(",\"scope\":")
                .str(a.scope);
            o.raw(",\"delta\":")
                .str(&a.delta)
                .raw(",\"dm\":")
                .num(round(a.dm, 5));
            o.raw(",\"ppm\":")
                .num(round(a.ppm, 2) + 0.0)
                .raw(",\"delta_rt\":")
                .num(round(a.delta_rt, 2));
            o.raw(",\"kinetic\":").str(a.kinetic).raw(",\"ms2\":");
            match &a.ms2 {
                Some(m) => {
                    o.raw("{\"equal\":")
                        .rounded(&m.equal, 4)
                        .raw(",\"shifted\":")
                        .rounded(&m.shifted, 4);
                    o.raw(",\"n_equal\":")
                        .int(m.equal.len() as i64)
                        .raw(",\"n_shifted\":")
                        .int(m.shifted.len() as i64)
                        .raw("}");
                }
                None => {
                    o.raw("null");
                }
            }
            o.raw(",\"reason\":").str(&a.reason).raw("}");
        }
        o.raw("],\"n_arcs\":").int(self.arcs.len() as i64).raw("}");
        o.0
    }
}
