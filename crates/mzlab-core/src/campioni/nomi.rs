//! The one parser of the file names: role, time as written, replicate, series and concentration proposed from a name (`tempo_confermato` false).
//! The role and the minutes follow `project::guess_sample` (the same names give the same minutes), plus seconds (`90 s`), the replicate `(2)`,
//! the infusion (`infusion`, `infusione`, `direct-infusion`, `FIA` or an MSn file), the dark control (`dark`, `buio`, `adsorption`) and the series.

use super::{number_replicates, Campione};
use crate::project::{guess_conc, guess_sample};

const CAL_WORDS: [&str; 6] = [
    "cal",
    "calib",
    "calibr",
    "calibrazione",
    "taratura",
    "curva",
];
const DARK_WORDS: [&str; 3] = ["dark", "buio", "adsorption"];
const INFUSION_WORDS: [&str; 3] = ["infusion", "infusione", "fia"];
/// Units after the number of a time: the text in the name and the unit it stands for (`s`, `min`, `h`, `d`).
const TIME_WORDS: [(&str, &str); 11] = [
    ("min", "min"),
    ("m", "min"),
    ("h", "h"),
    ("ore", "h"),
    ("d", "d"),
    ("day", "d"),
    ("gg", "d"),
    ("giorni", "d"),
    ("giorno", "d"),
    ("sec", "s"),
    ("s", "s"),
];

fn is_sep(c: char) -> bool {
    c == '_' || c == '-' || c.is_whitespace()
}
fn digits(s: &[char], mut i: usize) -> usize {
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    i
}
fn skip_ws(s: &[char], mut i: usize) -> usize {
    while i < s.len() && s[i].is_whitespace() {
        i += 1;
    }
    i
}
fn starts(s: &[char], i: usize, w: &str) -> bool {
    let w: Vec<char> = w.chars().collect();
    s.len() >= i + w.len() && s[i..i + w.len()] == w[..]
}

/// `\d+(?:[.,]\d+)?`: the end positions, the one with the fraction first.
fn number_ends(s: &[char], i: usize) -> Vec<usize> {
    let d = digits(s, i);
    if d == i {
        return vec![];
    }
    let mut v = vec![];
    if d < s.len() && (s[d] == '.' || s[d] == ',') && digits(s, d + 1) > d + 1 {
        v.push(digits(s, d + 1));
    }
    v.push(d);
    v
}

/// The start of the name and every position after a separator.
fn after_prefix(s: &[char]) -> Vec<usize> {
    let mut v = vec![0];
    v.extend((0..s.len()).filter(|&i| is_sep(s[i])).map(|i| i + 1));
    v
}

/// The unit of the time that ends at `e` (`min` when none is written), or None when what follows is not a time.
fn time_tail(s: &[char], e: usize) -> Option<&'static str> {
    let term = |j: usize| j >= s.len() || is_sep(s[j]);
    let k = skip_ws(s, e);
    for (w, unit) in TIME_WORDS {
        if starts(s, k, w) {
            let j = k + w.len();
            if term(j) || (w == "day" && j < s.len() && s[j] == 's' && term(j + 1)) {
                return Some(unit);
            }
        }
    }
    (k > e || term(e)).then_some("min")
}

/// A time read from a name: the value as written, its unit and where the match starts.
struct Time {
    value: f64,
    unit: &'static str,
    start: usize,
}

/// `t?\d+([.,]\d+)?\s*unit?` after a separator, in the order of `guess_sample`.
fn time_of(s: &[char]) -> Option<Time> {
    for p in after_prefix(s) {
        let i = if p < s.len() && s[p] == 't' { p + 1 } else { p };
        for e in number_ends(s, i) {
            if let Some(unit) = time_tail(s, e) {
                let text: String = s[i..e]
                    .iter()
                    .map(|c| if *c == ',' { '.' } else { *c })
                    .collect();
                return Some(Time {
                    value: text.parse().unwrap_or(0.0),
                    unit,
                    start: p,
                });
            }
        }
    }
    None
}

/// A word of the name with its folders (letters and digits) equal to `word`, or `word` followed by digits only.
fn has_word(low: &str, word: &str) -> bool {
    low.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| {
        w == word || (w.starts_with(word) && w[word.len()..].chars().all(|c| c.is_ascii_digit()))
    })
}

/// The role without the infusion, as `campioni/1` gives it: blank, standard or calibration, dark, QC, time zero, sample.
fn base_kind(low: &str, guess: &str, time_min: Option<f64>) -> &'static str {
    match guess {
        "blank" => "blank",
        "standard" => CAL_WORDS
            .iter()
            .find(|w| has_word(low, w))
            .map_or("std", |_| "calibration"),
        _ if DARK_WORDS.iter().any(|w| has_word(low, w)) => "dark",
        _ if has_word(low, "qc") => "qc",
        _ if time_min == Some(0.0) => "t0",
        _ => "sample",
    }
}

/// The role of `campioni/1` for a name (no infusion there): what `samples.rs` writes.
pub fn tipo_v1(name: &str) -> &'static str {
    let (_, time, guess) = guess_sample(name);
    base_kind(&name.to_lowercase(), guess, time)
}

fn kind_of(low: &str, guess: &str, time_min: Option<f64>, msn: bool) -> &'static str {
    if guess != "blank" && (msn || INFUSION_WORDS.iter().any(|w| has_word(low, w))) {
        return "infusion";
    }
    base_kind(low, guess, time_min)
}

fn blank_kind(low: &str) -> Option<String> {
    let word = |pre: &str| {
        low.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w.starts_with(pre))
    };
    if word("solv") {
        Some("solvent".into())
    } else if word("proc") {
        Some("procedure".into())
    } else {
        None
    }
}

/// `(2)` at the end of the name: the replicate written by the user.
fn written_replicate(stem: &str) -> Option<u32> {
    let s = stem.trim_end().strip_suffix(')')?;
    let (_, n) = s.rsplit_once('(')?;
    (!n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        .then(|| n.parse().ok())
        .flatten()
        .filter(|&n| n >= 1)
}

/// The text before the time (`X_TiO2_t005min` → `X_TiO2`), in the case of the name.
fn prefix_of(stem: &str, low: &[char], start: usize) -> String {
    let orig: Vec<char> = stem.chars().collect();
    let s = if orig.len() == low.len() {
        &orig[..start]
    } else {
        &low[..start]
    };
    let mut p: String = s.iter().collect();
    while p.ends_with(is_sep) {
        p.pop();
    }
    p
}

/// The series: the prefix before the time when the timed files have more than one (an empty prefix belongs to every series); a file without
/// a time takes the longest series its name starts with.
fn assign_series(out: &mut [Campione], prefixes: &[Option<String>]) {
    let mut distinct: Vec<&String> = prefixes
        .iter()
        .flatten()
        .filter(|p| !p.is_empty())
        .collect();
    distinct.sort();
    distinct.dedup();
    if distinct.len() <= 1 {
        return;
    }
    for (c, p) in out.iter_mut().zip(prefixes) {
        c.series = match p {
            Some(p) => p.clone(),
            None => {
                let low = guess_sample(&c.file).0.to_lowercase();
                let fits = |s: &&&String| {
                    let s = s.to_lowercase();
                    low.starts_with(&s) && low[s.len()..].chars().next().is_none_or(is_sep)
                };
                distinct
                    .iter()
                    .filter(fits)
                    .max_by_key(|s| s.len())
                    .map_or(String::new(), |s| s.to_string())
            }
        };
    }
}

/// The metadata proposed for `files` (`(name, SHA-256, kind of the file)`, in the order of injection; kind `msn` = an infusion); an infusion and a QC have no time.
pub fn proponi(files: &[(String, String, String)]) -> Vec<Campione> {
    let mut out = Vec::with_capacity(files.len());
    let mut prefixes = Vec::with_capacity(files.len());
    let mut written = Vec::with_capacity(files.len());
    for (i, (name, sha, file_kind)) in files.iter().enumerate() {
        let (stem, _, guess) = guess_sample(name);
        let lowc: Vec<char> = stem.to_lowercase().chars().collect();
        let low = name.to_lowercase();
        let mut time = if guess == "sample" {
            time_of(&lowc)
        } else {
            None
        };
        let mut c = Campione::new(name, sha, i as u32 + 1);
        if let Some(t) = &time {
            c.set_time(Some(t.value), t.unit);
        }
        c.kind = kind_of(&low, guess, c.time_min, file_kind == "msn").to_string();
        if c.kind == "infusion" || c.kind == "qc" {
            c.set_time(None, "min");
            time = None;
        }
        if c.kind == "blank" {
            c.blank_kind = blank_kind(&low);
        }
        if let (Some((v, u)), "standard") = (guess_conc(name), guess) {
            c.conc = Some(v);
            c.unit = u;
        }
        prefixes.push(time.map(|t| prefix_of(&stem, &lowc, t.start)));
        written.push(written_replicate(&stem));
        out.push(c);
    }
    assign_series(&mut out, &prefixes);
    number_replicates(&mut out);
    for (c, w) in out.iter_mut().zip(written) {
        if let Some(n) = w {
            c.replicate = n;
        }
    }
    out
}
