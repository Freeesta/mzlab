//! `import`: a CSV/TSV of file name -> sample name, role and group laid over the samples of a set of files (`sample-import/1`). Each row is matched to a file
//! (exact name, then case, then without the extension); what cannot be matched, what is ambiguous and what is repeated is reported with its evidence and every
//! automatic choice is logged. Nothing is refused for a finding: the user decides from the report.

use super::csv::{detect, field_of, sep_of};
use super::csv_fields::records;
use super::json::role_alias;
use super::order::{base, stem};
use super::{files_of, propose, table_from_value, table_to_json, Sample, Table};
use crate::error::{Error, Result};
use crate::json::{parse, Out, Value};

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

/// Rows matched to files: for each file the rows (indices) that name it, and the rows that name none or several.
fn match_rows(
    rows: &[Row],
    names: &[String],
    log: &mut Vec<String>,
) -> (Vec<Vec<usize>>, Vec<String>) {
    let mut by_file: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    let mut unmatched = Vec::new();
    for (k, r) in rows.iter().enumerate() {
        let (rule, hit) = candidates(&r.key, names);
        let why = if hit.is_empty() {
            "no file"
        } else {
            "ambiguous"
        };
        if let [i] = hit.as_slice() {
            by_file[*i].push(k);
            if rule != "exact" {
                let m = format!("match:{rule}");
                log.push(obj(&[
                    ("what", m),
                    ("row", r.n.to_string()),
                    ("file", names[*i].clone()),
                    ("from", r.key.clone()),
                    ("to", names[*i].clone()),
                ]));
            }
            continue;
        }
        let near: Vec<String> = hit.iter().map(|&i| names[i].clone()).collect();
        unmatched.push(obj(&[
            ("row", r.n.to_string()),
            ("file", r.key.clone()),
            ("why", why.into()),
            ("candidates", quoted(&near)),
        ]));
    }
    (by_file, unmatched)
}

/// What laying the rows over the samples found: duplicate rows, unknown roles, how many files got a row and whether any duplicates disagree.
#[derive(Default)]
struct Settled {
    duplicates: Vec<String>,
    unknown: Vec<String>,
    matched: usize,
    conflict: bool,
}

/// Lays the first row of each file over its sample; a file with several rows is a duplicate (the first is kept, the others are the evidence).
fn settle(
    table: &mut Table,
    rows: &[Row],
    by_file: &[Vec<usize>],
    log: &mut Vec<String>,
) -> Settled {
    let mut st = Settled::default();
    for (i, ks) in by_file.iter().enumerate() {
        let Some(&first) = ks.first() else { continue };
        let file = table.samples[i].file.clone();
        st.matched += 1;
        if ks.len() > 1 {
            let key = |k: usize| (&rows[k].name, &rows[k].role, &rows[k].group);
            let differ = ks.iter().any(|&k| key(k) != key(first));
            st.conflict |= differ;
            let nums: Vec<String> = ks.iter().map(|&k| rows[k].n.to_string()).collect();
            let kept = rows[first].n.to_string();
            st.duplicates.push(obj(&[
                ("file", file.clone()),
                ("rows", list(&nums)),
                ("conflict", differ.to_string()),
                ("kept", kept.clone()),
            ]));
            log.push(obj(&[
                ("what", "duplicate_first".into()),
                ("row", kept.clone()),
                ("file", file.clone()),
                ("from", nums.join(",")),
                ("to", kept),
            ]));
        }
        if !apply(&mut table.samples[i], &rows[first], log) {
            st.unknown.push(obj(&[
                ("row", rows[first].n.to_string()),
                ("file", file),
                ("value", rows[first].role.clone()),
            ]));
        }
    }
    st
}

/// A sample name on several files: evidence only, one sample in the two polarities is two files with one name.
fn shared_names(table: &Table) -> Vec<String> {
    let mut shared: Vec<(&str, Vec<&str>)> = Vec::new();
    for s in table.samples.iter().filter(|s| !s.sample_id.is_empty()) {
        match shared.iter_mut().find(|(id, _)| *id == s.sample_id) {
            Some((_, files)) => files.push(&s.file),
            None => shared.push((&s.sample_id, vec![&s.file])),
        }
    }
    let files = |f: &[&str]| quoted(&f.iter().map(|x| x.to_string()).collect::<Vec<_>>());
    shared
        .iter()
        .filter(|(_, f)| f.len() > 1)
        .map(|(id, f)| obj(&[("name", id.to_string()), ("files", files(f))]))
        .collect()
}

/// The table the rows go over: the given `samples/1`, else the samples the names of `files` propose.
fn base_table(v: &Value, input: &str) -> Result<Table> {
    match v.get("samples") {
        Some(doc) => table_from_value(doc),
        None => Ok(Table {
            samples: propose(&files_of(input)?),
            columns: Vec::new(),
        }),
    }
}

/// `{"text", "separator"?, "files": [{"name","sha256"?}] | "samples": <samples/1>}` -> `sample-import/1`: the samples with the rows laid over them and the report
/// (`status` complete | partial, `rows`, `matched`, `unmatched_rows`, `unmatched_files`, `duplicates`, `unknown_roles`, `shared_names`, `choices`).
pub fn import(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let text = v
        .get("text")
        .and_then(|x| x.str())
        .ok_or_else(|| Error::new("error.badRequest"))?;
    let sep = sep_of(&v)?;
    let mut table = base_table(&v, input)?;
    let mut log = Vec::new();
    let rows = read_rows(text, sep, &mut log)?;
    let names: Vec<String> = table.samples.iter().map(|s| s.file.clone()).collect();
    let (by_file, unmatched) = match_rows(&rows, &names, &mut log);
    let st = settle(&mut table, &rows, &by_file, &mut log);
    let free: Vec<String> = (0..names.len())
        .filter(|&i| by_file[i].is_empty())
        .map(|i| names[i].clone())
        .collect();
    let complete = unmatched.is_empty() && free.is_empty() && !st.conflict && st.unknown.is_empty();
    let mut o = Out::default();
    o.raw("{\"report\":{\"choices\":").raw(&list(&log));
    o.raw(",\"duplicates\":").raw(&list(&st.duplicates));
    o.raw(",\"matched\":").int(st.matched as i64);
    o.raw(",\"rows\":").int(rows.len() as i64);
    o.raw(",\"shared_names\":")
        .raw(&list(&shared_names(&table)));
    o.raw(",\"status\":")
        .str(if complete { "complete" } else { "partial" });
    o.raw(",\"unknown_roles\":").raw(&list(&st.unknown));
    o.raw(",\"unmatched_files\":").raw(&quoted(&free));
    o.raw(",\"unmatched_rows\":").raw(&list(&unmatched));
    o.raw("},\"samples\":").raw(&table_to_json(&table));
    o.raw(",\"schema\":").str(SCHEMA).raw("}");
    Ok(o.0)
}
