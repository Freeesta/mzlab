//! `mzlab-cli commands --find WORDS... [--json]`: the commands and Flow steps that do a task, ranked by the words of their name, the task
//! words of `related.rs`, their summary (a step's title) and a step's doc. Each match carries a one-line example, `see_also` and, where a
//! loop of calls has a one-call equivalent, `bulk`. Plain word matching, no network, the same answer on every run (`commands-find/1`).

use mzlab_core::json::Value;
use mzlab_core::jsonb::{text, write, Obj};

use super::related;
use crate::cmd::entries;
use crate::Outcome;

/// How many matches the answer lists at most.
const TOP: usize = 5;

/// Words that say nothing about the task.
const STOP: &[&str] = &[
    "a", "an", "the", "of", "in", "on", "for", "to", "and", "or", "with", "from", "by", "at", "is",
    "it", "as", "into", "how", "do", "my", "me", "what", "which", "this", "that", "use", "using",
    "over", "per", "all", "each", "every", "one", "be", "can", "i", "want",
];

/// One command or step as the search sees it.
pub(crate) struct Item {
    pub kind: &'static str,
    pub name: String,
    pub summary: String,
    pub example: String,
    pub native: bool,
    /// The words of the name, task words, summary and doc, each field with its weight.
    fields: [(u32, Vec<String>); 4],
}

/// The same word for its plural and a few spellings (`s/n` = `snr`, `m/z` = `mz`).
fn stem(w: &str) -> String {
    let n = w.len();
    if n > 5 && w.ends_with("ing") {
        w[..n - 3].to_string()
    } else if n > 4 && w.ends_with("ies") {
        format!("{}y", &w[..n - 3])
    } else if n > 3 && w.ends_with('s') && !w.ends_with("ss") {
        w[..n - 1].to_string()
    } else {
        w.to_string()
    }
}

/// The search words of `s`: lower case, split at anything but letters and digits, stop words and one-letter words dropped.
pub(crate) fn words(s: &str) -> Vec<String> {
    let s = s.to_lowercase().replace("s/n", "snr").replace("m/z", "mz");
    let out: Vec<String> = s
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() > 1 && !STOP.contains(w))
        .map(stem)
        .collect();
    unique(out)
}

/// The words in their first-seen order, each once: a repeated word must not count again.
fn unique(v: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(v.len());
    for w in v {
        if !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

fn item(
    kind: &'static str,
    name: &str,
    summary: &str,
    doc: &str,
    example: String,
    native: bool,
) -> Item {
    let rel = related::of(name);
    let example = rel
        .map(|r| r.example)
        .filter(|e| !e.is_empty())
        .map_or(example, str::to_string);
    Item {
        kind,
        name: name.to_string(),
        summary: summary.to_string(),
        example,
        native,
        fields: [
            (4, words(name)),
            (3, words(rel.map_or("", |r| r.words))),
            (2, words(summary)),
            (1, words(doc)),
        ],
    }
}

/// Every listed command and every Flow step (its highest version), in catalog order.
pub(crate) fn items() -> Vec<Item> {
    let mut v: Vec<Item> = entries()
        .filter(|e| !e.help.is_empty())
        .map(|e| {
            let u = super::usage_of(e);
            let ex = e
                .examples
                .first()
                .map_or_else(|| super::doc::skeleton(&u), |x| x.to_string());
            item("command", e.name, &u.summary, "", ex, true)
        })
        .collect();
    let reg = crate::flow::steps();
    for s in &reg {
        if reg.iter().any(|o| o.id == s.id && o.version > s.version) {
            continue;
        }
        let ex = format!("mzlab-cli flow steps --id {} --template > plan.json", s.id);
        v.push(item(
            "step",
            s.id,
            s.title,
            s.doc,
            ex,
            crate::flow::executable(s.id),
        ));
    }
    v
}

/// The matches of `query` with their score and matched words, best first (ties in catalog order); only items that match a word.
pub(crate) fn search(query: &[String]) -> Vec<(Item, f64, Vec<String>)> {
    let all = items();
    let n = all.len() as f64;
    // a word that many items carry tells less: weight ln(1 + N / items with it)
    let idf = |w: &str| {
        let df = all
            .iter()
            .filter(|i| i.fields.iter().any(|(_, ws)| ws.iter().any(|x| x == w)))
            .count()
            .max(1) as f64;
        (1.0 + n / df).ln()
    };
    let weights: Vec<(String, f64)> = unique(query.to_vec())
        .into_iter()
        .map(|w| {
            let f = idf(&w);
            (w, f)
        })
        .collect();
    let mut hits: Vec<(Item, f64, Vec<String>)> = all
        .into_iter()
        .filter_map(|it| {
            let mut score = 0.0;
            let mut matched = Vec::new();
            for (w, f) in &weights {
                let best = it
                    .fields
                    .iter()
                    .filter(|(_, ws)| ws.contains(w))
                    .map(|(k, _)| *k)
                    .max();
                if let Some(k) = best {
                    score += f64::from(k) * f;
                    matched.push(w.clone());
                }
            }
            // a step only the page runs comes after what this binary runs
            if !it.native {
                score *= 0.5;
            }
            (score > 0.0).then(|| (it, (score * 100.0).round() / 100.0, matched))
        })
        .collect();
    // on equal scores the one call for many goes before a one-at-a-time command (its `bulk` is another command)
    let one_at_a_time = |it: &Item| related::of(&it.name).is_some_and(related::Rel::elsewhere);
    hits.sort_by(|a, b| {
        b.1.total_cmp(&a.1)
            .then_with(|| one_at_a_time(&a.0).cmp(&one_at_a_time(&b.0)))
    });
    hits.truncate(TOP);
    hits
}

fn strings(v: &[&str]) -> Value {
    Value::Arr(v.iter().map(|s| text(s)).collect())
}

/// The answer of `commands --find WORDS...`: `commands-find/1` with `json`, else one line per match and its example under it.
pub(crate) fn run(query: &[String], json: bool) -> Outcome {
    let q = unique(query.iter().flat_map(|w| words(w)).collect());
    if q.is_empty() {
        let err =
            mzlab_core::Error::new("err.cli.usage").with("message", "--find needs task words");
        return Outcome::fail_error(&err)
            .with_hint("mzlab-cli commands --find integrate sim channels --json");
    }
    let hits = search(&q);
    if hits.is_empty() {
        let err = mzlab_core::Error::new("err.cli.find.none").with("words", q.join(" "));
        return Outcome::fail_error(&err)
            .with_hint("mzlab-cli commands lists every command, mzlab-cli flow steps every step");
    }
    if !json {
        let lines: String = hits
            .iter()
            .map(|(it, _, _)| {
                format!(
                    "{} ({})  {}\n    {}\n",
                    it.name, it.kind, it.summary, it.example
                )
            })
            .collect();
        return Outcome::ok(lines.into_bytes());
    }
    let matches = hits
        .iter()
        .map(|(it, score, matched)| {
            let rel = related::of(&it.name);
            let mut o = Obj::new()
                .put("kind", text(it.kind))
                .put("name", text(&it.name))
                .put("summary", text(&it.summary))
                .put("example", text(&it.example))
                .put("score", Value::Num(*score))
                .put(
                    "matched",
                    Value::Arr(matched.iter().map(|w| text(w)).collect()),
                )
                .put("see_also", strings(rel.map_or(&[], |r| r.also)));
            if let Some(b) = rel.map(|r| r.bulk).filter(|b| !b.is_empty()) {
                o = o.put("bulk", text(b));
            }
            if it.kind == "step" {
                o = o.b("executable_native", it.native);
            }
            o.done()
        })
        .collect();
    let doc = Obj::new()
        .put("schema", text("commands-find/1"))
        .put("query", Value::Arr(q.iter().map(|w| text(w)).collect()))
        .put("matches", Value::Arr(matches))
        .done();
    Outcome::ok((write(&doc) + "\n").into_bytes())
}

#[cfg(test)]
#[path = "find_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "find_offset_tests.rs"]
mod offset_tests;

#[cfg(test)]
#[path = "find_states_tests.rs"]
mod states_tests;
