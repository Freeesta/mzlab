//! The one parser of the file names: role, time as written, replicate, series and concentration proposed from a name (`tempo_confermato` false).
//! The role and the minutes follow `project::guess_sample` (the same names give the same minutes), plus seconds (`90 s`), the replicate `(2)`,
//! the infusion (`infusion`, `infusione`, `direct-infusion`, `FIA` or an MSn file), the dark control (`dark`, `buio`, `adsorption`) and the series.

use super::blank_words::is_blank_name;
use super::positions::{self, Proposal};
use super::{minutes_per, number_replicates, Sample};
use crate::project::{guess_conc, guess_sample as guess_stem, guess_type};

/// `project::guess_sample` plus the blank words of `data/nomi-bianco.json` (`MQ_Wash_1`), which the parity with `project.py` does not carry.
/// The stem and the type of a name (`blank`, `standard`, `sample`) without reading its time, with the blank words of `nomi-bianco.json`.
fn guess_kind(name: &str) -> (String, &'static str) {
    let (stem, guess) = guess_type(name);
    if guess == "sample" && is_blank_name(&stem.to_lowercase()) {
        return (stem, "blank");
    }
    (stem, guess)
}

fn guess_sample(name: &str) -> (String, Option<f64>, &'static str) {
    let (stem, time, guess) = guess_stem(name);
    if guess == "sample" && is_blank_name(&stem.to_lowercase()) {
        return (stem, None, "blank");
    }
    (stem, time, guess)
}

const CAL_WORDS: [&str; 6] = [
    "cal",
    "calib",
    "calibr",
    "calibrazione",
    "taratura",
    "curva",
];
const DARK_WORDS: [&str; 5] = ["dark", "buio", "adsorption", "control", "ctrl"];
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
    // `w` is ASCII: compare in place, no allocation
    let w = w.as_bytes();
    s.len() >= i + w.len()
        && s[i..i + w.len()]
            .iter()
            .zip(w)
            .all(|(c, b)| *c == char::from(*b))
}

/// `\d+(?:[.,]\d+)?` (also `\d+p\d+` after a `t`, as `T2p5`): the end positions, the one with the fraction first.
fn number_ends(s: &[char], i: usize, p_point: bool) -> impl Iterator<Item = usize> + use<> {
    let d = digits(s, i);
    let with_fraction = (d < s.len() && (s[d] == '.' || s[d] == ',' || (p_point && s[d] == 'p')))
        .then(|| digits(s, d + 1))
        .filter(|f| *f > d + 1);
    [with_fraction, (d > i).then_some(d)].into_iter().flatten()
}

/// The start of the name and every position after a separator.
fn after_prefix(s: &[char]) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(0).chain((0..s.len()).filter(|&i| is_sep(s[i])).map(|i| i + 1))
}

/// The unit of the time that ends at `e` (`min` when none is written), where the time ends and whether a unit is written, or None when what follows is not a time.
fn time_tail(s: &[char], e: usize, after_t: bool) -> Option<(&'static str, usize, bool)> {
    // `(` ends a time too: `t30min(2)` is the second replicate of 30 min
    let term = |j: usize| j >= s.len() || is_sep(s[j]) || s[j] == '(';
    let k = skip_ws(s, e);
    for (w, unit) in TIME_WORDS {
        if w == "m" && !after_t {
            continue;
        }
        if starts(s, k, w) {
            let j = k + w.len();
            if term(j) {
                return Some((unit, j, true));
            }
            if w == "day" && j < s.len() && s[j] == 's' && term(j + 1) {
                return Some((unit, j + 1, true));
            }
        }
    }
    term(e).then_some(("min", e, false))
}

/// A time read from a name: the value as written, its unit and where the match starts and ends.
struct Time {
    value: f64,
    unit: &'static str,
    start: usize,
    end: usize,
}

/// `t?\d+([.,]\d+)?\s*unit` after a separator: the LAST one with a written unit or a `t` prefix (`Smp_01_30h_2` is 30 h, its `01` and `_2`
/// are an index and a replicate); a bare number (`Run_5`) is never a time. A bare `tN` is minutes.
fn time_of(s: &[char]) -> Option<Time> {
    let mut last = None;
    for p in after_prefix(s) {
        let after_t = p < s.len() && s[p] == 't';
        let i = if after_t { p + 1 } else { p };
        for e in number_ends(s, i, after_t) {
            if let Some((unit, end, written)) = time_tail(s, e, after_t) {
                if !written && !after_t {
                    break;
                }
                let text: String = s[i..e]
                    .iter()
                    .map(|c| if *c == ',' || *c == 'p' { '.' } else { *c })
                    .collect();
                last = Some(Time {
                    value: text.parse().unwrap_or(0.0),
                    unit,
                    start: p,
                    end,
                });
                break;
            }
        }
    }
    last
}

/// A word of the name with its folders (letters and digits) equal to one of `words`, or one followed by digits only (one pass over the name).
fn has_any(low: &str, words: &[&str]) -> bool {
    low.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| {
        words.iter().any(|word| {
            w == *word
                || (w.starts_with(word) && w[word.len()..].chars().all(|c| c.is_ascii_digit()))
        })
    })
}

/// The role without the infusion, as `campioni/1` gives it: blank, standard or calibration, dark, QC, time zero, sample.
fn base_kind(low: &str, guess: &str, time_min: Option<f64>) -> &'static str {
    match guess {
        "blank" => "blank",
        "standard" if has_any(low, &CAL_WORDS) => "calibration",
        "standard" => "std",
        _ if has_any(low, &DARK_WORDS) => "dark",
        _ if has_any(low, &["qc"]) => "qc",
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
    if guess != "blank" && (msn || has_any(low, &INFUSION_WORDS)) {
        return "infusion";
    }
    base_kind(low, guess, time_min)
}

fn blank_kind(low: &str) -> Option<String> {
    let word = |pre: &str| {
        low.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w.starts_with(pre))
    };
    if word("wash") {
        Some("wash".into())
    } else if word("solv") {
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
fn assign_series(out: &mut [Sample], prefixes: &[Option<String>]) {
    let mut distinct: Vec<&String> = prefixes
        .iter()
        .flatten()
        .filter(|p| !p.is_empty())
        .collect();
    super::keep_first(&mut distinct);
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

/// The first word of a stem when it can be a matrix: 2-4 letters followed by more name (`TW` in `TW_PMS_T05`).
fn matrix_word(stem: &str) -> Option<&str> {
    let (w, rest) = stem.split_once(is_sep)?;
    let ok = (2..=4).contains(&w.len()) && w.chars().all(|c| c.is_ascii_alphabetic());
    (ok && !rest.is_empty()).then_some(w)
}

/// The matrix and the group proposed from the shape `MAT_COND_Tnn`: the first word of the names when at least two different words each start
/// two or more files (`TW`, `MQ`), whatever they are. The group of a timed file is `MAT-COND` (its series without the matrix). Nothing is
/// proposed with a single word: one matrix says nothing the series does not.
fn assign_matrix(out: &mut [Sample]) {
    let words: Vec<Option<String>> = out
        .iter()
        .map(|c| matrix_word(&guess_sample(&c.file).0).map(String::from))
        .collect();
    let mut counts: Vec<(&String, usize)> = Vec::new();
    for w in words.iter().flatten() {
        match counts.iter_mut().find(|(k, _)| *k == w) {
            Some(e) => e.1 += 1,
            None => counts.push((w, 1)),
        }
    }
    counts.retain(|e| e.1 >= 2);
    if counts.len() < 2 {
        return;
    }
    for (c, w) in out.iter_mut().zip(&words) {
        let Some(w) = w.as_ref().filter(|w| counts.iter().any(|e| e.0 == *w)) else {
            continue;
        };
        c.matrix = w.clone();
        let cond = c
            .series
            .get(w.len()..)
            .map(|r| r.trim_start_matches(is_sep));
        if let (true, Some(cond)) = (
            c.series.starts_with(w.as_str()),
            cond.filter(|r| !r.is_empty()),
        ) {
            c.group = format!("{w}-{cond}");
        }
    }
}

/// The role (`proponi` names) and the time of one name as `(minutes, start, end)`, the span in characters of the file stem: what `variabili` needs.
pub(crate) fn role_and_time(name: &str) -> (&'static str, Option<(f64, usize, usize)>) {
    let (stem, guess) = guess_kind(name);
    let lowc: Vec<char> = stem.to_lowercase().chars().collect();
    let time = (guess == "sample").then(|| time_of(&lowc)).flatten();
    let minutes = time.as_ref().map(|t| t.value * minutes_per(t.unit));
    let kind = kind_of(&name.to_lowercase(), guess, minutes, false);
    let keep = kind != "infusion" && kind != "qc";
    let span = time.filter(|_| keep).zip(minutes);
    (kind, span.map(|(t, m)| (m, t.start, t.end)))
}

/// The role (`proponi` names) and the minutes of one name, without the series, replicates and positions: what a caller that only classifies a name needs.
pub fn role_and_minutes(name: &str) -> (&'static str, Option<f64>) {
    let (kind, time) = role_and_time(name);
    (kind, time.map(|t| t.0))
}

/// The metadata proposed for `files` (`(name, SHA-256, kind of the file)`, in the order of injection; kind `msn` = an infusion); an infusion and a QC have no time.
pub fn proponi(files: &[(String, String, String)]) -> Vec<Sample> {
    propose_explained(files)
        .into_iter()
        .map(|p| p.sample)
        .collect()
}

/// The same as `proponi`, each file with the parts of its name that decided its fields (`positions.rs`); the values are the ones of `proponi`.
pub fn propose_explained(files: &[(String, String, String)]) -> Vec<Proposal> {
    let mut out = Vec::with_capacity(files.len());
    let mut prefixes = Vec::with_capacity(files.len());
    let mut written = Vec::with_capacity(files.len());
    let mut spans = Vec::with_capacity(files.len());
    for (i, (name, sha, file_kind)) in files.iter().enumerate() {
        let (stem, _, guess) = guess_sample(name);
        let lowc: Vec<char> = stem.to_lowercase().chars().collect();
        let low = name.to_lowercase();
        let mut time = if guess == "sample" {
            time_of(&lowc)
        } else {
            None
        };
        let mut c = Sample::new(name, sha, i as u32 + 1);
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
        spans.push(positions::of_file(
            name,
            &c,
            &stem,
            time.as_ref().map(|t| (t.start, t.end)),
        ));
        prefixes.push(time.map(|t| prefix_of(&stem, &lowc, t.start)));
        written.push(written_replicate(&stem));
        out.push(c);
    }
    assign_series(&mut out, &prefixes);
    assign_matrix(&mut out);
    number_replicates(&mut out);
    for (c, w) in out.iter_mut().zip(&written) {
        if let Some(n) = w {
            c.replicate = *n;
        }
    }
    out.into_iter()
        .zip(spans)
        .zip(written)
        .map(|((c, sp), w)| {
            let positions = sp.finish(&c, w.is_some());
            Proposal {
                sample: c,
                positions,
            }
        })
        .collect()
}
