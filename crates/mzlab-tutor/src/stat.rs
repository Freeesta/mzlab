//! The counters for the teacher (`tutorstat/1`, level a: no text, no student, no IP): per day, which blocks were chosen, how many answers fell to the
//! passages mode or found nothing, intents, negative votes by reason, tokens and cost. `share_text` is level b: the cleaned text of a question the
//! student agreed to share. Used by the Worker.
use crate::pick::Intent;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub const SCHEMA: &str = "tutorstat/1";
/// The reasons a vote can give; anything else is counted as `other`.
pub const REASONS: &[&str] = &["wrong", "unclear", "solved_exercise", "other"];
pub const SHARE_MAX: usize = 600;
/// What replaces an email, a phone number or a long number.
const MASK: &str = "[…]";

/// What happened, as the Worker reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    /// A question answered by the model (`passi` false) or with the passages only.
    Answer {
        passi: bool,
        intent: Intent,
        /// Ids of the blocks the answer received.
        blocks: Vec<String>,
        /// No block matched at all: the Teoria does not cover the question.
        uncovered: bool,
        tokens_in: u64,
        tokens_out: u64,
        cost_micros: u64,
    },
    /// A vote on an answer, with the ids of the blocks it cited.
    Vote {
        negative: bool,
        reason: Option<String>,
        cited: Vec<String>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Day {
    questions: u64,
    passi: u64,
    uncovered: u64,
    intents: BTreeMap<String, u64>,
    blocks: BTreeMap<String, u64>,
    votes_up: u64,
    votes_down: BTreeMap<String, u64>,
    down_blocks: BTreeMap<String, u64>,
    tokens_in: u64,
    tokens_out: u64,
    cost_micros: u64,
}

fn bump(m: &mut BTreeMap<String, u64>, k: &str) {
    *m.entry(k.to_string()).or_insert(0) += 1;
}

/// The counters of one course.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stat {
    pub course: String,
    days: BTreeMap<String, Day>,
}

impl Stat {
    pub fn new(course: &str) -> Stat {
        Stat {
            course: course.to_string(),
            days: BTreeMap::new(),
        }
    }

    /// Counts a record on `day` (`YYYY-MM-DD`).
    pub fn add(&mut self, day: &str, r: &Record) {
        let d = self.days.entry(day.to_string()).or_default();
        match r {
            Record::Answer {
                passi,
                intent,
                blocks,
                uncovered,
                tokens_in,
                tokens_out,
                cost_micros,
            } => {
                d.questions += 1;
                d.passi += u64::from(*passi);
                d.uncovered += u64::from(*uncovered);
                bump(&mut d.intents, intent.name());
                for b in blocks {
                    bump(&mut d.blocks, b);
                }
                d.tokens_in += tokens_in;
                d.tokens_out += tokens_out;
                d.cost_micros += cost_micros;
            }
            Record::Vote {
                negative,
                reason,
                cited,
            } => {
                if *negative {
                    let why = reason
                        .as_deref()
                        .filter(|x| REASONS.contains(x))
                        .unwrap_or("other");
                    bump(&mut d.votes_down, why);
                    for b in cited {
                        bump(&mut d.down_blocks, b);
                    }
                } else {
                    d.votes_up += 1;
                }
            }
        }
    }
}

fn counts(m: &BTreeMap<String, u64>) -> Value {
    Value::Object(m.iter().map(|(k, v)| (k.clone(), json!(v))).collect())
}

impl Stat {
    pub fn to_json(&self) -> Value {
        let mut days = Map::new();
        for (k, d) in &self.days {
            days.insert(
                k.clone(),
                json!({
                    "blocks": counts(&d.blocks),
                    "cost_micros": d.cost_micros,
                    "down_blocks": counts(&d.down_blocks),
                    "intents": counts(&d.intents),
                    "passi": d.passi,
                    "questions": d.questions,
                    "tokens_in": d.tokens_in,
                    "tokens_out": d.tokens_out,
                    "uncovered": d.uncovered,
                    "votes_down": counts(&d.votes_down),
                    "votes_up": d.votes_up,
                }),
            );
        }
        json!({"course": self.course, "days": days, "schema": SCHEMA})
    }
}

/// Level b: the text of a question the student agreed to share, without email addresses, phone numbers and numbers of 6 or more digits (a student
/// number); `None` when the text is longer than 600 characters or nothing readable is left. A run of digit groups split by single spaces or hyphens with
/// 8 or more digits counts as a phone number even when it is a series of values: for a stored question, too much is cleaned rather than too little.
pub fn share_text(q: &str) -> Option<String> {
    if q.chars().count() > SHARE_MAX {
        return None;
    }
    let out = mask_digits(&mask_emails(q));
    let out = out.trim().to_string();
    (out.chars().filter(|c| c.is_alphabetic()).count() >= 3).then_some(out)
}

fn mask_emails(q: &str) -> String {
    let word_end =
        |c: char| c.is_whitespace() || matches!(c, ',' | ';' | '<' | '>' | '(' | ')' | '"');
    q.split_inclusive(word_end)
        .map(|piece| {
            let body = piece.trim_end_matches(word_end);
            let tail = &piece[body.len()..];
            let at = body.find('@');
            let is_mail = at.is_some_and(|i| {
                i > 0 && body[i + 1..].contains('.') && !body[i + 1..].starts_with('.')
            });
            if is_mail {
                // a full stop that closes the sentence stays outside the address
                let dot = if body.ends_with(['.', '!', '?']) {
                    &body[body.len() - 1..]
                } else {
                    ""
                };
                format!("{MASK}{dot}{tail}")
            } else {
                piece.to_string()
            }
        })
        .collect()
}

/// Replaces digit runs of 6 or more, and chains of digit groups (optional `+`, groups split by one space or hyphen) with 8 or more digits.
fn mask_digits(q: &str) -> String {
    let c: Vec<char> = q.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        let plus = c[i] == '+' && c.get(i + 1).is_some_and(char::is_ascii_digit);
        if !(c[i].is_ascii_digit() || plus) {
            out.push(c[i]);
            i += 1;
            continue;
        }
        let start = i;
        let (mut j, mut digits, mut longest) = (if plus { i + 1 } else { i }, 0usize, 0usize);
        loop {
            let g = j;
            while j < c.len() && c[j].is_ascii_digit() {
                j += 1;
            }
            digits += j - g;
            longest = longest.max(j - g);
            let joins = j + 1 < c.len() && matches!(c[j], ' ' | '-') && c[j + 1].is_ascii_digit();
            if joins {
                j += 1;
            } else {
                break;
            }
        }
        if digits >= 8 || longest >= 6 {
            out.push_str(MASK);
        } else {
            out.extend(&c[start..j]);
        }
        i = j;
    }
    out
}
