//! The request of a student, `tutor-ask/1`, parsed with the limits of the contract: the question, the open section, at most two earlier turns.
//! Used by the Worker (first thing it does) and by `eval`. Errors are `err.tutor.*` keys, never texts.
use crate::bad;
use mzlab_core::error::Error;
use serde_json::Value;

pub const MAX_QUESTION: usize = 600;
pub const MAX_TURNS: usize = 2;
/// Characters of all the earlier questions and answers together.
pub const MAX_HISTORY: usize = 1500;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Turn {
    pub question: String,
    pub answer: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    pub course: String,
    /// 128 random bits as 32 hex digits, made in the browser.
    pub client: String,
    /// `it` or `en`.
    pub lang: String,
    pub question: String,
    /// The section the student has open: `file#anchor` or `file`.
    pub where_: Option<String>,
    pub history: Vec<Turn>,
    /// The student agrees to share this question (level b).
    pub share: bool,
}

const KEYS: &[&str] = &[
    "course", "client", "lang", "question", "where", "history", "share",
];

fn too_long(field: &str) -> Error {
    Error::new("err.tutor.too_long").with("field", field)
}

fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, Error> {
    v.get(key).and_then(Value::as_str).ok_or_else(|| bad(key))
}

/// Reads and checks a `tutor-ask/1` text.
pub fn parse(input: &str) -> Result<Ask, Error> {
    let v: Value = serde_json::from_str(input).map_err(|_| bad("json"))?;
    let obj = v.as_object().ok_or_else(|| bad("json"))?;
    if let Some(k) = obj.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(bad(&format!("unknown:{k}")));
    }
    let course = text(&v, "course")?.trim().to_string();
    if course.is_empty() || course.chars().count() > 64 || course.chars().any(char::is_control) {
        return Err(bad("course"));
    }
    let client = text(&v, "client")?.to_string();
    if client.len() != 32
        || !client
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(bad("client"));
    }
    let lang = text(&v, "lang")?.to_string();
    if lang != "it" && lang != "en" {
        return Err(bad("lang"));
    }
    let question = text(&v, "question")?.trim().to_string();
    if question.is_empty() {
        return Err(bad("question"));
    }
    if question.chars().count() > MAX_QUESTION {
        return Err(too_long("question"));
    }
    let where_ = match v.get("where") {
        None | Some(Value::Null) => None,
        Some(Value::String(w))
            if !w.is_empty()
                && w.len() <= 80
                && w.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'#')) =>
        {
            Some(w.clone())
        }
        _ => return Err(bad("where")),
    };
    let mut history = Vec::new();
    match v.get("history") {
        None | Some(Value::Null) => {}
        Some(Value::Array(a)) => {
            if a.len() > MAX_TURNS {
                return Err(too_long("history"));
            }
            for t in a {
                history.push(Turn {
                    question: text(t, "q")?.to_string(),
                    answer: text(t, "a")?.to_string(),
                });
            }
        }
        _ => return Err(bad("history")),
    }
    let chars: usize = history
        .iter()
        .map(|t| t.question.chars().count() + t.answer.chars().count())
        .sum();
    if chars > MAX_HISTORY {
        return Err(too_long("history"));
    }
    let share = match v.get("share") {
        None => false,
        Some(Value::Bool(b)) => *b,
        _ => return Err(bad("share")),
    };
    Ok(Ask {
        course,
        client,
        lang,
        question,
        where_,
        history,
        share,
    })
}
