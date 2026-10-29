//! The Teoria pages cut into blocks (`teoria-blocchi/1`): paragraphs under the same h2/h3 up to ~800 characters, one block per glossary entry,
//! exercises (`details.q`, `details.poe`) as the bare question plus the fingerprint of the hidden answer (never its text).
//! Read by `cerca` and by the tutor Worker; the output is canonical (sorted keys, one block per line), so the same pages give the same bytes.
use crate::pagina::read_page;
use mzlab_core::budget::sha256_hex;
use mzlab_libro::norm;
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashMap};

/// Target size of a block (characters) and the hard ceiling for a single piece that cannot be cut at a sentence.
const TARGET: usize = 800;
const CEILING: usize = 1100;
/// Words per shingle of the hidden answer.
const SHINGLE: usize = 5;

#[derive(Default)]
pub(crate) struct Page {
    pub(crate) file: String,
    pub(crate) chapter: String,
    pub(crate) title: String,
    pub(crate) blocks: Vec<Block>,
    pub(crate) counters: HashMap<String, usize>,
    // section being read
    pub(crate) sect_title: String,
    pub(crate) sect_anchor: String,
    pub(crate) paras: Vec<(String, bool)>,
}

pub(crate) struct Block {
    id: String,
    anchor: String,
    pub(crate) title: String,
    kind: &'static str,
    text: String,
    hidden: Option<String>,
}

impl Page {
    pub(crate) fn push(
        &mut self,
        anchor: &str,
        title: &str,
        kind: &'static str,
        text: String,
        hidden: Option<String>,
    ) {
        let k = self.counters.entry(anchor.to_string()).or_insert(0);
        *k += 1;
        self.blocks.push(Block {
            id: format!("{}#{}.{}", self.file, anchor, k),
            anchor: anchor.to_string(),
            title: title.to_string(),
            kind,
            text,
            hidden,
        });
    }

    /// Turns the paragraphs collected under the current section into blocks.
    pub(crate) fn emit_section(&mut self) {
        let paras = std::mem::take(&mut self.paras);
        let (anchor, title) = (self.sect_anchor.clone(), self.sect_title.clone());
        let mut cur = String::new();
        let mut fig = false;
        let mut done: Vec<(String, bool)> = Vec::new();
        for (p, f) in paras {
            for piece in cut(&p) {
                if !cur.is_empty() && cur.chars().count() + 1 + piece.chars().count() > TARGET {
                    done.push((std::mem::take(&mut cur), fig));
                    fig = false;
                }
                if !cur.is_empty() {
                    cur.push('\n');
                }
                cur.push_str(&piece);
                fig |= f;
            }
        }
        if !cur.is_empty() {
            done.push((cur, fig));
        }
        for (text, fig) in done {
            self.push(
                &anchor,
                &title,
                if fig { "figura" } else { "testo" },
                text,
                None,
            );
        }
    }
}

/// A paragraph cut into pieces of at most ~800 characters, at sentence ends when it can.
pub(crate) fn cut(mut p: &str) -> Vec<String> {
    let mut out = Vec::new();
    while p.chars().count() > TARGET {
        let idx_at = |n: usize| p.char_indices().nth(n).map_or(p.len(), |(i, _)| i);
        let (target_byte, ceiling_byte) = (idx_at(TARGET), idx_at(CEILING));
        let sentence = p[..target_byte]
            .char_indices()
            .rev()
            .find(|&(i, c)| {
                matches!(c, '.' | '!' | '?' | ';') && p[i + c.len_utf8()..].starts_with(' ')
            })
            .map(|(i, c)| i + c.len_utf8());
        let at = sentence
            .or_else(|| {
                p[target_byte..ceiling_byte]
                    .find(' ')
                    .map(|i| target_byte + i)
            })
            .unwrap_or(ceiling_byte);
        out.push(p[..at].trim().to_string());
        p = p[at..].trim_start();
    }
    let tail = p.trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out
}

/// `sha` of a text: first 12 hex digits of the SHA-256 of its normalised form.
fn sha12(text: &str) -> String {
    sha256_hex(norm(text).as_bytes())[..12].to_string()
}

/// Hashed 5-word shingles of a text (6 hex digits each, sorted, unique): enough to measure how much of a hidden answer a reply copies, without the words.
pub fn shingles(text: &str) -> Vec<String> {
    let w = crate::words(text);
    let set: BTreeSet<String> = w
        .windows(SHINGLE)
        .map(|g| sha256_hex(g.join(" ").as_bytes())[..6].to_string())
        .collect();
    set.into_iter().collect()
}

fn block_json(file: &str, chapter: &str, b: &Block) -> Value {
    let mut m = Map::new();
    m.insert("anchor".into(), json!(b.anchor));
    m.insert("chapter".into(), json!(chapter));
    m.insert("file".into(), json!(file));
    if let Some(h) = &b.hidden {
        m.insert("hidden_sha".into(), json!(sha12(h)));
        m.insert("hidden_shingles".into(), json!(shingles(h)));
    }
    m.insert("id".into(), json!(b.id));
    m.insert("kind".into(), json!(b.kind));
    m.insert("sha".into(), json!(sha12(&b.text)));
    m.insert("text".into(), json!(b.text));
    m.insert("title".into(), json!(b.title));
    Value::Object(m)
}

/// The blocks of the pages `(file name, html)` as a `teoria-blocchi/1` value; pages that only redirect are skipped. Pages are taken in name order.
pub fn build(pages: &[(String, String)]) -> Result<Value, String> {
    let mut sorted: Vec<&(String, String)> = pages.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut blocks = Vec::new();
    let mut chapters = Map::new();
    let mut seen = BTreeSet::new();
    for (name, html) in sorted {
        let Some(pg) = read_page(name, html) else {
            continue;
        };
        let file = name.trim_end_matches(".html");
        chapters.insert(file.to_string(), json!(pg.title));
        for b in &pg.blocks {
            if !seen.insert(b.id.clone()) {
                return Err(format!("duplicate block id {}", b.id));
            }
            blocks.push(block_json(file, &pg.chapter, b));
        }
    }
    let n = blocks.len();
    let mut top = Map::new();
    top.insert("blocks".into(), Value::Array(blocks));
    top.insert("chapters".into(), Value::Object(chapters));
    top.insert("n".into(), json!(n));
    top.insert("schema".into(), json!(crate::SCHEMA));
    Ok(Value::Object(top))
}

/// Canonical text of the value `build` returns: keys in alphabetical order, one block per line, a final newline.
pub fn to_text(v: &Value) -> String {
    let blocks: Vec<String> = v["blocks"]
        .as_array()
        .map_or_else(Vec::new, |a| a.iter().map(Value::to_string).collect());
    format!(
        "{{\"blocks\":[\n{}\n],\"chapters\":{},\"n\":{},\"schema\":{}}}\n",
        blocks.join(",\n"),
        v["chapters"],
        v["n"],
        v["schema"]
    )
}

#[cfg(test)]
mod cut_tests;
