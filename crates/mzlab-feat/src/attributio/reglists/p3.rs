// Ported from Attributio reglists/compare.py.
//! P3: for each list, the compounds found in one campaign, in the other or in both, by InChIKey first block only.

use super::b1::{LongRow, B1, MATCH_FULL};
use super::catalog;
use std::collections::{BTreeMap, BTreeSet};

/// What one campaign has for a (list, first block).
#[derive(Clone, Debug, PartialEq)]
pub struct Side {
    pub identity: String,
    pub level: i64,
    pub group: &'static str,
    pub sites: String,
    pub matched: &'static str,
}

/// One (list, first block) of the table with the side of every campaign (`None`: not found there).
#[derive(Clone, Debug, PartialEq)]
pub struct CrossRow {
    pub list_group: &'static str,
    pub list_id: String,
    pub list: String,
    pub first_block: String,
    pub list_name: String,
    pub found_in: String,
    pub sides: Vec<Option<Side>>,
    pub list_inchikey: String,
    pub list_attribute: String,
}

/// One row per list of the summary: campaign-only counts, both/all, total.
#[derive(Clone, Debug, PartialEq)]
pub struct CrossSummary {
    pub list_group: &'static str,
    pub list_id: String,
    pub list: String,
    pub only: Vec<usize>,
    pub both: usize,
    pub total: usize,
    pub compounds_in_both: String,
}

fn side(b: &B1) -> BTreeMap<(String, String), (Side, &LongRow)> {
    let mut groups: BTreeMap<(String, String), Vec<&LongRow>> = BTreeMap::new();
    for r in &b.long {
        groups
            .entry((r.list_id.clone(), r.first_block.clone()))
            .or_default()
            .push(r);
    }
    groups
        .into_iter()
        .map(|(k, g)| {
            let ncs: Vec<&str> = g
                .iter()
                .map(|r| r.nome_comune.as_str())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let id = |n: &str| b.ident.iter().find(|i| i.nome_comune == n);
            let level = ncs
                .iter()
                .filter_map(|n| id(n))
                .map(|i| i.best_level)
                .min()
                .unwrap_or(0);
            let contaminant = ncs
                .iter()
                .filter_map(|n| id(n))
                .any(|i| i.group == "contaminant");
            let sites = ncs
                .iter()
                .filter_map(|n| id(n))
                .map(|i| i.sites.clone())
                .collect::<Vec<_>>()
                .join(" | ");
            let matched = if g.iter().any(|r| r.matched == MATCH_FULL) {
                MATCH_FULL
            } else {
                "first block"
            };
            (
                k,
                (
                    Side {
                        identity: ncs.join(" | "),
                        level,
                        group: if contaminant {
                            "contaminant"
                        } else {
                            "natural/other"
                        },
                        sites,
                        matched,
                    },
                    g[0],
                ),
            )
        })
        .collect()
}

/// (long table, summary per list) for campaigns given as (name, B1) in order. The long table is sorted by list in catalogue order (unknown lists sort among the
/// known ones by `found_in` and block, as the reference does), then `found_in`, then first block.
type Present<'a> = Vec<Option<&'a (Side, &'a LongRow)>>;

fn cross_row(k: &(String, String), present: &Present, names: &[&str]) -> CrossRow {
    let first = present
        .iter()
        .flatten()
        .next()
        .map(|(_, r)| *r)
        .expect("one side");
    let found = names
        .iter()
        .zip(present)
        .filter(|(_, p)| p.is_some())
        .map(|(n, _)| *n)
        .collect::<Vec<_>>()
        .join("+");
    let found_in = if found != names.join("+") {
        found
    } else if names.len() == 2 {
        "both".to_string()
    } else {
        "all".to_string()
    };
    CrossRow {
        list_group: catalog::group_of(&k.0),
        list_id: k.0.clone(),
        list: catalog::label(&k.0),
        first_block: k.1.clone(),
        list_name: first.list_name.clone(),
        found_in,
        sides: present.iter().map(|p| p.map(|(s, _)| s.clone())).collect(),
        list_inchikey: first.list_inchikey.clone(),
        list_attribute: first.list_attribute.clone(),
    }
}

fn summarize(rows: &[CrossRow], names: &[&str]) -> Vec<CrossSummary> {
    let mut order: Vec<&str> = Vec::new();
    for r in rows {
        if !order.contains(&r.list_id.as_str()) {
            order.push(&r.list_id);
        }
    }
    order
        .into_iter()
        .map(|lid| {
            let g: Vec<&CrossRow> = rows.iter().filter(|r| r.list_id == lid).collect();
            let in_both = |r: &&&CrossRow| r.found_in == "both" || r.found_in == "all";
            CrossSummary {
                list_group: catalog::group_of(lid),
                list_id: lid.to_string(),
                list: catalog::label(lid),
                only: names
                    .iter()
                    .map(|n| g.iter().filter(|r| r.found_in == *n).count())
                    .collect(),
                both: g.iter().filter(in_both).count(),
                total: g.len(),
                compounds_in_both: g
                    .iter()
                    .filter(in_both)
                    .map(|r| r.list_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            }
        })
        .collect()
}

pub fn cross_table(results: &[(&str, &B1)]) -> (Vec<CrossRow>, Vec<CrossSummary>) {
    let sides: Vec<_> = results.iter().map(|(_, b)| side(b)).collect();
    let keys: BTreeSet<&(String, String)> = sides.iter().flat_map(|s| s.keys()).collect();
    if keys.is_empty() {
        return (vec![], vec![]);
    }
    let names: Vec<&str> = results.iter().map(|(n, _)| *n).collect();
    let mut rows: Vec<CrossRow> = keys
        .into_iter()
        .map(|k| cross_row(k, &sides.iter().map(|s| s.get(k)).collect(), &names))
        .collect();
    // NaN (unknown list) sorts last in pandas; the three keys are applied one after the other with a stable sort, so an unknown list is a tie on the first key
    rows.sort_by(|a, b| {
        let ord = |r: &CrossRow| catalog::order_of(&r.list_id).unwrap_or(usize::MAX);
        ord(a)
            .cmp(&ord(b))
            .then_with(|| a.found_in.cmp(&b.found_in))
            .then_with(|| a.first_block.cmp(&b.first_block))
    });
    let summary = summarize(&rows, &names);
    (rows, summary)
}
