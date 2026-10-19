// Ported from Attributio reglists/schema.py.
//! The one format of every regulatory list (schema A1, one row per InChIKey first block inside a list) and the membership tables built from the lists: the
//! long join table (one row per list and first block) and its wide form (one 0/1 column per list). Text in, text out: where the files live is the caller's business.

use crate::attributio::error::Result;
use crate::attributio::frame::Frame;
use crate::attributio::tox::snapshot::MembershipRow;
use crate::attributio::tox::vocab::canon;
use std::collections::{BTreeMap, BTreeSet};

/// Columns of a list file (schema A1), in order.
pub const SCHEMA: [&str; 20] = [
    "list_id",
    "list_version",
    "access_date",
    "source",
    "name",
    "cas",
    "pubchem_cid",
    "inchikey",
    "inchikey_skeleton",
    "smiles",
    "formula",
    "mono_mass",
    "list_attribute",
    "is_tp",
    "parent",
    "tp_reference",
    "lc_esi_amenable",
    "amenability_rule",
    "xlogp",
    "resolved_by",
];
const VOCAB_COLUMNS: [&str; 3] = ["is_tp", "lc_esi_amenable", "resolved_by"];
const TEXT_COLUMNS: [&str; 5] = [
    "name",
    "list_attribute",
    "parent",
    "tp_reference",
    "amenability_rule",
];

/// One row of a list in schema A1: every cell is text, in `SCHEMA` order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ListRow(pub [String; 20]);

impl ListRow {
    fn idx(col: &str) -> usize {
        SCHEMA
            .iter()
            .position(|c| *c == col)
            .expect("schema column")
    }
    pub fn get(&self, col: &str) -> &str {
        &self.0[Self::idx(col)]
    }
    pub fn set(&mut self, col: &str, v: String) {
        self.0[Self::idx(col)] = v;
    }
}

/// A list file as rows; missing columns are empty, vocabulary columns are read in English (`canon`).
pub fn read_list(csv: &str) -> Result<Vec<ListRow>> {
    let f = Frame::parse(csv, ';')?;
    Ok((0..f.len())
        .map(|r| {
            let mut row = ListRow::default();
            for c in SCHEMA {
                let v = f.col(c).map_or("", |i| f.cell(r, i)).to_string();
                row.set(
                    c,
                    if VOCAB_COLUMNS.contains(&c) {
                        canon(&v)
                    } else {
                        v
                    },
                );
            }
            row
        })
        .collect())
}

fn clean(s: &str) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t == "nan" || t == "None" {
        String::new()
    } else {
        t
    }
}

/// Text columns cleaned: single spaces, no «nan» and no «None».
pub fn normalise(rows: &[ListRow]) -> Vec<ListRow> {
    rows.iter()
        .map(|r| {
            let mut o = r.clone();
            for c in TEXT_COLUMNS {
                o.set(c, clean(r.get(c)));
            }
            o
        })
        .collect()
}

fn csv_cell(s: &str) -> String {
    if s.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn csv_text(header: &[&str], rows: impl Iterator<Item = Vec<String>>) -> String {
    let mut out = header.join(";");
    out.push('\n');
    for r in rows {
        out.push_str(&r.iter().map(|c| csv_cell(c)).collect::<Vec<_>>().join(";"));
        out.push('\n');
    }
    out
}

/// The text of a list file: normalised, sorted by first block then name (stable).
pub fn write_list(rows: &[ListRow]) -> String {
    let mut n = normalise(rows);
    n.sort_by(|a, b| {
        (a.get("inchikey_skeleton"), a.get("name"))
            .cmp(&(b.get("inchikey_skeleton"), b.get("name")))
    });
    csv_text(&SCHEMA, n.iter().map(|r| r.0.to_vec()))
}

fn join_unique<'a>(vals: impl Iterator<Item = &'a str>) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for v in vals {
        if !seen.contains(&v) {
            seen.push(v);
        }
    }
    seen.into_iter()
        .filter(|v| !v.is_empty() && *v != "nan")
        .collect::<Vec<_>>()
        .join(" ; ")
}

/// One row per first block inside a list: entries that collapse (a salt and its free acid, enantiomers) are merged, names, CAS, attributes and
/// resolution kinds joined by « ; ».
pub fn collapse_by_skeleton(rows: &[ListRow]) -> Vec<ListRow> {
    let n = normalise(rows);
    let mut order: Vec<&str> = Vec::new();
    let mut groups: BTreeMap<&str, Vec<&ListRow>> = BTreeMap::new();
    for r in &n {
        let k = r.get("inchikey_skeleton");
        if !groups.contains_key(k) {
            order.push(k);
        }
        groups.entry(k).or_default().push(r);
    }
    let merged: Vec<ListRow> = order
        .iter()
        .map(|k| {
            let g = &groups[k];
            let mut o = g[0].clone();
            for c in ["name", "cas", "list_attribute", "resolved_by"] {
                o.set(c, join_unique(g.iter().map(|r| r.get(c))));
            }
            o
        })
        .collect();
    normalise(&merged)
}

/// The long join table from `(list_id, rows)` pairs: rows whose first block is not 14 characters are dropped, `groups` gives the group of each list; sorted by
/// first block then list.
pub fn membership(
    lists: &[(&str, &[ListRow])],
    groups: &BTreeMap<String, String>,
) -> Vec<MembershipRow> {
    let mut out: Vec<MembershipRow> = Vec::new();
    for (lid, rows) in lists {
        for r in rows
            .iter()
            .filter(|r| r.get("inchikey_skeleton").chars().count() == 14)
        {
            out.push(MembershipRow {
                first_block: r.get("inchikey_skeleton").into(),
                list_id: r.get("list_id").into(),
                name: r.get("name").into(),
                list_attribute: r.get("list_attribute").into(),
                is_tp: r.get("is_tp").into(),
                parent: r.get("parent").into(),
                lc_esi_amenable: r.get("lc_esi_amenable").into(),
                amenability_rule: r.get("amenability_rule").into(),
                formula: r.get("formula").into(),
                mono_mass: r.get("mono_mass").into(),
                smiles: r.get("smiles").into(),
                inchikey: r.get("inchikey").into(),
                list_group: groups.get(*lid).cloned().unwrap_or_default(),
                cas: r.get("cas").into(),
                pubchem_cid: r.get("pubchem_cid").into(),
                xlogp: r.get("xlogp").into(),
                list_version: r.get("list_version").into(),
                resolved_by: r.get("resolved_by").into(),
            });
        }
    }
    out.sort_by(|a, b| (&a.first_block, &a.list_id).cmp(&(&b.first_block, &b.list_id)));
    out
}

/// The text of the membership table, columns as in `MEMBERSHIP_COLUMNS`.
pub fn write_membership(rows: &[MembershipRow]) -> String {
    csv_text(
        &crate::attributio::tox::snapshot::MEMBERSHIP_COLUMNS,
        rows.iter().map(|m| {
            vec![
                m.first_block.clone(),
                m.list_id.clone(),
                m.name.clone(),
                m.list_attribute.clone(),
                m.is_tp.clone(),
                m.parent.clone(),
                m.lc_esi_amenable.clone(),
                m.amenability_rule.clone(),
                m.formula.clone(),
                m.mono_mass.clone(),
                m.smiles.clone(),
                m.inchikey.clone(),
                m.list_group.clone(),
                m.cas.clone(),
                m.pubchem_cid.clone(),
                m.xlogp.clone(),
                m.list_version.clone(),
                m.resolved_by.clone(),
            ]
        }),
    )
}

/// One row of the wide table: a 0/1 flag per list (`flags`, in the order of `Wide::lists`), the number of lists and the first entry's identity columns.
#[derive(Clone, Debug, PartialEq)]
pub struct WideRow {
    pub first_block: String,
    pub flags: Vec<u8>,
    pub n_lists: usize,
    pub name: String,
    pub formula: String,
    pub mono_mass: String,
    pub lc_esi_amenable: String,
}

/// The wide membership table: the lists of `order` that occur first, then the others alphabetically.
#[derive(Clone, Debug, PartialEq)]
pub struct Wide {
    pub lists: Vec<String>,
    pub rows: Vec<WideRow>,
}

/// One row per first block, one 0/1 column per list, `n_lists`; sorted by `n_lists` descending then name.
pub fn membership_wide(long: &[MembershipRow], order: &[&str]) -> Wide {
    let present: BTreeSet<&str> = long.iter().map(|m| m.list_id.as_str()).collect();
    let mut lists: Vec<String> = order
        .iter()
        .filter(|l| present.contains(**l))
        .map(|l| l.to_string())
        .collect();
    let rest: BTreeSet<&str> = present
        .iter()
        .copied()
        .filter(|l| !order.contains(l))
        .collect();
    lists.extend(rest.into_iter().map(String::from));
    let blocks: BTreeSet<&str> = long.iter().map(|m| m.first_block.as_str()).collect();
    let mut rows: Vec<WideRow> = blocks
        .into_iter()
        .map(|b| {
            let mine: Vec<&MembershipRow> = long.iter().filter(|m| m.first_block == b).collect();
            let flags: Vec<u8> = lists
                .iter()
                .map(|l| u8::from(mine.iter().any(|m| &m.list_id == l)))
                .collect();
            let first = mine[0];
            WideRow {
                first_block: b.to_string(),
                n_lists: flags.iter().map(|f| *f as usize).sum(),
                flags,
                name: first.name.clone(),
                formula: first.formula.clone(),
                mono_mass: first.mono_mass.clone(),
                lc_esi_amenable: first.lc_esi_amenable.clone(),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.n_lists.cmp(&a.n_lists).then_with(|| a.name.cmp(&b.name)));
    Wide { lists, rows }
}
