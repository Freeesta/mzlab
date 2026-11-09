//! The byte scanner behind `csv_leggi`: splits a CSV text into records of cells (a slice of the text unless the cell holds a `""` or text after its closing quote).
//! Same rules as the first char-by-char parser (kept as the reference in `csv_prove`).

use std::borrow::Cow;

use super::csv::bad;
use crate::error::Result;

pub(super) type Cells<'a> = Vec<(Cow<'a, str>, bool)>;

/// The field being read: a run of `text` still borrowed (`start..`), the text that came before it when the field stopped being one slice (`""`, text after a closing quote), and the flags.
struct Field {
    owned: String,
    has_owned: bool,
    start: usize,
    /// End of the run when a closing quote ended it (the quote itself is not content).
    stop: Option<usize>,
    quoted: bool,
}

impl Field {
    /// Whether what the field holds so far, with the run ending at `end`, is blank (a quote opens a quoted field only then).
    fn blank(&self, text: &str, end: usize) -> bool {
        self.owned.trim().is_empty() && text[self.start..end].trim().is_empty()
    }

    /// Whether the field holds nothing at all (before any trimming), with the run ending at `end`.
    fn empty(&self, end: usize) -> bool {
        self.owned.is_empty() && self.start >= end
    }

    /// The finished field, its run ending at `end` (the closing quote's position when there was one), and the start of the next.
    fn take<'a>(&mut self, text: &'a str, end: usize, next: usize) -> (Cow<'a, str>, bool) {
        let end = self.stop.unwrap_or(end);
        let run = &text[self.start..end];
        let v = if self.has_owned {
            self.owned.push_str(run);
            Cow::Owned(std::mem::take(&mut self.owned))
        } else if self.quoted {
            Cow::Borrowed(run)
        } else {
            Cow::Borrowed(run.trim())
        };
        let quoted = std::mem::take(&mut self.quoted);
        self.has_owned = false;
        self.stop = None;
        self.start = next;
        (v, quoted)
    }
}

/// The records of `text`: quoted fields keep their separators and newlines (`""` is a quote); a field is trimmed unless quoted. A field is a slice of `text` unless it holds a `""` or text after
/// its closing quote. Scans bytes: `"`, `\n`, `\r` and the separator are the only bytes that matter and none of them occurs inside a multi-byte character.
pub(super) fn records(text: &str, sep: char) -> Result<Vec<Cells<'_>>> {
    let text = text.trim_start_matches('\u{feff}');
    let bytes = text.as_bytes();
    let mut sb = [0u8; 4];
    let sepb = sep.encode_utf8(&mut sb).as_bytes();
    let special = |b: u8| matches!(b, b'"' | b'\n' | b'\r') || b == sepb[0];
    let mut out: Vec<Cells> = Vec::new();
    let (mut row, mut width): (Cells, usize) = (Vec::new(), 0);
    let mut f = Field {
        owned: String::new(),
        has_owned: false,
        start: 0,
        stop: None,
        quoted: false,
    };
    let (mut at, mut inq) = (0, false);
    while let Some(d) = bytes[at..].iter().position(|&b| special(b)) {
        let j = at + d;
        let b = bytes[j];
        at = j + 1;
        if inq {
            if b != b'"' {
                continue;
            }
            if bytes.get(j + 1) == Some(&b'"') {
                // `""`: the first quote is content, the second is skipped
                f.owned.push_str(&text[f.start..=j]);
                (f.has_owned, f.start, at) = (true, j + 2, j + 2);
                continue;
            }
            inq = false;
            let ends = match bytes.get(j + 1) {
                None => true,
                Some(&n) => matches!(n, b'\n' | b'\r') || bytes[j + 1..].starts_with(sepb),
            };
            if ends {
                f.stop = Some(j);
            } else {
                f.owned.push_str(&text[f.start..j]);
                (f.has_owned, f.start) = (true, j + 1);
            }
        } else if b == b'"' && f.blank(text, j) {
            f.owned.clear();
            (f.has_owned, f.start, f.quoted, inq) = (false, j + 1, true, true);
        } else if b == sepb[0] && bytes[j..].starts_with(sepb) {
            at = j + sepb.len();
            row.push(f.take(text, j, at));
        } else if b == b'\n' || b == b'\r' {
            if b == b'\r' && bytes.get(j + 1) == Some(&b'\n') {
                at += 1;
            }
            row.push(f.take(text, j, at));
            let blank = row.len() == 1 && row[0].0.is_empty() && !row[0].1;
            if blank {
                row.clear();
            } else {
                width = width.max(row.len());
                out.push(std::mem::replace(&mut row, Vec::with_capacity(width)));
            }
        }
    }
    if inq {
        return Err(bad(out.len() + 1, ""));
    }
    let end = f.stop.unwrap_or(bytes.len());
    if !f.empty(end) || !row.is_empty() {
        row.push(f.take(text, bytes.len(), bytes.len()));
        out.push(row);
    }
    Ok(out)
}
