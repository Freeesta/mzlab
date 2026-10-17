//! The files of mzFinder as `api.classify` names them (`guess_sample`, `file_kind`, `classify`), read from the
//! `.mzidx` of each file: label, treatment time and type guessed from the name, kind of the unit-resolution path and kind of the high-resolution path.

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;

/// What the name of a file says: label (the stem), time in minutes, type (`sample`, `blank`, `standard`, `control`).
#[derive(Debug, Clone, PartialEq)]
pub struct Guess {
    pub label: String,
    pub time: Option<f64>,
    pub kind: &'static str,
}

/// `Path(name).stem`: the last component without its last suffix.
pub fn stem(name: &str) -> String {
    let base = name.rsplit('/').next().unwrap_or(name);
    match base.rfind('.') {
        Some(i) if i > 0 && i + 1 < base.len() => base[..i].to_string(),
        _ => base.to_string(),
    }
}

fn sep(c: char) -> bool {
    c == '_' || c == '-' || c.is_whitespace()
}

/// Positions where a match may start: the start of the text and after every separator (`(^|[_\-\s])`).
fn starts(s: &[char]) -> Vec<usize> {
    let mut v = vec![0];
    v.extend((0..s.len()).filter(|&i| sep(s[i])).map(|i| i + 1));
    v
}

fn word_at(s: &[char], p: usize, w: &str) -> bool {
    let w: Vec<char> = w.chars().collect();
    p + w.len() <= s.len() && s[p..p + w.len()] == w[..]
}

/// `(^|[_\-\s])(w1|w2|...)([_\-\s\d]|$)`
fn has_word(s: &[char], words: &[&str]) -> bool {
    starts(s).into_iter().any(|p| {
        words.iter().any(|w| {
            let e = p + w.chars().count();
            word_at(s, p, w) && (e >= s.len() || sep(s[e]) || s[e].is_ascii_digit())
        })
    })
}

/// The end of `\d+(?:[.,]\d+)?` at `i` (greedy; a shorter number is followed by a digit or a decimal mark, which no tail accepts).
fn number_end(s: &[char], i: usize) -> Option<usize> {
    let d = |mut j: usize| {
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
        }
        j
    };
    let e = d(i);
    if e == i {
        return None;
    }
    if e < s.len() && (s[e] == '.' || s[e] == ',') && d(e + 1) > e + 1 {
        return Some(d(e + 1));
    }
    Some(e)
}

/// `\s*(units)?(?:[_\-\s(]|$)` after the number, with the backtracking of the regex: the most spaces first, then each unit in order, then no unit.
/// The unit matched, `""` for none, or None when the tail does not fit.
fn tail(s: &[char], e: usize, units: &[&'static str], unit_needed: bool) -> Option<&'static str> {
    let term = |j: usize| j >= s.len() || sep(s[j]) || s[j] == '(';
    let mut ws = e;
    while ws < s.len() && s[ws].is_whitespace() {
        ws += 1;
    }
    for k in (e..=ws).rev() {
        for u in units {
            if word_at(s, k, u) && term(k + u.chars().count()) {
                return Some(u);
            }
        }
        if !unit_needed && term(k) {
            return Some("");
        }
    }
    None
}

fn time_value(s: &[char], a: usize, e: usize, unit: &str) -> f64 {
    let t: String = s[a..e]
        .iter()
        .map(|&c| if c == ',' { '.' } else { c })
        .collect();
    let v: f64 = t.parse().unwrap_or(0.0);
    if unit == "h" || unit == "ore" {
        v * 60.0
    } else {
        v
    }
}

/// `t(\d+(?:[.,]\d+)?)\s*(min|m|h|ore)?` first, then `(\d+(?:[.,]\d+)?)\s*(min|h|ore)`, each from the leftmost start.
fn time_of(s: &[char]) -> Option<f64> {
    for p in starts(s) {
        if p < s.len() && s[p] == 't' {
            if let Some(e) = number_end(s, p + 1) {
                if let Some(u) = tail(s, e, &["min", "m", "h", "ore"], false) {
                    return Some(time_value(s, p + 1, e, u));
                }
            }
        }
    }
    for p in starts(s) {
        if let Some(e) = number_end(s, p) {
            if let Some(u) = tail(s, e, &["min", "h", "ore"], true) {
                return Some(time_value(s, p, e, u));
            }
        }
    }
    None
}

/// `guess_sample`: label, time (min) and type from a file name ('blank', 't30', 'STD_12ppm', '120min', 'dark').
pub fn guess(name: &str) -> Guess {
    let label = stem(name);
    let s: Vec<char> = label.to_lowercase().chars().collect();
    let kind = if has_word(&s, &["blank", "blk", "bianco"]) {
        "blank"
    } else if has_word(&s, &["std", "standard", "taratura", "cal"]) {
        "standard"
    } else if has_word(&s, &["control", "ctrl", "dark", "buio"]) {
        "control"
    } else {
        "sample"
    };
    let time = if kind == "sample" { time_of(&s) } else { None };
    Guess { label, time, kind }
}

/// `file_kind` (unit-resolution path) and `hr_engine.classify` of an index: (`full`|`ms2`|`mrm`|`empty`, `hr`|`msn`|`other`, number of scans).
/// The `.mzidx` keeps the number of chromatograms but not their ids: a file with no spectra to speak of and more than one chromatogram (the TIC and
/// at least one transition) is `mrm`.
pub fn kinds<R: ReadAt>(ix: &Index<R>) -> (&'static str, &'static str, usize) {
    let n1 = ix.scans.iter().filter(|s| s.level == 1).count();
    let n2 = ix.scans.iter().filter(|s| s.level == 2).count();
    let chroms = ix
        .run
        .get("n_chromatograms")
        .and_then(|v| v.f64())
        .unwrap_or(0.0);
    let kind = if n1 >= 20 && n1 >= n2 {
        "full"
    } else if n2 >= 5 {
        "ms2"
    } else if chroms > 1.0 {
        "mrm"
    } else {
        "empty"
    };
    let max_level = ix.scans.iter().map(|s| s.level).max().unwrap_or(1);
    let hr = if max_level >= 3 {
        "msn"
    } else if ix.hr1() && n1 > 0 {
        "hr"
    } else {
        "other"
    };
    (kind, hr, ix.scans.len())
}

/// One item of `api.classify`: `{name, kind, hr_kind, type, time, label, scans}`, or the error item when the index cannot be read.
pub fn item_json<R: ReadAt>(o: &mut Out, name: &str, ix: std::result::Result<&Index<R>, String>) {
    let g = guess(name);
    o.raw("{\"name\":").str(name);
    match ix {
        Ok(ix) => {
            let (kind, hr, n) = kinds(ix);
            o.raw(",\"kind\":").str(kind).raw(",\"hr_kind\":").str(hr);
            o.raw(",\"type\":")
                .str(g.kind)
                .raw(",\"time\":")
                .opt(g.time);
            o.raw(",\"label\":")
                .str(&g.label)
                .raw(",\"scans\":")
                .int(n as i64);
        }
        Err(e) => {
            let e: String = e.chars().take(160).collect();
            o.raw(",\"kind\":\"error\",\"error\":").str(&e);
            o.raw(",\"type\":\"sample\",\"time\":null,\"label\":")
                .str(name);
        }
    }
    o.raw("}");
}
