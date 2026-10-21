//! The first model step: from 20 BM25 candidates the small model returns the intent of the question, Italian search words and at most 5 block ids
//! (`request` builds the call, `parse` reads the strict JSON reply, `choose` turns the reply into the final ids). Used by the Worker and by `eval`.
use crate::ask::Ask;
use crate::cerca::Index;
use crate::{esc, with_options};
use mzlab_core::error::Error;
use mzlab_llm::conv::{Conversation, Message, Part, Role};
use mzlab_llm::provider::{request as build, Provider, Request};
use serde_json::Value;

/// The fixed text of the choice step (English, SHA-256 in `system_sha`).
pub const SYSTEM: &str = include_str!("../data/tutor-scelta-1.md");
pub const MAX_PICK: usize = 5;
pub const MAX_KEYWORDS: usize = 8;
/// Candidates shown to the model, and how many characters of each block.
pub const CANDIDATES: usize = 20;
pub const SNIPPET: usize = 200;
/// The reply is a short JSON object.
const MAX_TOKENS: u32 = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Concetto,
    Esercizio,
    DatiLab,
    FuoriTema,
    Iniezione,
}

impl Intent {
    pub fn name(self) -> &'static str {
        match self {
            Intent::Concetto => "concetto",
            Intent::Esercizio => "esercizio",
            Intent::DatiLab => "dati_lab",
            Intent::FuoriTema => "fuori_tema",
            Intent::Iniezione => "iniezione",
        }
    }

    pub fn from_name(n: &str) -> Option<Intent> {
        [
            Intent::Concetto,
            Intent::Esercizio,
            Intent::DatiLab,
            Intent::FuoriTema,
            Intent::Iniezione,
        ]
        .into_iter()
        .find(|i| i.name() == n)
    }
}

/// A block as the model sees it in the choice step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    pub title: String,
    pub snippet: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick {
    pub intent: Intent,
    pub keywords_it: Vec<String>,
    pub ids: Vec<String>,
}

fn one_line(t: &str, max: usize) -> String {
    let flat: String = t
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .take(max)
        .collect();
    flat.trim().to_string()
}

/// The candidates for the search hits `(id, score)`, in order; ids the blocks file does not know are skipped.
pub fn candidates(blocks: &Value, hits: &[(String, f64)]) -> Vec<Candidate> {
    let all = blocks["blocks"].as_array().map_or(&[][..], Vec::as_slice);
    hits.iter()
        .filter_map(|(id, _)| all.iter().find(|b| b["id"] == id.as_str()))
        .map(|b| Candidate {
            id: b["id"].as_str().unwrap_or("").to_string(),
            title: one_line(b["title"].as_str().unwrap_or(""), 120),
            snippet: one_line(b["text"].as_str().unwrap_or(""), SNIPPET),
        })
        .collect()
}

/// The request of the choice step: the question and the candidates as data, no thinking, the given effort.
pub fn request(
    ask: &Ask,
    cands: &[Candidate],
    provider: &Provider,
    model: &str,
    effort: &str,
) -> Result<Request, Error> {
    let mut user = format!("Question language: {}\n", ask.lang);
    if let Some(w) = &ask.where_ {
        user.push_str(&format!("Section open: {}\n", esc(w)));
    }
    user.push_str(&format!(
        "<question>{}</question>\n<candidates>\n",
        esc(&ask.question)
    ));
    for c in cands {
        user.push_str(&format!(
            "{}\t{}\t{}\n",
            esc(&c.id),
            esc(&c.title),
            esc(&c.snippet)
        ));
    }
    user.push_str("</candidates>");
    let conv = Conversation {
        provider: provider.id.clone(),
        model: model.to_string(),
        system: SYSTEM.to_string(),
        max_output_tokens: Some(MAX_TOKENS),
        messages: vec![Message {
            role: Role::User,
            parts: vec![Part::Text(user)],
        }],
    };
    with_options(build(provider, &conv, &[])?, provider, false, effort)
}

/// Strips one surrounding ```json fence, the only decoration tolerated.
fn unfence(t: &str) -> &str {
    let t = t.trim();
    match t.strip_prefix("```") {
        Some(r) => r
            .trim_start_matches("json")
            .trim()
            .strip_suffix("```")
            .unwrap_or(r)
            .trim(),
        None => t,
    }
}

/// Reads the reply of the choice step. Strict: one JSON object with a known intent, strings only; ids that are not candidates are dropped (and repeats),
/// at most 5 are kept. Anything else is `err.tutor.pick`.
pub fn parse(text: &str, cands: &[Candidate]) -> Result<Pick, Error> {
    let fail = |f: &str| Error::new("err.tutor.pick").with("field", f);
    let v: Value = serde_json::from_str(unfence(text)).map_err(|_| fail("json"))?;
    let intent = v["intent"]
        .as_str()
        .and_then(Intent::from_name)
        .ok_or_else(|| fail("intent"))?;
    let strings = |key: &str| -> Result<Vec<String>, Error> {
        v[key]
            .as_array()
            .ok_or_else(|| fail(key))?
            .iter()
            .map(|x| x.as_str().map(str::to_string).ok_or_else(|| fail(key)))
            .collect()
    };
    let mut keywords_it = strings("keywords_it")?;
    keywords_it.retain(|k| !k.trim().is_empty() && k.chars().count() <= 40);
    keywords_it.truncate(MAX_KEYWORDS);
    let mut ids: Vec<String> = Vec::new();
    for id in strings("pick")? {
        if cands.iter().any(|c| c.id == id) && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids.truncate(MAX_PICK);
    Ok(Pick {
        intent,
        keywords_it,
        ids,
    })
}

/// The ids the answer step receives. The picked ids come first; an English question adds what its Italian keywords find (one more search), and an
/// empty choice falls back on the best hits, so the model always has the nearest paragraphs to point at.
pub fn choose(index: &Index, ask: &Ask, pick: &Pick, hits: &[(String, f64)]) -> Vec<String> {
    let mut ids = pick.ids.clone();
    if ask.lang == "en" && !pick.keywords_it.is_empty() {
        let q = pick.keywords_it.join(" ");
        for (id, _) in index.search(&q, ask.where_.as_deref(), MAX_PICK) {
            if ids.len() < MAX_PICK && !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    if ids.is_empty() {
        ids = hits.iter().take(3).map(|(id, _)| id.clone()).collect();
    }
    ids.truncate(MAX_PICK);
    ids
}

/// SHA-256 of the fixed text, so a change of the text is visible in the logs.
pub fn system_sha() -> String {
    mzlab_core::budget::sha256_hex(SYSTEM.as_bytes())
}
