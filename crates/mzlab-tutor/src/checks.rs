//! The checks on a model answer, in Rust, after it is written: every `[n]` names a block that was sent, every statement carries one, every number is in a
//! block the sentence cites, and the answer does not copy a hidden exercise answer. A failed check never deletes the answer: the page marks the sentence.
//! Used by the Worker and by `eval`.
use crate::answer::{Block, NOT_COVERED_EN, NOT_COVERED_IT};
use crate::blocchi::shingles;
use serde_json::Value;
use std::collections::HashSet;

/// Share of a hidden answer (by 5-word shingles) an answer may reproduce.
pub const OVERLAP_MAX: f64 = 0.15;

#[derive(Clone, Debug, PartialEq)]
pub struct Checks {
    /// Every `[n]` is a sent block, and every statement (not a question, not the fixed sentence) cites one.
    pub cites_ok: bool,
    /// Indices of the sentences, in order, that fail a check.
    pub bad_sentences: Vec<usize>,
    /// Every number of a sentence is in a block that sentence cites (any sent block when it cites none).
    pub numbers_ok: bool,
    /// The largest share of a hidden answer found in the answer.
    pub solution_overlap: f64,
}

impl Checks {
    pub fn solution_ok(&self) -> bool {
        self.solution_overlap < OVERLAP_MAX
    }
}

/// The shingle sets of the hidden answers of a `teoria-blocchi/1` value.
pub fn hidden_sets(blocks: &Value) -> Vec<Vec<String>> {
    blocks["blocks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|b| b["hidden_shingles"].as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .collect()
}

/// The citations of a sentence: `[3]`, `[1, 2]`; a bracket with anything else (`[M+H]+`) is not one. Returns the numbers and the text without them.
fn cites(s: &str) -> (Vec<u32>, String) {
    let (mut found, mut rest) = (Vec::new(), String::new());
    let mut i = 0;
    while i < s.len() {
        let tail = &s[i..];
        if tail.starts_with('[') {
            if let Some(end) = tail.find(']').filter(|&e| e <= 14) {
                let inner = &tail[1..end];
                let nums: Option<Vec<u32>> = inner
                    .split(',')
                    .map(|x| x.trim().parse::<u32>().ok())
                    .collect();
                if let Some(n) = nums.filter(|n| !n.is_empty()) {
                    found.extend(n);
                    i += end + 1;
                    continue;
                }
            }
        }
        let c = tail.chars().next().unwrap_or(' ');
        rest.push(c);
        i += c.len_utf8();
    }
    (found, rest)
}

/// The sentences of an answer: cut at a full stop, `!` or `?` followed by a space or the end, and at line ends. A piece with no letters or digits
/// (a lone `[2]` after the full stop) joins the sentence before it.
fn sentences(answer: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in answer.lines() {
        let chars: Vec<char> = line.chars().collect();
        let mut cur = String::new();
        for (i, &c) in chars.iter().enumerate() {
            cur.push(c);
            if matches!(c, '.' | '!' | '?') && chars.get(i + 1).is_none_or(|n| n.is_whitespace()) {
                push(&mut out, std::mem::take(&mut cur));
            }
        }
        push(&mut out, cur);
    }
    out
}

fn push(out: &mut Vec<String>, piece: String) {
    let piece = piece.trim().to_string();
    if piece.is_empty() {
        return;
    }
    let (_, bare) = cites(&piece);
    if !bare.chars().any(char::is_alphanumeric) {
        if let Some(last) = out.last_mut() {
            last.push(' ');
            last.push_str(&piece);
            return;
        }
    }
    out.push(piece);
}

/// The numbers of a text as canonical strings (`0,95` and `0.95` meet); digits glued to a letter (`C18`, `H2O`) are part of a name and are skipped, and so
/// is the list marker at the start of a line.
pub fn numbers(text: &str) -> Vec<String> {
    let t = text.trim_start();
    let t = t
        .find(|c: char| !c.is_ascii_digit())
        .filter(|&i| i > 0 && t[i..].starts_with(['.', ')']) && t[i + 1..].starts_with(' '))
        .map_or(t, |i| &t[i + 1..]);
    let c: Vec<char> = t.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if !c[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < c.len() && c[i].is_ascii_digit() {
            i += 1;
        }
        if i + 1 < c.len() && matches!(c[i], '.' | ',') && c[i + 1].is_ascii_digit() {
            i += 1;
            while i < c.len() && c[i].is_ascii_digit() {
                i += 1;
            }
        }
        let glued = start > 0 && c[start - 1].is_alphabetic();
        if !glued {
            let raw: String = c[start..i].iter().collect::<String>().replace(',', ".");
            out.push(raw.parse::<f64>().map_or(raw, |x| x.to_string()));
        }
    }
    out
}

fn fixed_sentence(s: &str) -> bool {
    s.contains(NOT_COVERED_IT.trim_end_matches('.'))
        || s.contains(NOT_COVERED_EN.trim_end_matches('.'))
}

/// Runs the checks of a finished answer against the blocks that were sent (block `n` is `sent[n - 1]`) and the hidden answers of the corpus.
pub fn run(answer: &str, sent: &[Block], hidden: &[Vec<String>]) -> Checks {
    let block_numbers: Vec<HashSet<String>> = sent
        .iter()
        .map(|b| {
            numbers(&format!("{} {}", b.title, b.text.replace('^', " ")))
                .into_iter()
                .collect()
        })
        .collect();
    let (mut cites_ok, mut numbers_ok) = (true, true);
    let mut bad_sentences = Vec::new();
    for (i, s) in sentences(answer).iter().enumerate() {
        let (cited, bare) = cites(s);
        let invalid = cited.iter().any(|&n| n == 0 || n as usize > sent.len());
        let words = bare
            .split_whitespace()
            .filter(|w| w.chars().any(char::is_alphabetic))
            .count();
        let statement = !bare.trim_end().ends_with('?') && !fixed_sentence(s) && words >= 3;
        let uncited = statement && cited.is_empty();
        let nums = numbers(&bare);
        let wrong_numbers = nums.iter().any(|n| {
            if cited.is_empty() || invalid {
                !block_numbers.iter().any(|set| set.contains(n))
            } else {
                !cited
                    .iter()
                    .any(|&c| block_numbers[c as usize - 1].contains(n))
            }
        });
        if invalid || uncited {
            cites_ok = false;
        }
        if wrong_numbers {
            numbers_ok = false;
        }
        if invalid || uncited || wrong_numbers {
            bad_sentences.push(i);
        }
    }
    let mine: HashSet<String> = shingles(answer).into_iter().collect();
    let solution_overlap = hidden
        .iter()
        .filter(|h| !h.is_empty())
        .map(|h| h.iter().filter(|s| mine.contains(*s)).count() as f64 / h.len() as f64)
        .fold(0.0, f64::max);
    Checks {
        cites_ok,
        bad_sentences,
        numbers_ok,
        solution_overlap,
    }
}

/// Every block number the answer cites, in order of appearance (repeats kept).
pub fn cited(answer: &str) -> Vec<u32> {
    cites(answer).0
}
