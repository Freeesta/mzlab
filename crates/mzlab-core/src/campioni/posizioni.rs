//! Where in the name each proposed field comes from (`proponi_spiegato`): the span (in characters of the whole name, end excluded) of the word, the
//! time, the series prefix, the written replicate or the concentration that decided it, so the page can highlight it. Only a locating step: it
//! never changes a proposed value, and a field whose part cannot be found has no span.

use super::Campione;

/// The part of the name that decided `field` (`tipo`, `tipo_bianco`, `tempo`, `serie`, `replica`, `concentrazione`).
#[derive(Debug, Clone, PartialEq)]
pub struct Posizione {
    pub field: &'static str,
    pub start: usize,
    pub end: usize,
}

/// One file with what `proponi` proposes and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposta {
    pub campione: Campione,
    pub positions: Vec<Posizione>,
}

/// The spans found before the series and the replicates are known.
pub struct Spans {
    ok: bool,
    /// Characters of the folders before the file name: the stem starts here.
    off: usize,
    stem: Vec<char>,
    items: Vec<Posizione>,
}

const BLANK_WORDS: [&str; 3] = ["blank", "blk", "bianco"];
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

fn push(v: &mut Vec<Posizione>, field: &'static str, start: usize, end: usize) {
    v.push(Posizione { field, start, end });
}

/// The first alphanumeric run of `low` equal to one of `words`, or one of them followed by digits only.
fn word_span(low: &[char], words: &[&str]) -> Option<(usize, usize)> {
    let mut i = 0;
    while i < low.len() {
        if !low[i].is_ascii_alphanumeric() {
            i += 1;
            continue;
        }
        let mut e = i;
        while e < low.len() && low[e].is_ascii_alphanumeric() {
            e += 1;
        }
        let tok: String = low[i..e].iter().collect();
        let hit = words.iter().any(|w| {
            tok == *w || (tok.starts_with(w) && tok[w.len()..].chars().all(|c| c.is_ascii_digit()))
        });
        if hit {
            return Some((i, e));
        }
        i = e;
    }
    None
}

fn digits(s: &[char], mut i: usize) -> usize {
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    i
}

/// `\d+([.,]\d+)?` from `i`: the end and the value, or None.
fn number(s: &[char], i: usize) -> Option<(usize, f64)> {
    let d = digits(s, i);
    if d == i {
        return None;
    }
    let mut e = d;
    if d < s.len() && (s[d] == '.' || s[d] == ',') && digits(s, d + 1) > d + 1 {
        e = digits(s, d + 1);
    }
    let text: String = s[i..e]
        .iter()
        .map(|c| if *c == ',' { '.' } else { *c })
        .collect();
    text.parse().ok().map(|v| (e, v))
}

/// A unit written after a number (`mg/L`, `ppm`, `ng_mL`): the end of its letters and its key (letters only, `µ` as `u`).
fn unit_after(s: &[char], mut j: usize) -> (usize, String) {
    while j < s.len() && (s[j].is_whitespace() || s[j] == '_' || s[j] == '-') {
        j += 1;
    }
    let mut key = String::new();
    let mut e = j;
    while e < s.len() && (s[e].is_ascii_alphabetic() || matches!(s[e], '/' | 'µ' | 'μ')) {
        if s[e] != '/' {
            key.push(if matches!(s[e], 'µ' | 'μ') {
                'u'
            } else {
                s[e]
            });
        }
        e += 1;
    }
    (e, key)
}

/// The concentration `conc` (with `unit` or without) in the stem: `7_2ppm`, `5 ppm`, `std 5`.
fn conc_span(s: &[char], conc: f64, unit: Option<&str>) -> Option<(usize, usize)> {
    for i in 0..s.len() {
        if !s[i].is_ascii_digit() || (i > 0 && s[i - 1].is_ascii_digit()) {
            continue;
        }
        let Some(u) = unit else { break };
        if let Some((e, v)) = number(s, i) {
            let (ue, key) = unit_after(s, e);
            if (v - conc).abs() < 1e-9 && key == u {
                return Some((i, ue));
            }
        }
        // laboratory style: 7_2ppm
        let a = digits(s, i);
        if a < s.len() && s[a] == '_' {
            let b = digits(s, a + 1);
            let text: String = s[i..a]
                .iter()
                .chain(['.'].iter())
                .chain(&s[a + 1..b])
                .collect();
            if b > a + 1 && text.parse::<f64>().is_ok_and(|v| (v - conc).abs() < 1e-9) {
                let (ue, key) = unit_after(s, b);
                if key == u {
                    return Some((i, ue));
                }
            }
        }
    }
    if unit.is_none() {
        let (_, we) = word_span(s, &STD_WORDS)?;
        let mut j = we;
        while j < s.len() && (s[j] == '_' || s[j] == '-' || s[j].is_whitespace()) {
            j += 1;
        }
        let (e, v) = number(s, j)?;
        return ((v - conc).abs() < 1e-9).then_some((j, e));
    }
    None
}

/// The spans of the role, the blank kind, the time and the concentration of a file; `time` is the span of the time in the stem.
pub fn of_file(name: &str, c: &Campione, stem: &str, time: Option<(usize, usize)>) -> Spans {
    let chars: Vec<char> = name.chars().collect();
    let low: Vec<char> = name.to_lowercase().chars().collect();
    let stem_c: Vec<char> = stem.chars().collect();
    let stem_low: Vec<char> = stem.to_lowercase().chars().collect();
    let base = name.rsplit('/').next().unwrap_or(name).chars().count();
    let ok = low.len() == chars.len() && stem_low.len() == stem_c.len() && base >= stem_c.len();
    let off = chars.len().saturating_sub(base);
    let mut items = Vec::new();
    if !ok {
        return Spans {
            ok,
            off,
            stem: stem_c,
            items,
        };
    }
    let words = |w: &[&str]| word_span(&low, w);
    let by_time = time.map(|(a, b)| (a + off, b + off));
    let kind_span = match c.kind.as_str() {
        "infusion" => words(&["infusion", "infusione", "fia"]),
        "dark" => words(&["dark", "buio", "adsorption"]),
        "qc" => words(&["qc"]),
        "blank" => words(&BLANK_WORDS),
        "calibration" => words(&[
            "cal",
            "calib",
            "calibr",
            "calibrazione",
            "taratura",
            "curva",
        ]),
        "std" => words(&STD_WORDS),
        "t0" => by_time,
        _ => None,
    };
    if let Some((a, b)) = kind_span {
        push(&mut items, "tipo", a, b);
    }
    if c.blank_kind.is_some() {
        let pre = if c.blank_kind.as_deref() == Some("solvent") {
            "solv"
        } else {
            "proc"
        };
        let mut i = 0;
        while i < low.len() {
            let mut e = i;
            while e < low.len() && low[e].is_ascii_alphanumeric() {
                e += 1;
            }
            if e > i && low[i..e].iter().collect::<String>().starts_with(pre) {
                push(&mut items, "tipo_bianco", i, e);
                break;
            }
            i = e.max(i + 1);
        }
    }
    if let Some((a, b)) = by_time {
        push(&mut items, "tempo", a, b);
    }
    if let Some(v) = c.conc {
        if let Some((a, b)) = conc_span(&stem_low, v, c.unit.as_deref()) {
            push(&mut items, "concentrazione", a + off, b + off);
        }
    }
    Spans {
        ok,
        off,
        stem: stem_c,
        items,
    }
}

impl Spans {
    /// Adds the series (the prefix of the stem that `c.series` names) and the written replicate `(2)`, in the order of the fields.
    pub fn finish(mut self, c: &Campione, written_replicate: bool) -> Vec<Posizione> {
        if !self.ok {
            return Vec::new();
        }
        if !c.series.is_empty() {
            let n = c.series.chars().count();
            if n <= self.stem.len() {
                push(&mut self.items, "serie", self.off, self.off + n);
            }
        }
        if written_replicate {
            let end = self.stem.iter().rposition(|c| *c == ')');
            let start = self.stem.iter().rposition(|c| *c == '(');
            if let (Some(a), Some(b)) = (start, end) {
                push(&mut self.items, "replica", a + self.off, b + 1 + self.off);
            }
        }
        const ORDER: [&str; 6] = [
            "tipo",
            "tipo_bianco",
            "tempo",
            "serie",
            "replica",
            "concentrazione",
        ];
        let mut v = Vec::with_capacity(self.items.len());
        for f in ORDER {
            v.extend(self.items.iter().filter(|p| p.field == f).cloned());
        }
        v
    }
}

/// `{"campioni":<campioni/2>,"posizioni":[[{"campo","inizio","fine"}]]}`: one list of spans per file, in the order of the files.
pub fn proposte_json(p: &[Proposta]) -> String {
    let cs: Vec<Campione> = p.iter().map(|x| x.campione.clone()).collect();
    let mut j = crate::json::Out::default();
    j.raw("{\"campioni\":")
        .raw(&super::to_json(&cs))
        .raw(",\"posizioni\":[");
    for (i, x) in p.iter().enumerate() {
        j.raw(if i > 0 { ",[" } else { "[" });
        for (k, q) in x.positions.iter().enumerate() {
            j.raw(if k > 0 { ",{\"campo\":" } else { "{\"campo\":" })
                .str(q.field)
                .raw(",\"fine\":")
                .int(q.end as i64)
                .raw(",\"inizio\":")
                .int(q.start as i64)
                .raw("}");
        }
        j.raw("]");
    }
    j.raw("]}");
    j.0
}
