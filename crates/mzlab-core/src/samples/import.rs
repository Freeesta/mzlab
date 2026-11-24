//! `import`: a CSV/TSV of file name -> sample name, role and group laid over the samples of a set of files (`sample-import/1`). Each row is matched to a file
//! (exact name, then case, then without the extension); what cannot be matched, what is ambiguous and what is repeated is reported with its evidence and every
//! automatic choice is logged. Nothing is refused for a finding: the user decides from the report.

use super::csv::{detect, field_of};
use super::csv_fields::records;
use super::json::role_alias;
use super::order::{base, stem};
use super::{files_of, propose, table_from_value, table_to_json, Sample, Table};
use crate::error::{Error, Result};
use crate::json::{parse, Out};

pub const SCHEMA: &str = "sample-import/1";

/// One row of the table, its position (the header is row 1) and the values it sets.
struct Row {
    n: usize,
    key: String,
    name: String,
    role: String,
    group: String,
}

/// `(kind, blank_kind)` of a role word: `wash`, `solvent` and `procedure` are blanks of that kind.
fn role_of(word: &str) -> Option<(&'static str, Option<&'static str>)> {
    let l = word.trim().to_lowercase();
    match l.as_str() {
        "wash" => Some(("blank", Some("wash"))),
        "solvent" => Some(("blank", Some("solvent"))),
        "procedure" => Some(("blank", Some("procedure"))),
        _ => role_alias(&l).map(|k| (k, None)),
    }
}

/// The files a key may mean, from the strictest rule that finds any: (rule, indices).
fn candidates(key: &str, files: &[String]) -> (&'static str, Vec<usize>) {
    let key = base(key);
    let rules: [(&str, &dyn Fn(&str) -> bool); 4] = [
        ("exact", &|f| base(f) == key),
        ("case", &|f| base(f).eq_ignore_ascii_case(key)),
        ("stem", &|f| stem(f) == stem(key)),
        ("stem-case", &|f| stem(f).eq_ignore_ascii_case(stem(key))),
    ];
    for (rule, same) in rules {
        let hit: Vec<usize> = (0..files.len()).filter(|&i| same(&files[i])).collect();
        if !hit.is_empty() {
            return (rule, hit);
        }
    }
    ("none", Vec::new())
}

/// A JSON object of text fields, in the order given (one entry of a report list).
fn obj(fields: &[(&str, String)]) -> String {
    let mut o = Out::default();
    o.raw("{");
    for (i, (k, v)) in fields.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":");
        // numbers, lists and flags are written as they are; every other value is text, even one that looks like a number (a sample called `12`)
        if matches!(
            *k,
            "row" | "kept" | "rows" | "candidates" | "files" | "conflict"
        ) {
            o.raw(v);
        } else {
            o.str(v);
        }
    }
    o.raw("}");
    o.0
}

fn list<S: AsRef<str>>(items: &[S]) -> String {
    format!(
        "[{}]",
        items
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn quoted(items: &[String]) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, s) in items.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(s);
    }
    o.raw("]");
    o.0
}

/// Reads the rows and logs the columns that are not used.
fn read_rows(text: &str, sep: Option<char>, log: &mut Vec<String>) -> Result<Vec<Row>> {
    let recs = records(text, sep.unwrap_or_else(|| detect(text)))?;
    let head = recs.first().ok_or_else(|| super::csv::bad(1, "file"))?;
    let fields: Vec<Option<&str>> = head.iter().map(|(h, _)| field_of(h)).collect();
    let at = |f: &str| fields.iter().position(|x| *x == Some(f));
    let (file, name, role, group) = (at("file"), at("sample_id"), at("kind"), at("group"));
    let file = file.ok_or_else(|| super::csv::bad(1, "file"))?;
    for (i, (h, _)) in head.iter().enumerate() {
        if ![Some(file), name, role, group].contains(&Some(i)) {
            log.push(obj(&[
                ("what", "ignored_column".into()),
                ("row", "1".into()),
                ("file", String::new()),
                ("from", h.to_string()),
                ("to", String::new()),
            ]));
        }
    }
    let cell = |r: &super::csv_fields::Cells, i: Option<usize>| {
        i.and_then(|i| r.get(i))
            .map_or(String::new(), |c| c.0.trim().to_string())
    };
    Ok(recs[1..]
        .iter()
        .enumerate()
        .filter(|(_, r)| r.iter().any(|c| !c.0.trim().is_empty()))
        .map(|(k, r)| Row {
            n: k + 2,
            key: cell(r, Some(file)),
            name: cell(r, name),
            role: cell(r, role),
            group: cell(r, group),
        })
        .collect())
}

/// Lays the row `r` over the sample `s`, logging each value it changes; false when the role word is not a role.
fn apply(s: &mut Sample, r: &Row, log: &mut Vec<String>) -> bool {
    let mut change = |what: &str, from: &str, to: &str| {
        if from != to {
            log.push(obj(&[
                ("what", what.into()),
                ("row", r.n.to_string()),
                ("file", s.file.clone()),
                ("from", from.into()),
                ("to", to.into()),
            ]));
        }
    };
    if !r.name.is_empty() {
        change("sample_name", &s.sample_id, &r.name);
        s.sample_id = r.name.clone();
    }
    if !r.group.is_empty() {
        change("group", &s.group, &r.group);
        s.group = r.group.clone();
    }
    if r.role.is_empty() {
        return true;
    }
    let Some((kind, blank)) = role_of(&r.role) else {
        return false;
    };
    let before = format!(
        "{}{}",
        s.kind,
        s.blank_kind
            .as_deref()
            .map_or(String::new(), |b| format!("/{b}"))
    );
    s.kind = kind.into();
    s.blank_kind = blank.map(String::from);
    let after = format!("{kind}{}", blank.map_or(String::new(), |b| format!("/{b}")));
    change("role", &before, &after);
    true
}

/// `{"text", "separator"?, "files": [{"name","sha256"?}] | "samples": <samples/1>}` -> `sample-import/1`: the samples with the rows laid over them and the report
/// (`status` complete | partial, `rows`, `matched`, `unmatched_rows`, `unmatched_files`, `duplicates`, `unknown_roles`, `shared_names`, `choices`).
pub fn import(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let text = v
        .get("text")
        .and_then(|x| x.str())
        .ok_or_else(|| Error::new("error.badRequest"))?;
    let sep = match v.get("separator").and_then(|s| s.str()) {
        None | Some("") => None,
        Some(",") => Some(','),
        Some(";") => Some(';'),
        Some("\t") | Some("tab") => Some('\t'),
        Some(_) => return Err(Error::new("error.badRequest")),
    };
    let mut table = match v.get("samples") {
        Some(doc) => table_from_value(doc)?,
        None => Table {
            samples: propose(&files_of(input)?),
            columns: Vec::new(),
        },
    };
    let (mut log, mut unmatched, mut unknown) = (Vec::new(), Vec::new(), Vec::new());
    let rows = read_rows(text, sep, &mut log)?;
    let names: Vec<String> = table.samples.iter().map(|s| s.file.clone()).collect();
    let mut by_file: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for (k, r) in rows.iter().enumerate() {
        let (rule, hit) = candidates(&r.key, &names);
        match hit.as_slice() {
            [i] => {
                by_file[*i].push(k);
                if rule != "exact" {
                    log.push(obj(&[
                        ("what", format!("match:{rule}")),
                        ("row", r.n.to_string()),
                        ("file", names[*i].clone()),
                        ("from", r.key.clone()),
                        ("to", names[*i].clone()),
                    ]));
                }
            }
            [] => unmatched.push(obj(&[
                ("row", r.n.to_string()),
                ("file", r.key.clone()),
                ("why", "no file".into()),
                ("candidates", "[]".into()),
            ])),
            many => unmatched.push(obj(&[
                ("row", r.n.to_string()),
                ("file", r.key.clone()),
                ("why", "ambiguous".into()),
                (
                    "candidates",
                    quoted(&many.iter().map(|&i| names[i].clone()).collect::<Vec<_>>()),
                ),
            ])),
        }
    }
    let (mut duplicates, mut matched) = (Vec::new(), 0);
    let mut conflict = false;
    for (i, ks) in by_file.iter().enumerate() {
        let Some(&first) = ks.first() else { continue };
        matched += 1;
        if ks.len() > 1 {
            let same =
                |a: &Row, b: &Row| (&a.name, &a.role, &a.group) == (&b.name, &b.role, &b.group);
            let differ = ks.iter().any(|&k| !same(&rows[k], &rows[first]));
            conflict |= differ;
            let nums: Vec<String> = ks.iter().map(|&k| rows[k].n.to_string()).collect();
            duplicates.push(obj(&[
                ("file", names[i].clone()),
                ("rows", list(&nums)),
                ("conflict", differ.to_string()),
                ("kept", rows[first].n.to_string()),
            ]));
            log.push(obj(&[
                ("what", "duplicate_first".into()),
                ("row", rows[first].n.to_string()),
                ("file", names[i].clone()),
                ("from", nums.join(",")),
                ("to", rows[first].n.to_string()),
            ]));
        }
        if !apply(&mut table.samples[i], &rows[first], &mut log) {
            unknown.push(obj(&[
                ("row", rows[first].n.to_string()),
                ("file", names[i].clone()),
                ("value", rows[first].role.clone()),
            ]));
        }
    }
    let free: Vec<String> = (0..names.len())
        .filter(|&i| by_file[i].is_empty())
        .map(|i| names[i].clone())
        .collect();
    // a sample name on several files: one sample in both polarities is legitimate, so this is evidence, not a fault
    let mut shared = Vec::new();
    for (i, s) in table.samples.iter().enumerate() {
        let id = &s.sample_id;
        let all: Vec<String> = table
            .samples
            .iter()
            .filter(|o| &o.sample_id == id)
            .map(|o| o.file.clone())
            .collect();
        let first = table.samples.iter().position(|o| &o.sample_id == id) == Some(i);
        if !id.is_empty() && all.len() > 1 && first {
            shared.push(obj(&[("name", id.clone()), ("files", quoted(&all))]));
        }
    }
    let complete = unmatched.is_empty() && free.is_empty() && !conflict && unknown.is_empty();
    let mut o = Out::default();
    o.raw("{\"report\":{\"choices\":")
        .raw(&list(&log))
        .raw(",\"duplicates\":")
        .raw(&list(&duplicates))
        .raw(",\"matched\":")
        .int(matched)
        .raw(",\"rows\":")
        .int(rows.len() as i64)
        .raw(",\"shared_names\":")
        .raw(&list(&shared))
        .raw(",\"status\":")
        .str(if complete { "complete" } else { "partial" })
        .raw(",\"unknown_roles\":")
        .raw(&list(&unknown))
        .raw(",\"unmatched_files\":")
        .raw(&quoted(&free))
        .raw(",\"unmatched_rows\":")
        .raw(&list(&unmatched))
        .raw("},\"samples\":")
        .raw(&table_to_json(&table))
        .raw(",\"schema\":")
        .str(SCHEMA)
        .raw("}");
    Ok(o.0)
}
