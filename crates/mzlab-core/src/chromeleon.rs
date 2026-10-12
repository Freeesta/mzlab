//! `parse_chromeleon` of `mzlab/analog.py`: the gradient from the text of a Chromeleon instrument method (`PumpModule.Pump.%B.Value: 60.0 [%]`,
//! `Flow.Nominal`, `Curve: 5`, rows that start with the time in min). The regular expressions of Python are read here by hand (no regex crate).

use crate::error::{Error, Result};
use crate::gradient::{model, num_text, Gradient, Raw};

/// A cursor over the characters of a line.
struct Cur<'a> {
    c: &'a [char],
    i: usize,
}

impl Cur<'_> {
    fn ws(&mut self) {
        while self.i < self.c.len() && self.c[self.i].is_whitespace() {
            self.i += 1;
        }
    }
    /// A word, ignoring case (`re.I`).
    fn word(&mut self, w: &str) -> bool {
        let n = w.chars().count();
        if self.i + n > self.c.len() || !self.c[self.i..self.i + n].iter().zip(w.chars()).all(|(a, b)| a.to_lowercase().eq(b.to_lowercase())) {
            return false;
        }
        self.i += n;
        true
    }
    fn eat(&mut self, ch: char) -> bool {
        let ok = self.c.get(self.i) == Some(&ch);
        self.i += ok as usize;
        ok
    }
    fn digits(&mut self) -> Option<usize> {
        let a = self.i;
        while self.i < self.c.len() && self.c[self.i].is_ascii_digit() {
            self.i += 1;
        }
        (self.i > a).then_some(a)
    }
    /// `\d+(?:[.,]\d+)?` (with `-?` in front when `signed`).
    fn number(&mut self, signed: bool) -> Option<String> {
        let a = self.i;
        if signed {
            self.eat('-');
        }
        self.digits()?;
        let at = self.i;
        if matches!(self.c.get(self.i), Some('.' | ',')) {
            self.i += 1;
            if self.digits().is_none() {
                self.i = at;
            }
        }
        Some(self.c[a..self.i].iter().collect())
    }
    fn assign(&mut self) -> bool {
        self.ws();
        let ok = self.c.get(self.i).is_some_and(|c| *c == ':' || *c == '=');
        self.i += ok as usize;
        self.ws();
        ok
    }
}

/// The first position of the line where `f` matches, with its capture.
fn search<T>(line: &[char], f: impl Fn(&mut Cur) -> Option<T>) -> Option<T> {
    (0..line.len()).find_map(|i| f(&mut Cur { c: line, i }))
}

/// `%\s*B\s*(?:\.\s*Value)?\s*[:=]\s*(-?\d+(?:[.,]\d+)?)`
fn pct_b(c: &mut Cur) -> Option<String> {
    (c.eat('%') && {
        c.ws();
        c.word("b")
    })
    .then_some(())?;
    c.ws();
    dotted(c, "value")
}

/// `Flow\s*(?:\.\s*Nominal)?\s*[:=]\s*(-?\d+(?:[.,]\d+)?)`
fn flow(c: &mut Cur) -> Option<String> {
    c.word("flow").then_some(())?;
    c.ws();
    dotted(c, "nominal")
}

/// `(?:\.\s*<w>)?\s*[:=]\s*(-?number)`
fn dotted(c: &mut Cur, w: &str) -> Option<String> {
    let at = c.i;
    if c.eat('.') {
        c.ws();
        if !c.word(w) {
            c.i = at;
        }
    }
    c.assign().then_some(())?;
    c.number(true)
}

/// `Curve\s*[:=]\s*(\d+)`
fn curve(c: &mut Cur) -> Option<String> {
    c.word("curve").then_some(())?;
    c.assign().then_some(())?;
    let a = c.digits()?;
    Some(c.c[a..c.i].iter().collect())
}

/// `Run\s*Time\s*[:=]\s*(\d+(?:[.,]\d+)?)`
fn run_time(c: &mut Cur) -> Option<String> {
    c.word("run").then_some(())?;
    c.ws();
    c.word("time").then_some(())?;
    c.assign().then_some(())?;
    c.number(false)
}

/// `%\s*([AB])\d?\s*_?\s*Equate\s*[:=]\s*"?([^"\n;]+)` from `from`: (letter, text, end).
fn equate(line: &[char], from: usize) -> Option<(char, String, usize)> {
    (from..line.len()).find_map(|i| {
        let mut c = Cur { c: line, i };
        c.eat('%').then_some(())?;
        c.ws();
        let l = line.get(c.i).copied().filter(|l| matches!(l.to_ascii_lowercase(), 'a' | 'b'))?;
        c.i += 1;
        c.digits();
        c.ws();
        c.eat('_');
        c.ws();
        c.word("equate").then_some(())?;
        c.assign().then_some(())?;
        c.eat('"');
        let a = c.i;
        while c.i < line.len() && !matches!(line[c.i], '"' | '\n' | ';') {
            c.i += 1;
        }
        (c.i > a).then(|| (l.to_ascii_lowercase(), line[a..c.i].iter().collect(), c.i))
    })
}

/// `^\s*(\d+(?:[.,]\d+)?)\s*(?:\[?\s*min\s*\]?)?\s*(?=[A-Za-z%(]|$)`: the time that opens a row.
fn row_time(line: &[char]) -> Option<f64> {
    let mut c = Cur { c: line, i: 0 };
    c.ws();
    let a = c.i;
    c.digits()?;
    let after_int = c.i;
    let mut ends = vec![];
    if matches!(line.get(c.i), Some('.' | ',')) {
        c.i += 1;
        if c.digits().is_some() {
            ends.push(c.i);
        }
    }
    ends.push(after_int);
    let look = |mut c: Cur| {
        c.ws();
        c.c.get(c.i).is_none_or(|ch| ch.is_ascii_alphabetic() || *ch == '%' || *ch == '(')
    };
    let tail_ok = |e: usize| {
        let mut d = Cur { c: line, i: e };
        d.ws();
        d.eat('[');
        d.ws();
        let unit = d.word("min") && {
            d.ws();
            d.eat(']');
            true
        };
        (unit && look(d)) || look(Cur { c: line, i: e })
    };
    let end = ends.into_iter().find(|&e| tail_ok(e))?;
    num_text(&line[a..end].iter().collect::<String>())
}

#[derive(Default, Clone, Copy)]
struct Slot {
    b: Option<f64>,
    flow: Option<f64>,
    curve: Option<i64>,
}

fn clean_lines(text: &str) -> Vec<Vec<char>> {
    let t = text.replace('\r', "\n");
    t.split('\n').map(|l| l.chars().map(|c| if (c == '\t' || c >= ' ') && c != '\x7f' && c != '\u{fffd}' { c } else { ' ' }).collect()).collect()
}

fn has_word(line: &[char], words: &[&str]) -> bool {
    let s: String = line.iter().collect();
    words.iter().any(|w| s.contains(w))
}

/// Gradient from the text of a Chromeleon instrument method. Tolerant: any module name, a statement on the row of the time or on the following lines,
/// values with a decimal comma, broken characters.
pub fn parse(text: &str) -> Result<Gradient> {
    let mut rows: Vec<(f64, Slot)> = Vec::new();
    let mut initial = Slot::default();
    let mut t_cur: Option<f64> = None;
    let mut run: Option<f64> = None;
    let mut solv: Vec<(char, String)> = Vec::new();
    for ln in clean_lines(text) {
        if run.is_none() {
            run = search(&ln, run_time).and_then(|s| num_text(&s));
        }
        let mut from = 0;
        while let Some((l, v, end)) = equate(&ln, from) {
            if !solv.iter().any(|(k, _)| *k == l) {
                solv.push((l, v.trim().to_string()));
            }
            from = end.max(from + 1);
        }
        let (b, fl, cv) = (search(&ln, pct_b), search(&ln, flow), search(&ln, curve));
        if let Some(t) = row_time(&ln).filter(|_| b.is_some() || fl.is_some() || !has_word(&ln, &["Run", "Time", ":"])) {
            t_cur = Some(t);
        }
        let slot = match t_cur {
            Some(t) => {
                let at = rows.iter().position(|(k, _)| *k == t).unwrap_or_else(|| {
                    rows.push((t, Slot::default()));
                    rows.len() - 1
                });
                &mut rows[at].1
            }
            None => &mut initial,
        };
        if let Some(v) = b.as_deref().and_then(num_text) {
            slot.b = Some(v);
        }
        if let Some(v) = fl.as_deref().and_then(num_text) {
            slot.flow = Some(v);
        }
        if let Some(v) = cv.as_deref().and_then(num_text) {
            slot.curve = Some(v as i64);
        }
    }
    let mut rows: Vec<(f64, Slot)> = rows.into_iter().filter(|(_, s)| s.b.is_some() || s.flow.is_some()).collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let pt = |t: f64, b: f64, flow: Option<f64>, curve: Option<i64>| Raw { t: Some(t), b: Some(b), flow, curve, shown: (t.to_string(), b.to_string()) };
    let (mut b, mut flow_v) = (initial.b, initial.flow);
    let mut pts = Vec::new();
    if let (Some(first), Some(b0)) = (rows.first(), b) {
        if first.0 > 0.0 {
            pts.push(pt(0.0, b0, flow_v, None));
        }
    }
    for (t, r) in &rows {
        b = r.b.or(b);
        flow_v = r.flow.or(flow_v);
        if let Some(bv) = b {
            pts.push(pt(*t, bv, flow_v, r.curve));
        }
    }
    if pts.len() < 2 {
        return Err(Error::new("gradient.err.noScript"));
    }
    let get = |k: char| solv.iter().find(|(c, _)| *c == k).map(|(_, v)| v.clone());
    model("raw", &pts, run, (!solv.is_empty()).then(|| (get('a'), get('b'))))
}
