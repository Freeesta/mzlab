//! The time point of a sample name (`t0`, `3h`, `1 h 30`, `d1`): the whole-token rules of the parser in `names.rs`, which lists the spellings.
//! Split out so each file stays small; the minutes, the written unit and the span in the name are what `names.rs` and `variables.rs` use.

/// Units after the number of a time: the text in the name and the unit it stands for (`s`, `min`, `h`, `d`).
const TIME_WORDS: [(&str, &str); 22] = [
    ("min", "min"),
    ("mins", "min"),
    ("minute", "min"),
    ("minutes", "min"),
    ("m", "min"),
    ("h", "h"),
    ("hr", "h"),
    ("hrs", "h"),
    ("hour", "h"),
    ("hours", "h"),
    ("ore", "h"),
    ("d", "d"),
    ("day", "d"),
    ("days", "d"),
    ("gg", "d"),
    ("giorni", "d"),
    ("giorno", "d"),
    ("sec", "s"),
    ("secs", "s"),
    ("second", "s"),
    ("seconds", "s"),
    ("s", "s"),
];

pub(super) fn is_sep(c: char) -> bool {
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

/// The start of the name and every position after a separator or an opening parenthesis (`RET (3h)`).
fn after_prefix(s: &[char]) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(0).chain(
        (0..s.len())
            .filter(|&i| is_sep(s[i]) || s[i] == '(')
            .map(|i| i + 1),
    )
}

/// What follows the number of a time: its unit, where the time ends, whether the unit is written and the minutes written after hours (`1 h 30`).
struct Tail {
    unit: &'static str,
    end: usize,
    written: bool,
    plus_min: f64,
}

/// Minutes written after hours: `1 h 30`, `1h30` (two digits, under 60, nothing else after) or `1h 5min` (a unit); the minutes and where they end.
fn minutes_after_hours(s: &[char], j: usize, term: &dyn Fn(usize) -> bool) -> Option<(f64, usize)> {
    // only spaces between the hours and the minutes: `3h_15` is a replicate, not 3 h 15
    let m0 = skip_ws(s, j);
    let m1 = digits(s, m0);
    let n = m1 - m0;
    if n == 0 || n > 2 {
        return None;
    }
    let v: f64 = s[m0..m1].iter().collect::<String>().parse().ok()?;
    if v >= 60.0 {
        return None;
    }
    let k = skip_ws(s, m1);
    for w in ["min", "mins", "minute", "minutes", "m"] {
        if starts(s, k, w) && term(k + w.len()) {
            return Some((v, k + w.len()));
        }
    }
    (n == 2 && term(m1)).then_some((v, m1))
}

/// The unit of the time that ends at `e` (`min` when none is written), where the time ends and whether a unit is written, or None when what follows is not a time.
fn time_tail(s: &[char], e: usize, after_t: bool) -> Option<Tail> {
    // `(` ends a time too: `t30min(2)` is the second replicate of 30 min; `)` closes `(3h)`
    let term = |j: usize| j >= s.len() || is_sep(s[j]) || s[j] == '(' || s[j] == ')';
    let k = skip_ws(s, e);
    for (w, unit) in TIME_WORDS {
        if w == "m" && !after_t {
            continue;
        }
        if starts(s, k, w) {
            let j = k + w.len();
            if unit == "h" {
                if let Some((m, end)) = minutes_after_hours(s, j, &term) {
                    return Some(Tail {
                        unit,
                        end,
                        written: true,
                        plus_min: m,
                    });
                }
            }
            if term(j) {
                return Some(Tail {
                    unit,
                    end: j,
                    written: true,
                    plus_min: 0.0,
                });
            }
        }
    }
    term(e).then_some(Tail {
        unit: "min",
        end: e,
        written: false,
        plus_min: 0.0,
    })
}

/// A time read from a name: the value as written, its unit and where the match starts and ends.
pub(super) struct Time {
    pub(super) value: f64,
    pub(super) unit: &'static str,
    pub(super) start: usize,
    pub(super) end: usize,
}

/// `t?\d+([.,]\d+)?\s*unit` after a separator: the LAST one with a written unit or a `t` prefix (`Smp_01_30h_2` is 30 h, its `01` and `_2`
/// are an index and a replicate); a bare number (`Run_5`) is never a time. A bare `tN` (also `t=N`, `t N`) is minutes. A day written first
/// (`d1`) counts only when nothing else in the name is a time (`caffeine_D3` stays a doubt for the table, never a guess over a real time).
pub(super) fn time_of(s: &[char]) -> Option<Time> {
    let mut last = None;
    for p in after_prefix(s) {
        // inside the time just read (`1h 30min`: its `30min` is not another one)
        if last.as_ref().is_some_and(|l: &Time| p < l.end) {
            continue;
        }
        let after_t = p < s.len() && s[p] == 't';
        let mut i = if after_t { p + 1 } else { p };
        if after_t {
            // `t=3h`, `t 3`: the `t` is a token of its own
            let k = skip_ws(s, i);
            if k < s.len() && s[k] == '=' && i == k {
                i = skip_ws(s, k + 1);
            } else if k > i {
                i = k;
            }
        }
        for e in number_ends(s, i, after_t) {
            if let Some(tail) = time_tail(s, e, after_t) {
                if !tail.written && !after_t {
                    break;
                }
                let text: String = s[i..e]
                    .iter()
                    .map(|c| if *c == ',' || *c == 'p' { '.' } else { *c })
                    .collect();
                let v: f64 = text.parse().unwrap_or(0.0);
                let (value, unit) = if tail.plus_min > 0.0 {
                    (v * 60.0 + tail.plus_min, "min")
                } else {
                    (v, tail.unit)
                };
                last = Some(Time {
                    value,
                    unit,
                    start: p,
                    end: tail.end,
                });
                break;
            }
        }
    }
    last.or_else(|| day_first(s))
}

/// `d1`, `D14`: a token that is `d` and one to three digits, the day of the experiment; the last one.
fn day_first(s: &[char]) -> Option<Time> {
    let mut last = None;
    for p in after_prefix(s) {
        if p < s.len() && s[p] == 'd' {
            let e = digits(s, p + 1);
            if (1..=3).contains(&(e - p - 1))
                && (e >= s.len() || is_sep(s[e]) || s[e] == '(' || s[e] == ')')
            {
                let v: f64 = s[p + 1..e]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0.0);
                last = Some(Time {
                    value: v,
                    unit: "d",
                    start: p,
                    end: e,
                });
            }
        }
    }
    last
}
