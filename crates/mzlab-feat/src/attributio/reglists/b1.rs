// Ported from Attributio reglists/match.py.
//! B1: the identities of a campaign against the entries of the regulatory lists. The comparison is by full InChIKey; the first block (14 characters) only as a
//! flagged fallback. Never by name, never by formula. The inputs are plain rows (identities, their structures, list entries): where they come from is the
//! caller's business.

use super::catalog;
use crate::attributio::tox::snapshot::MembershipRow;
use std::collections::{BTreeMap, BTreeSet};

const PROTON: f64 = 1.007276;
pub const MATCH_FULL: &str = "InChIKey";
pub const MATCH_BLOCK: &str = "first block";

/// One identity of the campaign (one row per common name) with its presence at each screened site.
#[derive(Clone, Debug, Default)]
pub struct IdentityIn {
    pub nome_comune: String,
    pub group: String,
    pub best_level: i64,
    pub in_site: Vec<bool>,
}

/// One structure of an identity (the registry rules give `first_block`, the full key and its origin).
#[derive(Clone, Debug, Default)]
pub struct StructRow {
    pub nome_comune: String,
    pub identity: String,
    pub first_block: Option<String>,
    pub inchikey: Option<String>,
    pub key_origin: String,
    pub ik_basis: String,
}

/// The screening set-up: sites, sub-matrices strictly inside them (name, indexes into `sites`) and the MS1 window of the instrument.
#[derive(Clone, Debug, Default)]
pub struct B1Config {
    pub sites: Vec<String>,
    pub submatrices: Vec<(String, Vec<usize>)>,
    pub window: Option<(f64, f64)>,
}

/// An identity with the columns B1 adds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IdentityOut {
    pub nome_comune: String,
    pub group: String,
    pub best_level: i64,
    pub first_blocks: Vec<String>,
    pub inchikeys: Option<String>,
    pub key_origin: Option<String>,
    pub structure_basis: Option<String>,
    pub split: bool,
    pub merge: String,
    pub sites: String,
    pub in_sub: Vec<bool>,
    pub all_sites: bool,
    /// One label per list (empty = not in the list), in `B1::lists` order.
    pub labels: Vec<String>,
    pub n_lists: usize,
    pub lists: String,
}

/// An identity × structure × list-entry join row.
#[derive(Clone, Debug, PartialEq)]
pub struct LongRow {
    pub nome_comune: String,
    pub identity: String,
    pub first_block: String,
    pub list_id: String,
    pub list_name: String,
    pub list_attribute: String,
    pub list_inchikey: String,
    pub matched: &'static str,
}

/// X of Y per list.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SummaryRow {
    pub list_id: String,
    pub group: String,
    pub list: String,
    pub question: String,
    pub entries: usize,
    pub y_true_in_window: usize,
    pub y_true_doubtful_in_window: usize,
    pub x_identities: usize,
    pub x_anthropogenic: usize,
    pub x_natural: usize,
    pub x_by_inchikey: usize,
    pub x_first_block_only: usize,
    pub in_sub: Vec<usize>,
    pub all_sites: usize,
    pub matched: String,
}

/// The result of B1.
#[derive(Clone, Debug, Default)]
pub struct B1 {
    pub lists: Vec<String>,
    pub ident: Vec<IdentityOut>,
    pub long: Vec<LongRow>,
    pub summary: Vec<SummaryRow>,
    pub coverage: Vec<(&'static str, usize)>,
}

/// `[M+H]+` or `[M-H]-` inside the MS1 scan range (no window: always true; an unreadable mass is false).
pub fn in_window(mass: &str, window: Option<(f64, f64)>) -> bool {
    let Ok(m) = mass.trim().parse::<f64>() else {
        return false;
    };
    let Some((lo, hi)) = window else { return true };
    (lo..=hi).contains(&(m + PROTON)) || (lo..=hi).contains(&(m - PROTON))
}

fn sorted_join<'a>(it: impl Iterator<Item = &'a str>, sep: &str) -> String {
    it.collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(sep)
}

/// B1. `lists` is the set to compare (`None`: every list of the table); it is put in catalogue order.
type Owners<'a> = BTreeMap<&'a str, BTreeSet<&'a str>>;

/// Inner join of the structures with a first block and the list entries on that block, in the order of the structures then of the entries.
fn join_long(structs: &[&StructRow], mem: &[&MembershipRow]) -> Vec<LongRow> {
    let mut long = Vec::new();
    for s in structs.iter().filter(|s| s.first_block.is_some()) {
        let fb = s.first_block.as_deref().unwrap_or("");
        for m in mem.iter().filter(|m| m.first_block == fb) {
            let full = s.inchikey.as_deref().unwrap_or("") == m.inchikey;
            long.push(LongRow {
                nome_comune: s.nome_comune.clone(),
                identity: s.identity.clone(),
                first_block: fb.to_string(),
                list_id: m.list_id.clone(),
                list_name: m.name.clone(),
                list_attribute: m.list_attribute.clone(),
                list_inchikey: m.inchikey.clone(),
                matched: if full { MATCH_FULL } else { MATCH_BLOCK },
            });
        }
    }
    long
}

/// The identities that own each first block (more than one means a MERGE).
fn block_owners<'a>(with_block: &[&&'a StructRow]) -> Owners<'a> {
    let mut owners: Owners = BTreeMap::new();
    for s in with_block {
        owners
            .entry(s.first_block.as_deref().unwrap_or(""))
            .or_default()
            .insert(s.nome_comune.as_str());
    }
    owners
}

/// The per-identity row before the list labels: sites, structure keys and SPLIT / MERGE flags.
fn identity_out(
    i: &IdentityIn,
    mine: &[&&&StructRow],
    owners: &Owners,
    cfg: &B1Config,
) -> IdentityOut {
    let blocks: Vec<String> = mine
        .iter()
        .map(|s| s.first_block.clone().unwrap_or_default())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let merged = blocks
        .iter()
        .any(|b| owners.get(b.as_str()).is_some_and(|o| o.len() > 1));
    let merge = if merged {
        blocks
            .iter()
            .flat_map(|b| owners.get(b.as_str()).into_iter().flatten().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join("; ")
    } else {
        String::new()
    };
    let present = |k: usize| i.in_site.get(k).copied().unwrap_or(false);
    let mut out = IdentityOut {
        nome_comune: i.nome_comune.clone(),
        group: i.group.clone(),
        best_level: i.best_level,
        split: blocks.len() > 1,
        merge,
        sites: cfg
            .sites
            .iter()
            .zip(&i.in_site)
            .filter(|(_, p)| **p)
            .map(|(s, _)| s.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        in_sub: cfg
            .submatrices
            .iter()
            .map(|(_, ix)| ix.iter().any(|k| present(*k)))
            .collect(),
        all_sites: (0..cfg.sites.len()).all(present),
        ..IdentityOut::default()
    };
    if !mine.is_empty() {
        out.inchikeys = Some(sorted_join(
            mine.iter().filter_map(|s| s.inchikey.as_deref()),
            " | ",
        ));
        out.key_origin = Some(sorted_join(
            mine.iter().map(|s| s.key_origin.as_str()),
            " | ",
        ));
        out.structure_basis = Some(sorted_join(mine.iter().map(|s| s.ik_basis.as_str()), " | "));
    }
    out.first_blocks = blocks;
    out
}

/// The label of one identity in one list: the entries it matched, the first-block-only ones flagged.
fn list_label(long: &[LongRow], lid: &str, name: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for r in long
        .iter()
        .filter(|r| r.list_id == lid && r.nome_comune == name)
    {
        let flag = if r.matched == MATCH_BLOCK {
            " {first block}"
        } else {
            ""
        };
        let p = format!("{} [{}]{flag}", r.list_name, r.list_attribute);
        if !parts.contains(&p) {
            parts.push(p);
        }
    }
    parts.join(" || ")
}

fn fill_labels(ident: &mut [IdentityOut], lists: &[String], long: &[LongRow]) {
    for i in ident {
        i.labels = lists
            .iter()
            .map(|lid| list_label(long, lid, &i.nome_comune))
            .collect();
        i.n_lists = i.labels.iter().filter(|l| !l.is_empty()).count();
        i.lists = lists
            .iter()
            .zip(&i.labels)
            .filter(|(_, l)| !l.is_empty())
            .map(|(l, _)| catalog::label(l))
            .collect::<Vec<_>>()
            .join(", ");
    }
}

fn summary_row(
    k: usize,
    lid: &str,
    mem: &[&MembershipRow],
    ident: &[IdentityOut],
    long: &[LongRow],
    cfg: &B1Config,
) -> SummaryRow {
    let entries: Vec<&&MembershipRow> = mem.iter().filter(|m| m.list_id == lid).collect();
    let hit: Vec<&IdentityOut> = ident.iter().filter(|i| !i.labels[k].is_empty()).collect();
    let full: BTreeSet<&str> = long
        .iter()
        .filter(|r| r.list_id == lid && r.matched == MATCH_FULL)
        .map(|r| r.nome_comune.as_str())
        .collect();
    let in_win = |m: &&&MembershipRow| in_window(&m.mono_mass, cfg.window);
    let amenable = |m: &&&MembershipRow, doubtful: bool| {
        (m.lc_esi_amenable == "true" || (doubtful && m.lc_esi_amenable == "doubtful")) && in_win(m)
    };
    let by_key = |i: &&&IdentityOut| full.contains(i.nome_comune.as_str());
    SummaryRow {
        list_id: lid.to_string(),
        group: catalog::group_of(lid).to_string(),
        list: catalog::label(lid),
        question: catalog::question(lid).to_string(),
        entries: entries
            .iter()
            .map(|m| m.first_block.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        y_true_in_window: entries.iter().filter(|m| amenable(m, false)).count(),
        y_true_doubtful_in_window: entries.iter().filter(|m| amenable(m, true)).count(),
        x_identities: hit.len(),
        x_anthropogenic: hit.iter().filter(|i| i.group == "contaminant").count(),
        x_natural: hit.iter().filter(|i| i.group != "contaminant").count(),
        x_by_inchikey: hit.iter().filter(by_key).count(),
        x_first_block_only: hit.iter().filter(|i| !by_key(i)).count(),
        in_sub: (0..cfg.submatrices.len())
            .map(|s| hit.iter().filter(|i| i.in_sub[s]).count())
            .collect(),
        all_sites: hit.iter().filter(|i| i.all_sites).count(),
        matched: hit
            .iter()
            .map(|i| i.nome_comune.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn coverage_of(ident: &[IdentityOut]) -> Vec<(&'static str, usize)> {
    let has = |i: &&IdentityOut| !i.first_blocks.is_empty();
    let anthro = |i: &&IdentityOut| i.group == "contaminant";
    let n = |f: &dyn Fn(&&IdentityOut) -> bool| ident.iter().filter(|i| f(i)).count();
    vec![
        ("identities Lvl1-3 (canonical dedup)", ident.len()),
        ("of which with >= 1 structure (registry)", n(&has)),
        ("anthropogenic", n(&anthro)),
        (
            "anthropogenic with >= 1 structure",
            n(&|i| anthro(i) && has(i)),
        ),
        ("identities with SPLIT (> 1 first block)", n(&|i| i.split)),
        (
            "identities involved in a MERGE",
            n(&|i| !i.merge.is_empty()),
        ),
        ("identities in >= 1 list", n(&|i| i.n_lists > 0)),
        ("identities without structure", n(&|i| !has(i))),
    ]
}

pub fn match_campaign(
    idents: &[IdentityIn],
    structs: &[StructRow],
    mem: &[MembershipRow],
    cfg: &B1Config,
    lists: Option<&[&str]>,
) -> B1 {
    let names: BTreeSet<&str> = idents.iter().map(|i| i.nome_comune.as_str()).collect();
    let structs: Vec<&StructRow> = structs
        .iter()
        .filter(|s| names.contains(s.nome_comune.as_str()))
        .collect();
    let all_lists: Vec<&str> = match lists {
        Some(l) => l.to_vec(),
        None => mem
            .iter()
            .map(|m| m.list_id.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    };
    let lists = catalog::ordered(&all_lists);
    let mem: Vec<&MembershipRow> = mem.iter().filter(|m| lists.contains(&m.list_id)).collect();
    let long = join_long(&structs, &mem);
    let with_block: Vec<&&StructRow> = structs.iter().filter(|s| s.first_block.is_some()).collect();
    let owners = block_owners(&with_block);
    let mut ident: Vec<IdentityOut> = idents
        .iter()
        .map(|i| {
            let mine: Vec<&&&StructRow> = with_block
                .iter()
                .filter(|s| s.nome_comune == i.nome_comune)
                .collect();
            identity_out(i, &mine, &owners, cfg)
        })
        .collect();
    fill_labels(&mut ident, &lists, &long);
    ident.sort_by(|a, b| {
        b.n_lists
            .cmp(&a.n_lists)
            .then_with(|| a.group.cmp(&b.group))
            .then_with(|| a.nome_comune.cmp(&b.nome_comune))
    });
    let summary = lists
        .iter()
        .enumerate()
        .map(|(k, lid)| summary_row(k, lid, &mem, &ident, &long, cfg))
        .collect();
    let coverage = coverage_of(&ident);
    B1 {
        lists,
        ident,
        long,
        summary,
        coverage,
    }
}
