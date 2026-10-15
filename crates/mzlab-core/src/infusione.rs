//! Is a file a direct infusion (syringe, FIA) with MSn on fixed precursors rather than an LC run? Decided from the rows of the `.mzidx`
//! alone, plus two hints (the LC part of the `.raw` method, the file name). The shape of the TIC is not used: measured, it does not separate.
//!
//! - constant RT (or one scan): `rt_costante`;
//! - B, fixed precursors: at most 64 distinct MSn keys, median scans per key >= 10, keys with >= 5 scans cover >= 95 % of the MSn scans;
//! - C, long blocks: >= 80 % of the MSn scans lie in consecutive runs of the same key at least 5 long (the MS1 scans are a key of their own).
//!
//! Infusion = constant RT, or B and C (`blocchi`), or B and a `.raw` method without LC (`metodo_senza_lc`), or B and a name with
//! `infus`/`fia`/`direct`/`syringe` (`nome`). A targeted LC run (PRM) has B but cycles its precursors (no C); a DDA run has neither.

use std::collections::HashMap;

use crate::error::Result;
use crate::idxread::{ReadAt, Scan};
use crate::json::Out;
use crate::mzmlhead::Mzml;

/// Why a file is an infusion, its deepest MSn level and its number of nominal MSn nodes (energies merged).
#[derive(Debug, Clone, PartialEq)]
pub struct Infusione {
    /// `rt_costante` | `blocchi` | `metodo_senza_lc` | `nome`
    pub motivo: &'static str,
    pub livelli: u8,
    pub nodi: usize,
}

const MAX_KEYS: usize = 64;
const MIN_MEDIAN: usize = 10;
const MIN_SCANS: usize = 5;
const MIN_COVER: f64 = 0.95;
const MIN_RUN: usize = 5;
const MIN_BLOCKS: f64 = 0.8;
const NAME_WORDS: [&str; 4] = ["infus", "fia", "direct", "syringe"];

/// Key of an MSn scan: the path of the filter at nominal m/z with activation and energy of each stage; without a path, the precursor
/// to 4 decimals. `nominal` drops activation and energy (the node of the tree with the energies merged).
fn key(s: &Scan, nominal: bool) -> String {
    let nom = |m: f64| m.round_ties_even() as i64;
    match s.path.as_deref() {
        Some(p) if !p.is_empty() => p
            .iter()
            .map(|(m, a, ce)| match nominal {
                true => nom(*m).to_string(),
                false => format!("{}@{}{}", nom(*m), a.to_lowercase(), ce),
            })
            .collect::<Vec<_>>()
            .join(">"),
        _ => match (nominal, s.precursor) {
            (true, Some(p)) => nom(p).to_string(),
            (false, Some(p)) => format!("{p:.4}"),
            _ => String::new(),
        },
    }
}

/// B: few keys, many scans each, the well-populated keys covering almost every MSn scan.
fn fixed_precursors(keys: &[String]) -> bool {
    let mut count: HashMap<&str, usize> = HashMap::new();
    for k in keys {
        *count.entry(k.as_str()).or_default() += 1;
    }
    if count.is_empty() || count.len() > MAX_KEYS {
        return false;
    }
    let mut n: Vec<usize> = count.values().copied().collect();
    crate::sortc::slice_ord(&mut n);
    let median = match n.len() % 2 {
        1 => n[n.len() / 2] as f64,
        _ => (n[n.len() / 2 - 1] + n[n.len() / 2]) as f64 / 2.0,
    };
    let covered: usize = n.iter().filter(|c| **c >= MIN_SCANS).sum();
    median >= MIN_MEDIAN as f64 && covered as f64 >= MIN_COVER * keys.len() as f64
}

/// C: share of the MSn scans that lie in runs (file order, MS1 = a key of its own) of the same key at least `MIN_RUN` long.
fn long_blocks(order: &[Option<&str>], n_msn: usize) -> bool {
    let (mut in_blocks, mut i) = (0usize, 0usize);
    while i < order.len() {
        let mut j = i + 1;
        while j < order.len() && order[j] == order[i] {
            j += 1;
        }
        if order[i].is_some() && j - i >= MIN_RUN {
            in_blocks += j - i;
        }
        i = j;
    }
    n_msn > 0 && in_blocks as f64 >= MIN_BLOCKS * n_msn as f64
}

/// The name of the file says infusion (lower-case base name).
fn name_says(nome: &str) -> bool {
    let base = nome.rsplit(['/', '\\']).next().unwrap_or(nome);
    let base = base.split('#').next().unwrap_or(base).to_lowercase();
    NAME_WORDS.iter().any(|w| base.contains(w))
}

/// An infusion recognised from the structure of its MSn scans: the file the Dati view shows as one item of kind `infusion`. A file of MS1 scans
/// only stays `full`, and one of constant RT (a product-ion scan of a low-resolution instrument) stays `ms2`.
pub fn con_msn(scans: &[Scan], lc: Option<bool>, nome: &str) -> bool {
    scans.iter().any(|s| s.level >= 2)
        && infusione(scans, lc, nome).is_some_and(|i| i.motivo != "rt_costante")
}

/// Whether the scans are an infusion (see the module), with `lc` = the `.raw` method has an LC part (`None`: unknown, e.g. an mzML).
pub fn infusione(scans: &[Scan], lc: Option<bool>, nome: &str) -> Option<Infusione> {
    if scans.is_empty() {
        return None;
    }
    let msn: Vec<&Scan> = scans.iter().filter(|s| s.level >= 2).collect();
    let mut nominal: Vec<String> = msn.iter().map(|s| key(s, true)).collect();
    let keys: Vec<String> = msn.iter().map(|s| key(s, false)).collect();
    let livelli = scans.iter().map(|s| s.level).max().unwrap_or(1);
    crate::sortc::slice_ord(&mut nominal);
    nominal.dedup();
    let nodi = nominal.len();
    let out = |motivo| {
        Some(Infusione {
            motivo,
            livelli,
            nodi,
        })
    };
    let (lo, hi) = scans
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), s| {
            (a.min(s.rt), b.max(s.rt))
        });
    if scans.len() == 1 || hi - lo < 1e-6 {
        return out("rt_costante");
    }
    if !fixed_precursors(&keys) {
        return None;
    }
    let mut it = keys.iter();
    let order: Vec<Option<&str>> = scans
        .iter()
        .map(|s| match s.level >= 2 {
            true => it.next().map(String::as_str),
            false => None,
        })
        .collect();
    if long_blocks(&order, msn.len()) {
        out("blocchi")
    } else if lc == Some(false) {
        out("metodo_senza_lc")
    } else if name_says(nome) {
        out("nome")
    } else {
        None
    }
}

/// `"msn":{motivo, livelli, nodi}` of an infusion, else `"msn":null` (`iteminfo`, `sniff`).
pub(crate) fn json(o: &mut Out, m: Option<&Infusione>) {
    o.raw(",\"msn\":");
    match m {
        Some(m) => o
            .raw("{\"motivo\":")
            .str(m.motivo)
            .raw(",\"livelli\":")
            .int(m.livelli as i64)
            .raw(",\"nodi\":")
            .int(m.nodi as i64)
            .raw("}"),
        None => o.raw("null"),
    };
}

/// Whether the `.raw` method an mzML carries (`metodo/1` written by the converter of a `.raw`) has an LC part: `None` when the file is
/// not a Thermo mzML, carries no such document or the `.raw` had no method.
pub(crate) fn raw_lc<M: ReadAt>(m: &mut Mzml<M>) -> Result<Option<bool>> {
    if !m.nce()? {
        return Ok(None);
    }
    let head = m.run_head(1 << 20)?;
    let Some(doc) = crate::metodo::from_userparam(&head) else {
        return Ok(None);
    };
    let Ok(v) = crate::json::parse(doc.as_bytes()) else {
        return Ok(None);
    };
    let raw = v
        .get("origine")
        .and_then(|o| o.get("formato"))
        .and_then(|f| f.str())
        == Some("thermo-raw");
    let no_method = doc.contains("\"meta.mancante\"");
    Ok((raw && !no_method).then(|| v.get("lc").is_some()))
}

#[cfg(test)]
mod tests;
