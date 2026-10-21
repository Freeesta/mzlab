//! BM25 search over the blocks of `teoria-blocchi/1`: words normalised like the Teoria search (`cerca.js`), a light stem, the synonyms of the
//! Teoria index (`mzlab_libro::sinonimi`) added to the query at half weight, and +20 % for the blocks of the section the student has open.
//! Used by the tutor Worker to pick the blocks it shows the model, and by `recall_at` to measure the retrieval on the eval cases.
use crate::words;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

const K1: f64 = 1.2;
const B: f64 = 0.75;
const SECTION_BOOST: f64 = 1.2;
const SYNONYM_WEIGHT: f64 = 0.5;

/// Words too common to tell blocks apart (Italian and English).
const STOP: &[&str] = &[
    "il", "lo", "la", "i", "gli", "le", "un", "uno", "una", "di", "a", "da", "in", "con", "su",
    "per", "tra", "fra", "e", "ed", "o", "che", "chi", "cui", "non", "si", "e'", "del", "dello",
    "della", "dei", "degli", "delle", "al", "allo", "alla", "ai", "agli", "alle", "dal", "dalla",
    "nel", "nella", "nei", "sul", "sulla", "come", "cosa", "cos", "qual", "quale", "quali",
    "perche", "ha", "ho", "sono", "puo", "the", "a", "an", "of", "in", "on", "to", "and", "or",
    "is", "are", "what", "how", "why", "does", "do", "for", "with", "by", "it", "this", "that",
    "from", "at", "be", "can", "which", "my", "me", "i",
];

/// A word reduced to its stem: final vowel dropped, then cut at 6 characters (so "frammentazione", "frammentazioni" and "frammentare" meet).
pub fn stem(w: &str) -> String {
    let mut s: &str = w;
    if s.len() > 5 && s.ends_with(['a', 'e', 'i', 'o']) {
        s = &s[..s.len() - 1];
    }
    if s.len() > 6 && s.ends_with('s') {
        s = &s[..s.len() - 1];
    }
    s.chars().take(7).collect()
}

fn terms(text: &str) -> Vec<String> {
    words(text)
        .into_iter()
        .filter(|w| !STOP.contains(&w.as_str()))
        .map(|w| stem(&w))
        .collect()
}

struct Doc {
    id: String,
    section: String,
    file: String,
    tf: HashMap<String, f64>,
    len: f64,
}

/// The search index: built once from the blocks value, then queried.
pub struct Index {
    docs: Vec<Doc>,
    df: HashMap<String, usize>,
    avg: f64,
}

impl Index {
    pub fn new(blocks: &Value) -> Index {
        let mut docs = Vec::new();
        let mut df: HashMap<String, usize> = HashMap::new();
        for b in blocks["blocks"].as_array().into_iter().flatten() {
            let s = |k: &str| b[k].as_str().unwrap_or("").to_string();
            let mut tf: HashMap<String, f64> = HashMap::new();
            let mut len = 0.0;
            // the title counts twice: a section named like the question is the best place to look
            for t in terms(&s("text"))
                .into_iter()
                .chain(terms(&s("title")))
                .chain(terms(&s("title")))
            {
                *tf.entry(t).or_insert(0.0) += 1.0;
                len += 1.0;
            }
            for t in tf.keys() {
                *df.entry(t.clone()).or_insert(0) += 1;
            }
            docs.push(Doc {
                id: s("id"),
                section: format!("{}#{}", s("file"), s("anchor")),
                file: s("file"),
                tf,
                len,
            });
        }
        let avg = docs.iter().map(|d| d.len).sum::<f64>() / docs.len().max(1) as f64;
        Index { docs, df, avg }
    }

    /// The query as weighted stems: its own words at weight 1, the synonyms of the matching keys at half.
    fn query_terms(query: &str) -> BTreeMap<String, f64> {
        let mut q: BTreeMap<String, f64> = BTreeMap::new();
        for t in terms(query) {
            q.insert(t, 1.0);
        }
        let key = mzlab_libro::norm(query);
        for (k, syn) in mzlab_libro::sinonimi() {
            if key.starts_with(k) || key.contains(&format!(" {k}")) {
                for t in syn.iter().flat_map(|w| terms(w)) {
                    q.entry(t).or_insert(SYNONYM_WEIGHT);
                }
            }
        }
        q
    }

    /// The best `k` blocks `(id, score)` for the question; `where_` is the section the student has open (`file#anchor`, or just `file`).
    pub fn search(&self, query: &str, where_: Option<&str>, k: usize) -> Vec<(String, f64)> {
        let q = Self::query_terms(query);
        let n = self.docs.len() as f64;
        let mut hits: Vec<(usize, f64)> = Vec::new();
        for (i, d) in self.docs.iter().enumerate() {
            let mut s = 0.0;
            for (t, w) in &q {
                let Some(f) = d.tf.get(t) else { continue };
                let df = *self.df.get(t).unwrap_or(&0) as f64;
                let idf = (1.0 + (n - df + 0.5) / (df + 0.5)).ln();
                s += w * idf * f * (K1 + 1.0) / (f + K1 * (1.0 - B + B * d.len / self.avg));
            }
            if s > 0.0 {
                if where_.is_some_and(|w| {
                    if w.contains('#') {
                        d.section == w
                    } else {
                        d.file == w
                    }
                }) {
                    s *= SECTION_BOOST;
                }
                hits.push((i, s));
            }
        }
        hits.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        hits.into_iter()
            .take(k)
            .map(|(i, s)| (self.docs[i].id.clone(), s))
            .collect()
    }

    /// `file#anchor` of a block id.
    fn section_of(&self, id: &str) -> Option<&str> {
        self.docs
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.section.as_str())
    }
}

/// Share of the eval cases (those with `expect_anchors`) whose expected section is among the sections of the top-`k` blocks.
/// An expected anchor is `file#anchor`, or `file` alone for any section of the chapter.
pub fn recall_at(index: &Index, cases: &[Value], k: usize) -> f64 {
    let (mut hit, mut total) = (0usize, 0usize);
    for c in cases {
        let want: Vec<&str> = c["expect_anchors"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if want.is_empty() {
            continue;
        }
        total += 1;
        let q = c["question"].as_str().unwrap_or("");
        let found = index
            .search(q, None, k)
            .iter()
            .filter_map(|(id, _)| index.section_of(id))
            .any(|sec| {
                want.iter().any(|w| {
                    if w.contains('#') {
                        *w == sec
                    } else {
                        sec.split('#').next() == Some(*w)
                    }
                })
            });
        hit += found as usize;
    }
    if total == 0 {
        1.0
    } else {
        hit as f64 / total as f64
    }
}
