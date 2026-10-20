//! Libro Markdown dialect -> Teoria HTML converter: a port of tools/libro (the Python reference) with byte-for-byte parity on every chapter.
//! Used by the parity test today and by the Teoria editor preview; pure and deterministic (no clock, no I/O).
mod blocks;
mod inline;
mod page;

pub use page::{convert, Options};

/// The one error type: always carries file and line, so an unsupported construct stops the build instead of being guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibroError {
    pub file: String,
    pub line: usize,
    pub msg: String,
}

impl LibroError {
    pub fn new(file: &str, line: usize, msg: impl Into<String>) -> Self {
        LibroError {
            file: file.to_string(),
            line,
            msg: msg.into(),
        }
    }
}

impl std::fmt::Display for LibroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.msg)
    }
}

impl std::error::Error for LibroError {}

/// Python `html.escape`.
pub(crate) fn escape(text: &str, quote: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if quote => out.push_str("&quot;"),
            '\'' if quote => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Python `str.splitlines()`.
pub(crate) fn splitlines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut it = text.char_indices().peekable();
    while let Some((i, ch)) = it.next() {
        let brk = matches!(
            ch,
            '\n' | '\r'
                | '\x0b'
                | '\x0c'
                | '\x1c'
                | '\x1d'
                | '\x1e'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if !brk {
            continue;
        }
        out.push(&text[start..i]);
        start = i + ch.len_utf8();
        if ch == '\r' && it.peek().is_some_and(|&(_, n)| n == '\n') {
            it.next();
            start += 1;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// `[\w]` of the Python reference.
pub(crate) fn is_w(c: char) -> bool {
    inline::is_w(c)
}
