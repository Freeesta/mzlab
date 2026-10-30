//! `variabili` and `nome_gruppo`: how the NAMES of a set of files split them (matrix, condition, time, role, concentration, replicate), ranked as grouping keys
//! (`nomivar/1`, `docs/agenti/campioni.md`). Role, time and concentration come from the parser of `nomi.rs`; the pieces are cut by `variabili_classi.rs`.
//! The same answer for every order of the files (the order only decides the order of `files[]` and the indices that follow it).

use super::nomi::ruolo_e_tempo;
use super::ordine::{base, stem};
use super::variabili_classi::{
    fold, is_blank_word, is_cond_word, is_role_word, natural, split, time_token, tokenize, Tok,
    UNITS,
};
pub(super) use super::variabili_uscita::route_nome_gruppo;
use super::variabili_uscita::{by_position, by_terms, common, write_var};
use crate::error::{Error, Result};
use crate::json::{fmt_g, parse, Out, Value};
use crate::project::guess_conc;
use std::collections::BTreeMap;

pub const SCHEMA_NOMIVAR: &str = "nomivar/1";
const MAX_NAMES: usize = 5000;
const MAX_LEN: usize = 256;
/// The automatic variables in the order they are listed when their score is good.
const ORDER: [&str; 8] = [
    "matrix", "cond", "time", "role", "matcond", "conc", "rep", "folder",
];

/// The files of one value while a variable is built: the number that orders it, the spellings with their counts, the files.
type Bucket = (Option<f64>, BTreeMap<String, usize>, Vec<usize>);

/// A value of a file: the key (folded), the label as written, the number that orders it.
pub(super) type Val = Option<(String, String, Option<f64>)>;

pub(super) struct Fl {
    pub(super) chars: Vec<char>,
    pub(super) dir: String,
    pub(super) toks: Vec<Tok>,
    pub(super) kind: &'static str,
    pub(super) time: Option<(f64, usize, usize)>,
    pub(super) conc: Option<(f64, String)>,
    pub(super) matrix: Option<usize>,
    pub(super) rep: Option<u32>,
}

pub(super) struct Value1 {
    pub(super) key: String,
    pub(super) label: String,
    pub(super) ord: Option<f64>,
    pub(super) files: Vec<usize>,
}

pub(super) struct Var {
    pub(super) id: String,
    pub(super) ordered: bool,
    pub(super) vals: Vec<Value1>,
    pub(super) without: Vec<usize>,
    pub(super) multiple: Option<Vec<usize>>,
    pub(super) score: f64,
}

fn bad(op: &str) -> Error {
    Error::new("error.badCampioni").with("op", op)
}

pub(super) fn names_of(v: &Value, key: &str, op: &str) -> Result<Vec<String>> {
    let list = v.get(key).map(|a| a.arr()).unwrap_or_default();
    if list.len() > MAX_NAMES {
        return Err(Error::new("error.badCampioni").with("limite", MAX_NAMES));
    }
    list.iter()
        .map(|f| {
            let s = if key == "files" {
                f.get("name").and_then(|x| x.str())
            } else {
                f.str()
            };
            let s = s.ok_or_else(|| bad(op))?;
            if s.chars().count() > MAX_LEN {
                return Err(Error::new("error.badCampioni").with("limite", MAX_LEN));
            }
            Ok(s.to_string())
        })
        .collect()
}

fn dir_of(name: &str) -> String {
    let b = base(name);
    name[..name.len() - b.len()]
        .trim_end_matches(['/', '\\'])
        .replace('\\', "/")
}

pub(super) fn text(chars: &[char], a: usize, b: usize) -> String {
    chars[a.min(chars.len())..b.min(chars.len())]
        .iter()
        .collect()
}

/// The facts of one name: tokens with their spans, role, time, concentration.
pub(super) fn read_file(name: &str) -> Fl {
    let chars: Vec<char> = name.chars().collect();
    let st = stem(name);
    let to = chars.len() - (base(name).chars().count() - st.chars().count());
    let from = chars.len() - base(name).chars().count();
    let toks: Vec<Tok> = tokenize(&chars, from, to)
        .into_iter()
        .map(|(a, b)| Tok {
            text: text(&chars, a, b),
            a,
            b,
            class: "word",
        })
        .collect();
    let (kind, time) = ruolo_e_tempo(name);
    let mut time = time.map(|(m, a, b)| (m, (a + from).min(to), (b + from).min(to)));
    // the parser reads a time after a separator: a name glued together (`MQH2O2T00`) has its time in the last token
    if time.is_none() && matches!(kind, "sample" | "t0") {
        time = toks
            .iter()
            .rev()
            .find_map(|t| time_token(&t.text).map(|m| (m, t.a, t.b)));
    }
    let conc = guess_conc(name).map(|(v, u)| {
        let unit = u.unwrap_or_default();
        let f = UNITS.iter().find(|x| x.0 == unit).and_then(|x| x.1);
        let shown = match unit.as_str() {
            "mgl" => "mg/L",
            "gl" => "g/L",
            "ugl" => "\u{b5}g/L",
            "ngl" => "ng/L",
            "ngml" => "ng/mL",
            o => o,
        };
        let label = if shown.is_empty() {
            fmt_g(v)
        } else {
            format!("{} {shown}", fmt_g(v))
        };
        (v * f.unwrap_or(1.0), label)
    });
    Fl {
        chars,
        dir: dir_of(name),
        toks,
        kind,
        time,
        conc,
        matrix: None,
        rep: None,
    }
}

/// A number then a unit in one token (`1ppm`), or the unit alone.
fn unit_of(tok: &str) -> Option<bool> {
    let low = fold(tok);
    let rest = low.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ',');
    let numeric = rest.len() < low.len();
    UNITS.iter().any(|u| u.0 == rest).then_some(numeric)
}

/// The first token as a matrix: letters only, not a role or condition, in 2 to 6 distinct spellings that each start two files at least.
fn find_matrix(fs: &mut [Fl]) {
    let word = |f: &Fl| {
        let t = f.toks.first()?;
        let low = fold(&t.text);
        let ok = f.toks.len() > 1
            && (2..=6).contains(&low.chars().count())
            && low.chars().all(|c| c.is_ascii_alphabetic())
            && !is_role_word(&low)
            && !is_cond_word(&t.text);
        ok.then_some(low)
    };
    let words: Vec<Option<String>> = fs.iter().map(word).collect();
    let mut counts: BTreeMap<&String, usize> = BTreeMap::new();
    for w in words.iter().flatten() {
        *counts.entry(w).or_default() += 1;
    }
    counts.retain(|_, n| *n >= 2);
    if !(2..=6).contains(&counts.len()) {
        return;
    }
    for (f, w) in fs.iter_mut().zip(&words) {
        if w.as_ref().is_some_and(|w| counts.contains_key(w)) {
            f.matrix = Some(0);
        }
    }
}

/// The class of every token, and the replicate of the files that have one.
fn classify(fs: &mut [Fl]) {
    find_matrix(fs);
    for f in fs.iter_mut() {
        let n = f.toks.len();
        for i in 0..n {
            let (a, b, low) = (f.toks[i].a, f.toks[i].b, fold(&f.toks[i].text));
            let in_time = f.time.is_some_and(|t| a < t.2 && b > t.1);
            let unit_next = i + 1 < n && unit_of(&f.toks[i + 1].text) == Some(false);
            let numeric = low.chars().all(|c| c.is_ascii_digit() || c == '.');
            f.toks[i].class = if in_time {
                "time"
            } else if f.conc.is_some()
                && (unit_of(&low) == Some(true)
                    || (numeric && unit_next)
                    || unit_of(&low) == Some(false))
            {
                "conc"
            } else if i == 0 && f.matrix.is_some() {
                "matrix"
            } else if is_role_word(&low) || is_blank_word(&low) {
                "role"
            } else if is_cond_word(&f.toks[i].text) {
                "cond"
            } else if numeric {
                "num"
            } else {
                "word"
            };
        }
    }
    // a replicate: a small number at the end, and the same name without it occurs with another number
    let mut seen: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    let mut cand = Vec::new();
    for (k, f) in fs.iter().enumerate() {
        let Some(t) = f
            .toks
            .last()
            .filter(|t| t.class == "num" || t.class == "word")
        else {
            continue;
        };
        let low = fold(&t.text);
        let digits = low
            .strip_prefix("rep")
            .or_else(|| low.strip_prefix('r'))
            .unwrap_or(&low);
        let Some(n) = digits.parse::<u32>().ok().filter(|n| (1..=20).contains(n)) else {
            continue;
        };
        let rest: Vec<String> = f.toks[..f.toks.len() - 1]
            .iter()
            .map(|t| fold(&t.text))
            .collect();
        let key = format!("{}\u{1}{}", f.dir, rest.join("\u{1}"));
        seen.entry(key.clone()).or_default().push(n);
        cand.push((k, key, n));
    }
    for (k, key, n) in cand {
        let ns = &seen[&key];
        if ns.iter().any(|x| *x != n) {
            fs[k].rep = Some(n);
            if let Some(t) = fs[k].toks.last_mut() {
                t.class = "rep";
            }
        }
    }
}

/// A variable from the value of every file; None when no file has one.
pub(super) fn build(id: &str, ordered: bool, vals: Vec<Val>) -> Var {
    let mut by: BTreeMap<String, Bucket> = BTreeMap::new();
    let mut without = Vec::new();
    for (i, v) in vals.into_iter().enumerate() {
        match v {
            Some((k, l, o)) => {
                let e = by.entry(k).or_insert((o, BTreeMap::new(), Vec::new()));
                *e.1.entry(l).or_default() += 1;
                e.2.push(i);
            }
            None => without.push(i),
        }
    }
    let mut vs: Vec<Value1> = by
        .into_iter()
        .map(|(key, (ord, spell, files))| {
            // the most frequent spelling; the natural order breaks a tie
            let label = spell
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then_with(|| natural(&b.0, &a.0)))
                .map(|x| x.0)
                .unwrap_or_default();
            Value1 {
                key,
                label,
                ord,
                files,
            }
        })
        .collect();
    if ordered {
        crate::sortc::slice_by(&mut vs, |a, b| {
            a.ord
                .partial_cmp(&b.ord)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| natural(&a.key, &b.key))
        });
    } else {
        crate::sortc::slice_by(&mut vs, |a, b| {
            b.files
                .len()
                .cmp(&a.files.len())
                .then_with(|| natural(&a.label, &b.label))
        });
    }
    let with: usize = vs.iter().map(|v| v.files.len()).sum();
    let n = with + without.len();
    let k = vs.len();
    // evenness of the group sizes with + × / only (the same bits in the .wasm and in the app): 1 - sum p² over its smallest value
    let balance = if k < 2 {
        0.0
    } else {
        let sq: f64 = vs
            .iter()
            .map(|v| (v.files.len() as f64 / with as f64).powi(2))
            .sum();
        (1.0 - sq) / (1.0 - 1.0 / k as f64)
    };
    let covered = if n == 0 { 0.0 } else { with as f64 / n as f64 };
    Var {
        id: id.to_string(),
        ordered,
        vals: vs,
        without,
        multiple: None,
        score: covered * balance,
    }
}

pub(super) fn val(key: &str, label: &str, ord: Option<f64>) -> Val {
    Some((fold(key), label.to_string(), ord))
}

/// The automatic variables of a set of read files, hidden ones removed, in the order the menu lists them.
fn variables(fs: &[Fl]) -> Vec<Var> {
    let cond = |f: &Fl| -> Option<(String, String)> {
        let (_, ts, _) = f.time?;
        let words: Vec<&str> = f
            .toks
            .iter()
            .enumerate()
            .filter(|(i, t)| t.a < ts && !(*i == 0 && f.matrix.is_some()))
            .map(|(_, t)| t.text.as_str())
            .collect();
        (!words.is_empty()).then(|| (fold(&words.join("_")), words.join("_")))
    };
    let matrix = |f: &Fl| f.matrix.map(|_| f.toks[0].text.clone());
    let mut all: Vec<Var> = Vec::new();
    let mut add = |id: &str, ordered: bool, g: &dyn Fn(&Fl) -> Val| {
        all.push(build(id, ordered, fs.iter().map(g).collect()));
    };
    add("matrix", false, &|f| {
        matrix(f).and_then(|m| val(&m, &m, None))
    });
    add("cond", false, &|f| {
        cond(f).and_then(|c| val(&c.0, &c.1, None))
    });
    add("time", true, &|f| {
        let (m, a, b) = f.time?;
        val(&fmt_g(m), &text(&f.chars, a, b), Some(m))
    });
    add("role", false, &|f| {
        let r = if f.kind == "t0" { "sample" } else { f.kind };
        val(r, r, None)
    });
    add("matcond", false, &|f| {
        let (m, c) = (matrix(f)?, cond(f)?);
        val(
            &format!("{m}\u{b7}{}", c.0),
            &format!("{m}\u{b7}{}", c.1),
            None,
        )
    });
    add("conc", true, &|f| {
        let (v, l) = f.conc.as_ref()?;
        val(&fmt_g(*v), l, Some(*v))
    });
    add("rep", true, &|f| {
        let n = f.rep?;
        val(&n.to_string(), &n.to_string(), Some(n as f64))
    });
    let dirs: Vec<&String> = fs.iter().map(|f| &f.dir).collect();
    if dirs.iter().any(|d| *d != dirs[0]) {
        add("folder", false, &|f| {
            (!f.dir.is_empty()).then(|| val(&f.dir, &f.dir, None))?
        });
    }
    all.retain(|v| {
        let with = fs.len() - v.without.len();
        let identifier = !v.ordered && with >= 3 && v.vals.len() == with;
        v.vals.len() >= 2 && !identifier && v.score >= 0.05
    });
    let pos = |v: &Var| ORDER.iter().position(|o| *o == v.id).unwrap_or(ORDER.len());
    // a good variable keeps its place in the menu while files arrive; the weak ones follow by score
    crate::sortc::slice_by(&mut all, |a, b| {
        let tier = |v: &Var| usize::from(v.score < 0.25);
        tier(a).cmp(&tier(b)).then_with(|| {
            if tier(a) == 0 {
                pos(a).cmp(&pos(b))
            } else {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| pos(a).cmp(&pos(b)))
            }
        })
    });
    all
}

/// `campioni(op="variabili", {"files":[{"name"}], "manuale"?: {"pos", "sep"?} | {"has": [terms], "substring"?}})` → `nomivar/1`.
pub(super) fn route_variabili(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let names = names_of(&v, "files", "variabili")?;
    let mut fs: Vec<Fl> = names.iter().map(|n| read_file(n)).collect();
    classify(&mut fs);
    let n = fs.len();
    let folders = fs.iter().any(|g| g.dir != fs[0].dir);
    let mut o = Out::default();
    o.raw("{\"comune\":").str(&common(&fs)).raw(",\"files\":[");
    for (i, (f, name)) in fs.iter().zip(&names).enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(name)
            .raw(",\"tokens\":[");
        let dir_toks = split(
            &f.chars,
            0,
            f.chars.len() - base(name).chars().count(),
            Some(&['/', '\\']),
        );
        let all = dir_toks
            .iter()
            .filter(|_| folders)
            .map(|&(a, b)| (a, b, "folder"))
            .chain(f.toks.iter().map(|t| (t.a, t.b, t.class)));
        for (j, (a, b, c)) in all.enumerate() {
            o.raw(if j > 0 { "," } else { "" })
                .raw("{\"classe\":")
                .str(c);
            o.raw(",\"fine\":")
                .int(b as i64)
                .raw(",\"inizio\":")
                .int(a as i64);
            o.raw(",\"testo\":").str(&text(&f.chars, a, b)).raw("}");
        }
        o.raw("]}");
    }
    o.raw("]");
    if let Some(m) = v.get("manuale").filter(|m| !m.is_null()) {
        let var = if let Some(p) = m.get("pos") {
            let p = p
                .f64()
                .filter(|p| p.fract() == 0.0 && *p != 0.0)
                .ok_or_else(|| bad("variabili"))?;
            let sep = m.get("sep").and_then(|s| s.str()).unwrap_or("_-. ");
            by_position(&fs, p as i64, sep)
        } else if let Some(t) = m.get("has") {
            let terms: Vec<String> = t
                .arr()
                .iter()
                .filter_map(|x| x.str().map(String::from))
                .collect();
            by_terms(
                &fs,
                &terms,
                m.get("substring").and_then(|b| b.bool()).unwrap_or(false),
            )
        } else {
            return Err(bad("variabili"));
        };
        o.raw(",\"manuale\":");
        write_var(&mut o, &var, n);
    }
    o.raw(",\"schema\":")
        .str(SCHEMA_NOMIVAR)
        .raw(",\"variabili\":[");
    for (i, var) in variables(&fs).iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        write_var(&mut o, var, n);
    }
    o.raw("]}");
    Ok(o.0)
}
