//! `sub/1`: where a drawn moiety occurs in a list of structures (feature `record`, matcher `mzlab_mol::SubQuery`). The request is
//! `{"query", "targets": [..], "stereo", "max"}` with SMILES or molfiles as text; the answer lists, per target that
//! contains the query, its distinct matches (target atoms of every query atom, merged query hydrogens left out). A hit is a suspect to
//! check, never an identification. With `"db"` (the text of an SDF, a SMILES list or a molecules CSV such as the catalogue) instead of `targets` the
//! answer is `subdb/1`: the same hits with the name, id and structure of each entry. The same bytes from the desktop route `feat/grafo`, the `molkit`
//! `.wasm` and `mzlab-cli mol-search --sub`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out};
use mzlab_mol::{read_target, SubOptions, SubQuery};

/// Most matches listed per target; the count `n` stops there too.
const MAX_MATCHES: usize = 1000;

/// One entry of `hits`: the target index, the number of matches and the target atoms of each.
fn hit(o: &mut Out, first: bool, i: usize, rows: &[Vec<u32>]) {
    o.raw(if first { "{\"i\":" } else { ",{\"i\":" })
        .int(i as i64)
        .raw(",");
    maps(o, rows);
}

/// `"n":N,"maps":[..]}`: the number of matches and the target atoms of each, closing the hit.
fn maps(o: &mut Out, rows: &[Vec<u32>]) {
    o.raw("\"n\":").int(rows.len() as i64).raw(",\"maps\":[");
    for (k, r) in rows.iter().enumerate() {
        o.raw(if k > 0 { ",[" } else { "[" });
        for (j, a) in r.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).int(i64::from(*a));
        }
        o.raw("]");
    }
    o.raw("]}");
}

/// One structure of a list: its name and id (empty when the list has none) and its text (SMILES or molfile).
struct Entry {
    name: String,
    id: String,
    text: String,
}

/// Most entries of a structure list.
const MAX_ENTRIES: usize = 200_000;

/// The value of an SDF data field `> <NAME>` (name compared without case), the first line under it.
fn sdf_field(lines: &[&str], name: &str) -> String {
    let at = lines.iter().position(|l| {
        let l = l.trim();
        l.starts_with('>')
            && l.split_once('<')
                .and_then(|(_, r)| r.split_once('>'))
                .is_some_and(|(n, _)| n.trim().eq_ignore_ascii_case(name))
    });
    at.and_then(|k| lines.get(k + 1))
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

/// The entries of an SDF: records split at `$$$$`, the molfile up to `M  END`, the name from the title line or the `name` field.
fn sdf(text: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    for (k, rec) in text.split("\n$$$$").enumerate() {
        // after a `$$$$` the rest of its line goes; an empty title line that follows is the record's own
        let rec = if k == 0 {
            rec
        } else {
            rec.split_once('\n').map_or("", |r| r.1)
        };
        let lines: Vec<&str> = rec.lines().collect();
        if lines.iter().all(|l| l.trim().is_empty()) {
            continue;
        }
        let end = lines.iter().position(|l| l.trim_end() == "M  END");
        let (mol, data) = match end {
            Some(e) => (lines[..=e].join("\n"), &lines[e + 1..]),
            None => (lines.join("\n"), &lines[lines.len()..]),
        };
        let title = lines[0].trim();
        out.push(Entry {
            name: if title.is_empty() {
                sdf_field(data, "name")
            } else {
                title.to_string()
            },
            id: sdf_field(data, "id"),
            text: mol + "\n",
        });
    }
    out
}

/// The entries of a table with a `smiles` column (a molecules CSV such as the catalogue), with how many rows have no SMILES.
fn table(text: &str) -> (Vec<Entry>, usize) {
    let rows = mzlab_core::molecules::table(text);
    let Some(((_, head), body)) = rows.split_first() else {
        return (Vec::new(), 0);
    };
    let col = |n: &str| head.iter().position(|h| h.trim().eq_ignore_ascii_case(n));
    let (smi, name, id) = (col("smiles"), col("name"), col("id"));
    let cell =
        |r: &[String], c: Option<usize>| c.and_then(|c| r.get(c)).cloned().unwrap_or_default();
    let (mut out, mut skipped) = (Vec::new(), 0);
    for (_, r) in body {
        let text = cell(r, smi);
        if text.is_empty() {
            skipped += 1;
            continue;
        }
        out.push(Entry {
            name: cell(r, name),
            id: cell(r, id),
            text,
        });
    }
    (out, skipped)
}

/// The entries of a SMILES list: one per line, the SMILES then (after a space or a tab) its name; empty and `#` lines are left out.
fn smiles_list(text: &str) -> Vec<Entry> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (s, n) = l.split_once([' ', '\t']).unwrap_or((l, ""));
            Entry {
                name: n.trim().to_string(),
                id: String::new(),
                text: s.to_string(),
            }
        })
        .collect()
}

/// The kind of a structure list and its entries: an SDF when a record ends with `M  END`, a table when the first line has a `smiles` column, else a
/// SMILES list; plus the table rows without a structure.
fn entries(text: &str) -> (&'static str, Vec<Entry>, usize) {
    let text = text.trim_start_matches('\u{feff}');
    if text.lines().any(|l| l.trim_end() == "M  END") {
        return ("sdf", sdf(text), 0);
    }
    let head = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .unwrap_or("");
    if head
        .split([';', '\t', ','])
        .any(|c| c.trim().trim_matches('"').eq_ignore_ascii_case("smiles"))
    {
        let (e, s) = table(text);
        return ("table", e, s);
    }
    ("smiles", smiles_list(text), 0)
}

/// The `subdb/1` answer: `sub/1` over the entries of a structure list, each hit with its name, id and structure.
fn db_json(q: &SubQuery, opts: &SubOptions, db: &str) -> Result<String> {
    let (source, list, skipped) = entries(db);
    if list.is_empty() {
        return Err(Error::new("error.sub.db"));
    }
    if list.len() > MAX_ENTRIES {
        return Err(Error::new("error.sub.dbSize")
            .with("n", list.len().to_string())
            .with("max", MAX_ENTRIES.to_string()));
    }
    let mut o = Out::default();
    o.raw("{\"format\":\"subdb/1\",\"atoms\":")
        .int(q.mapped_atoms() as i64)
        .raw(",\"stereo\":")
        .raw(if opts.stereo { "true" } else { "false" })
        .raw(",\"source\":")
        .str(source)
        .raw(",\"total\":")
        .int(list.len() as i64)
        .raw(",\"skipped\":")
        .int(skipped as i64)
        .raw(",\"hits\":[");
    let mut errors = Out::default();
    let (mut first, mut first_err) = (true, true);
    for (i, e) in list.iter().enumerate() {
        match read_target(&e.text).and_then(|m| q.matches(&m, opts)) {
            Ok(rows) if rows.is_empty() => {}
            Ok(rows) => {
                o.raw(if first { "{\"i\":" } else { ",{\"i\":" })
                    .int(i as i64)
                    .raw(",\"name\":")
                    .str(&e.name)
                    .raw(",\"id\":")
                    .str(&e.id)
                    .raw(",\"structure\":")
                    .str(&e.text)
                    .raw(",");
                maps(&mut o, &rows);
                first = false;
            }
            Err(err) => {
                errors
                    .raw(if first_err { "{\"i\":" } else { ",{\"i\":" })
                    .int(i as i64)
                    .raw(",\"name\":")
                    .str(&e.name)
                    .raw(",\"error\":")
                    .str(&err.to_string())
                    .raw("}");
                first_err = false;
            }
        }
    }
    o.raw("],\"errors\":[").raw(&errors.0).raw("]}");
    Ok(o.0)
}

/// The `sub/1` answer of a request (`subdb/1` when it carries a `db`).
pub fn search_json(request: &str) -> Result<String> {
    let bad = || Error::new("error.sub.request");
    let v = json::parse(request.as_bytes()).map_err(|_| bad())?;
    let query = v.get("query").and_then(|q| q.str()).ok_or_else(bad)?;
    let db = v.get("db").map(|d| d.str().ok_or_else(bad)).transpose()?;
    let opts = SubOptions {
        stereo: v.get("stereo").and_then(|s| s.bool()).unwrap_or(false),
        max_matches: v
            .get("max")
            .and_then(|m| m.f64())
            .map_or(MAX_MATCHES, |m| (m.max(1.0) as usize).min(MAX_MATCHES)),
        ..SubOptions::default()
    };
    let q =
        SubQuery::from_text(query).map_err(|e| Error::new("error.sub.query").with("detail", e))?;
    if let Some(db) = db {
        return db_json(&q, &opts, db);
    }
    let targets = v.get("targets").map(|t| t.arr()).ok_or_else(bad)?;
    let mut o = Out::default();
    o.raw("{\"format\":\"sub/1\",\"atoms\":")
        .int(q.mapped_atoms() as i64)
        .raw(",\"stereo\":")
        .raw(if opts.stereo { "true" } else { "false" })
        .raw(",\"hits\":[");
    let mut errors = Out::default();
    let (mut first, mut first_err) = (true, true);
    for (i, t) in targets.iter().enumerate() {
        let found = t
            .str()
            .ok_or_else(|| mzlab_mol::MolError::Json("not a text".into()))
            .and_then(read_target)
            .and_then(|m| q.matches(&m, &opts));
        match found {
            Ok(rows) if rows.is_empty() => {}
            Ok(rows) => {
                hit(&mut o, first, i, &rows);
                first = false;
            }
            Err(e) => {
                errors
                    .raw(if first_err { "{\"i\":" } else { ",{\"i\":" })
                    .int(i as i64)
                    .raw(",\"error\":")
                    .str(&e.to_string())
                    .raw("}");
                first_err = false;
            }
        }
    }
    o.raw("],\"errors\":[").raw(&errors.0).raw("]}");
    Ok(o.0)
}

#[cfg(test)]
#[path = "substructure_tests.rs"]
mod tests;
