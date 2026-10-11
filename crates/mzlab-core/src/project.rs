//! Label, treatment time, type and concentration guessed from a file name: the same rules, in the same order, as `mzlab/project.py`
//! (`guess_sample`, `guess_conc`). No regex crate (size in the WASM): every pattern is matched by hand with the backtracking order of `re`.

const STD_WORDS: [&str; 10] = [
    "std",
    "stds",
    "standard",
    "stand",
    "cal",
    "calib",
    "calibr",
    "calibrazione",
    "taratura",
    "curva",
];
const EXTS: [&str; 11] = [
    ".mzml", ".mzxml", ".wiff", ".scan", ".raw", ".dam", ".d", ".cdf", ".xml", ".txt", ".csv",
];

/// Concentration units found in names -> `true` when known (the factor to mg/L lives in the page).
fn known_unit(u: &str) -> bool {
    matches!(
        u,
        "mgl" | "ppm" | "gl" | "ugl" | "ppb" | "ngml" | "ngl" | "ppt"
    )
}

/// File name without a REAL extension: 'std_0.5ppm' keeps its decimals.
fn stem(name: &str) -> String {
    let base = name.rsplit('/').next().unwrap_or(name);
    if let Some(i) = base.rfind('.') {
        if i > 0 && i + 1 < base.len() {
            let suf = base[i..].to_lowercase();
            if EXTS.contains(&suf.as_str()) {
                return base[..i].to_string();
            }
        }
    }
    base.to_string()
}

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
fn lookbehind_ok(s: &[char], i: usize) -> bool {
    i == 0 || !(s[i - 1].is_ascii_digit() || s[i - 1] == '.')
}
fn not_lower(s: &[char], i: usize) -> bool {
    i >= s.len() || !s[i].is_ascii_lowercase()
}

/// `\d+(?:[.,p]\d+)?`: end positions in the order the engine tries them (with the fraction first).
fn number_ends(s: &[char], i: usize, frac: &[char]) -> Vec<usize> {
    let d = digits(s, i);
    if d == i {
        return vec![];
    }
    let mut v = vec![];
    if d < s.len() && frac.contains(&s[d]) {
        let f = digits(s, d + 1);
        if f > d + 1 {
            v.push(f);
        }
    }
    v.push(d);
    v
}
fn num(s: &[char]) -> f64 {
    s.iter()
        .map(|c| if *c == 'p' || *c == ',' { '.' } else { *c })
        .collect::<String>()
        .parse()
        .unwrap_or(0.0)
}

/// `(ppm|ppb|ppt|[mun]?g[/_.\- ]?(?:m?l)(?:-1)?)` followed by `(?![a-z])`: the end of the first candidate that passes, with the text
/// as `re.sub(r"[/_.\- ]|-1$", "", ...)` leaves it (the `-` always wins over `-1$`, so a unit written "mg/l-1" ends in "1" and is unknown).
fn unit_at(s: &[char], i: usize) -> Option<(usize, String)> {
    let clean = |a: usize, b: usize| {
        s[a..b]
            .iter()
            .filter(|c| !matches!(**c, '/' | '_' | '.' | '-' | ' '))
            .collect::<String>()
    };
    for w in ["ppm", "ppb", "ppt"] {
        if starts(s, i, w) && not_lower(s, i + 3) {
            return Some((i + 3, w.to_string()));
        }
    }
    let mut j = i;
    if j < s.len() && matches!(s[j], 'm' | 'u' | 'n') {
        j += 1;
    }
    if !(j < s.len() && s[j] == 'g') {
        return None;
    }
    j += 1;
    if j < s.len() && matches!(s[j], '/' | '_' | '.' | '-' | ' ') {
        j += 1;
    }
    if j < s.len() && s[j] == 'm' {
        j += 1;
    }
    if !(j < s.len() && s[j] == 'l') {
        return None;
    }
    j += 1;
    if starts(s, j, "-1") && not_lower(s, j + 2) {
        return Some((j + 2, clean(i, j + 2)));
    }
    if not_lower(s, j) {
        return Some((j, clean(i, j)));
    }
    None
}

/// `(^|[_\-\s])(word)(?=[_\-\s\d]|$)`: position right after the word of the first match from `from`, or None.
fn std_word_at(s: &[char], p: usize) -> Option<usize> {
    for w in STD_WORDS {
        if starts(s, p, w) {
            let e = p + w.chars().count();
            if e >= s.len() || is_sep(s[e]) || s[e].is_ascii_digit() {
                return Some(e);
            }
        }
    }
    None
}
/// Word positions where `(^|[_\-\s])` can sit, in the order the engine tries them.
fn after_prefix(s: &[char]) -> Vec<usize> {
    let mut v = vec![];
    for i in 0..=s.len() {
        if i == 0 {
            v.push(0);
        }
        if i < s.len() && is_sep(s[i]) {
            v.push(i + 1);
        }
    }
    v
}
fn has_std(s: &[char]) -> bool {
    after_prefix(s)
        .into_iter()
        .any(|p| std_word_at(s, p).is_some())
}

/// Concentration read from a file name (`guess_conc`): value and unit key (None when only a number follows a standard word).
pub fn guess_conc(name: &str) -> Option<(f64, Option<String>)> {
    let low: Vec<char> = stem(name)
        .to_lowercase()
        .chars()
        .map(|c| {
            if c == '\u{b5}' || c == '\u{3bc}' {
                'u'
            } else {
                c
            }
        })
        .collect();
    let s = &low[..];
    // laboratory style: '7_2ppm' = 7.2 ppm (only when a unit follows at once)
    for i in (0..s.len()).filter(|&i| lookbehind_ok(s, i)) {
        let a = digits(s, i);
        if a == i || a >= s.len() || s[a] != '_' {
            continue;
        }
        let b = digits(s, a + 1);
        if b == a + 1 {
            continue;
        }
        if let Some((_, u)) = unit_at(s, skip_ws(s, b)) {
            if known_unit(&u) {
                let v: f64 = format!(
                    "{}.{}",
                    s[i..a].iter().collect::<String>(),
                    s[a + 1..b].iter().collect::<String>()
                )
                .parse()
                .unwrap_or(0.0);
                return Some((v, Some(u)));
            }
            break;
        }
    }
    for i in (0..s.len()).filter(|&i| lookbehind_ok(s, i)) {
        for e in number_ends(s, i, &['.', ',', 'p']) {
            let mut j = skip_ws(s, e);
            if j < s.len() && (s[j] == '_' || s[j] == '-') {
                j += 1;
            }
            j = skip_ws(s, j);
            if let Some((_, u)) = unit_at(s, j) {
                if known_unit(&u) {
                    return Some((num(&s[i..e]), Some(u)));
                }
                // the engine stops at the first match even if its unit is unknown
                return guess_conc_std(s);
            }
        }
    }
    guess_conc_std(s)
}

fn guess_conc_std(s: &[char]) -> Option<(f64, Option<String>)> {
    for p in after_prefix(s) {
        if let Some(e) = std_word_at(s, p) {
            let mut j = e;
            while j < s.len() && is_sep(s[j]) {
                j += 1;
            }
            if let Some(&n) = number_ends(s, j, &['.', ',', 'p']).first() {
                return Some((num(&s[j..n]), None));
            }
        }
    }
    None
}

/// (label, time in minutes, type) guessed from a file name: 'blank', 'std_5', 't30', '120min', '2h', '1d'.
pub fn guess_sample(name: &str) -> (String, Option<f64>, &'static str) {
    let base = stem(name);
    let low: Vec<char> = base.to_lowercase().chars().collect();
    let s = &low[..];
    for p in after_prefix(s) {
        for w in ["blank", "blk", "bianco"] {
            if starts(s, p, w) {
                let e = p + w.len();
                if e >= s.len() || is_sep(s[e]) || s[e].is_ascii_digit() {
                    return (base, None, "blank");
                }
            }
        }
    }
    let t_digit = after_prefix(s)
        .into_iter()
        .any(|p| p < s.len() && s[p] == 't' && p + 1 < s.len() && s[p + 1].is_ascii_digit());
    if has_std(s) || (guess_conc(name).is_some() && !t_digit) {
        return (base, None, "standard");
    }
    for p in after_prefix(s) {
        let mut i = p;
        if i < s.len() && s[i] == 't' {
            i += 1;
        }
        for e in number_ends(s, i, &['.', ',']) {
            if let Some(m) = time_tail(s, e) {
                let v: f64 = s[i..e]
                    .iter()
                    .map(|c| if *c == ',' { '.' } else { *c })
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0.0);
                let v = match m.as_str() {
                    "h" | "ore" => v * 60.0,
                    "d" | "day" | "days" | "gg" | "giorni" | "giorno" => v * 1440.0,
                    _ => v,
                };
                return (base, Some(v), "sample");
            }
        }
    }
    (base, None, "sample")
}

/// `\s*(min|m|h|ore|d|days?|gg|giorni|giorno)?(?:[_\-\s]|$)` after the number: the unit text ("" when none), or None when the tail does not fit.
fn time_tail(s: &[char], e: usize) -> Option<String> {
    let term = |j: usize| j >= s.len() || is_sep(s[j]);
    let k = skip_ws(s, e);
    for w in ["min", "m", "h", "ore", "d", "day", "gg", "giorni", "giorno"] {
        if starts(s, k, w) {
            let j = k + w.len();
            if w == "day" && j < s.len() && s[j] == 's' && term(j + 1) {
                return Some("days".into());
            }
            if term(j) {
                return Some(w.to_string());
            }
        }
    }
    if k > e || term(e) {
        return Some(String::new());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(guess_sample("blank_1.mzML").2, "blank");
        assert_eq!(guess_sample("t30.mzML").1, Some(30.0));
        assert_eq!(guess_sample("2h.mzML").1, Some(120.0));
        assert_eq!(
            guess_conc("std_0.5ppm.mzML"),
            Some((0.5, Some("ppm".into())))
        );
        assert_eq!(guess_conc("STD7_2ppm"), Some((7.2, Some("ppm".into()))));
        assert_eq!(guess_conc("std_5"), Some((5.0, None)));
    }
}
