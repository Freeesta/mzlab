//! The optional `prior` of `posiz/1`: the condensed Fukui f(−) of the parent (from its `reatt/1`, `mzlab-chem`) on every site of the change, with a rank
//! among the sites. A prediction («Prev.»), never evidence: it does not touch the classes, their order, their status or the proposal.
//! Only for changes made by an electrophilic attack: `+O` (•OH); `+Cl-H` and `+Br-H` carry a caveat (not validated); any other change has none.

use crate::sortc::SortC;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};

use super::siti::Site;

/// Two symmetry classes whose f(−) differ by less than this share a rank.
pub const TIE: f64 = 0.005;
/// Caveat of the halogenations: the same f(−), not checked against experiments here.
pub const HALOGEN_CAVEAT: &str = "electrophilic substitution, not validated here";

/// The f(−) of one site atom. `rank` is 1 for the most reactive symmetry class (dense ranks, ties within `TIE` of the first of the group).
#[derive(Debug, Clone, PartialEq)]
pub struct Prior {
    pub atom: usize,
    pub value: f64,
    pub rank: u32,
    pub method: String,
    pub caveat: Option<&'static str>,
}

/// `None` when the change has no prior, `Some(caveat)` otherwise.
fn caveat_of(sites: &[Site]) -> Option<Option<&'static str>> {
    let k = sites.first()?.kind;
    match k {
        "aromatic_oh" | "aliphatic_oh" | "n_oxide" | "s_oxide" => Some(None),
        "aromatic_cl" | "aromatic_br" => Some(Some(HALOGEN_CAVEAT)),
        _ => None,
    }
}

/// f(−) per heavy atom (index = `heavy_index`) and the method, from a `reatt/1`; `n` heavy atoms are expected.
fn read_reatt(text: &str, n: usize) -> Result<(Vec<f64>, String)> {
    let bad = |why: &str| Error::new("error.posiz.prior").with("reason", why);
    let v = json::parse(text.as_bytes()).map_err(|_| bad("json"))?;
    if v.get("format").and_then(Value::str) != Some("reatt") {
        return Err(bad("format"));
    }
    let mut f = vec![f64::NAN; n];
    for a in v.get("atoms").map(Value::arr).unwrap_or(&[]) {
        let Some(h) = a.get("heavy_index").and_then(Value::f64) else {
            continue;
        };
        let h = h as usize;
        let fm = a.get("f_minus").and_then(Value::f64);
        match (f.get_mut(h), fm) {
            (Some(slot), Some(x)) if x.is_finite() => *slot = x,
            _ => return Err(bad("atoms")),
        }
    }
    if f.iter().any(|x| x.is_nan()) {
        return Err(bad("atoms"));
    }
    let m = v.get("method").and_then(Value::str).unwrap_or("?");
    let method = match v.get("solvent").and_then(Value::str) {
        Some("water") => format!("{m}, implicit water"),
        Some("none") => format!("{m}, gas phase"),
        _ => m.to_string(),
    };
    Ok((f, method))
}

/// Dense ranks of the symmetry classes of the sites, ordered by their mean f(−) (highest first): a class within `TIE` of the first class of the group
/// takes its rank.
fn class_ranks(sites: &[Site], f: &[f64]) -> Vec<(u32, u32)> {
    let mut means: Vec<(u32, f64)> = Vec::new();
    for s in sites {
        if means.iter().any(|(c, _)| *c == s.class) {
            continue;
        }
        let v: Vec<f64> = sites
            .iter()
            .filter(|t| t.class == s.class)
            .map(|t| f[t.atom])
            .collect();
        means.push((s.class, v.iter().sum::<f64>() / v.len() as f64));
    }
    means.sortc_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let (mut rank, mut lead) = (0u32, f64::INFINITY);
    means
        .iter()
        .map(|&(c, m)| {
            if lead - m >= TIE {
                rank += 1;
                lead = m;
            }
            (c, rank)
        })
        .collect()
}

/// The `prior` of the sites of a change from the `reatt/1` of the parent (`n` heavy atoms); empty when the change has none.
/// Errors: `error.posiz.prior` with `reason` (`json`, `format`, `atoms`: a heavy atom without f(−) or an index outside the parent).
pub fn prior_of(sites: &[Site], n: usize, reatt: &str) -> Result<Vec<Prior>> {
    let (f, method) = read_reatt(reatt, n)?;
    let Some(caveat) = caveat_of(sites) else {
        return Ok(Vec::new());
    };
    let ranks = class_ranks(sites, &f);
    let mut out: Vec<Prior> = Vec::new();
    for s in sites {
        if out.iter().any(|p| p.atom == s.atom) {
            continue;
        }
        let rank = ranks.iter().find(|(c, _)| *c == s.class).map_or(0, |r| r.1);
        out.push(Prior {
            atom: s.atom,
            value: f[s.atom],
            rank,
            method: method.clone(),
            caveat,
        });
    }
    Ok(out)
}

/// `,"prior":[{atom, value, rank, quantity:"f_minus", method, note:"Prev.", caveat?}, ..]`
pub(super) fn write_prior(o: &mut Out, v: &[Prior]) {
    o.raw(",\"prior\":[");
    for (i, p) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"atom\":")
            .int(p.atom as i64)
            .raw(",\"value\":")
            .num(round(p.value, 4))
            .raw(",\"rank\":")
            .int(p.rank as i64)
            .raw(",\"quantity\":\"f_minus\",\"method\":")
            .str(&p.method)
            .raw(",\"note\":\"Prev.\"");
        if let Some(c) = p.caveat {
            o.raw(",\"caveat\":").str(c);
        }
        o.raw("}");
    }
    o.raw("]");
}
