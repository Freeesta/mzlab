//! The manual variables of `variabili` (token position, terms), the `nomivar/1` writer of a variable and the part every name shares.

use super::variabili::{build, names_of, read_file, text, val, Fl, Var};
use super::variabili_classi::{fold, split};
use crate::error::Result;
use crate::json::{parse, Out};

/// The variable of a position: the token `n` (1-based, negative from the end) cut at the characters `sep`.
pub(super) fn by_position(fs: &[Fl], n: i64, sep: &str) -> Var {
    let seps: Vec<char> = sep.chars().collect();
    let vals = fs
        .iter()
        .map(|f| {
            let to = f.toks.last().map_or(f.chars.len(), |t| t.b);
            let from = f.toks.first().map_or(0, |t| t.a);
            let t = split(&f.chars, from, to, Some(&seps));
            let i = if n > 0 { n - 1 } else { t.len() as i64 + n };
            let (a, b) = *t.get(usize::try_from(i).ok()?)?;
            let s = text(&f.chars, a, b);
            val(&s, &s, None)
        })
        .collect();
    build(&format!("pos:{n}"), false, vals)
}

/// The variable of terms: a file goes to the first term equal to one of its tokens (or, with `substring`, inside one); the files that match
/// more than one term are listed apart, those that match none have no value.
pub(super) fn by_terms(fs: &[Fl], terms: &[String], substring: bool) -> Var {
    let mut multiple = Vec::new();
    let vals = fs
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let toks: Vec<String> = f.toks.iter().map(|t| fold(&t.text)).collect();
            let hit = |t: &String| {
                let t = fold(t);
                toks.iter()
                    .any(|k| if substring { k.contains(&t) } else { *k == t })
            };
            let n = terms.iter().filter(|t| !t.is_empty() && hit(t)).count();
            if n > 1 {
                multiple.push(i);
            }
            let t = terms.iter().find(|t| !t.is_empty() && hit(t))?;
            val(t, t, None)
        })
        .collect();
    let mut v = build("has", false, vals);
    v.multiple = Some(multiple);
    v
}

pub(super) fn write_var(o: &mut Out, v: &Var, n: usize) {
    let idx = |o: &mut Out, a: &[usize]| {
        o.raw("[");
        for (i, x) in a.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).int(*x as i64);
        }
        o.raw("]");
    };
    o.raw("{\"copertura\":").raw(&format!(
        "{:.4}",
        (n - v.without.len()) as f64 / n.max(1) as f64
    ));
    o.raw(",\"id\":").str(&v.id);
    if let Some(m) = &v.multiple {
        o.raw(",\"multipli\":");
        idx(o, m);
    }
    o.raw(",\"punteggio\":").raw(&format!("{:.4}", v.score));
    o.raw(",\"senza\":");
    idx(o, &v.without);
    o.raw(",\"tipo\":")
        .str(if v.ordered { "ordered" } else { "categorical" });
    o.raw(",\"valori\":[");
    for (i, x) in v.vals.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"chiave\":")
            .str(&x.key);
        o.raw(",\"etichetta\":").str(&x.label).raw(",\"file\":");
        idx(o, &x.files);
        o.raw("}");
    }
    o.raw("]}");
}

/// The characters shared by every name at the start and at the end, whole tokens (`MQ_*_T00`); the whole stem when the names are the same.
pub(super) fn common(fs: &[Fl]) -> String {
    let Some(first) = fs.iter().min_by(|a, b| a.chars.cmp(&b.chars)) else {
        return String::new();
    };
    if fs.len() < 2 {
        return String::new();
    }
    let keys: Vec<Vec<String>> = fs
        .iter()
        .map(|f| f.toks.iter().map(|t| fold(&t.text)).collect())
        .collect();
    let shortest = keys.iter().map(Vec::len).min().unwrap_or(0);
    let pre = (0..shortest)
        .take_while(|&i| keys.iter().all(|k| k[i] == keys[0][i]))
        .count();
    if pre == shortest && keys.iter().all(|k| k.len() == shortest) {
        return match (first.toks.first(), first.toks.last()) {
            (Some(a), Some(b)) => text(&first.chars, a.a, b.b),
            _ => String::new(),
        };
    }
    let suf = (0..shortest - pre)
        .take_while(|&i| {
            keys.iter()
                .all(|k| k[k.len() - 1 - i] == keys[0][keys[0].len() - 1 - i])
        })
        .count();
    let t = &first.toks;
    let head = if pre > 0 {
        text(&first.chars, t[0].a, t[pre - 1].b)
    } else {
        String::new()
    };
    let tail = if suf > 0 {
        text(&first.chars, t[t.len() - suf].a, t[t.len() - 1].b)
    } else {
        String::new()
    };
    if suf == 0 && pre == 0 {
        return String::new();
    }
    format!("{head}*{tail}")
}

/// `campioni(op="nome_gruppo", {"names":[...]})` → `{"nome"}`: the tokens every name has, in the order of the first, joined with the commonest separator.
pub(super) fn route_nome_gruppo(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let names = names_of(&v, "names", "nome_gruppo")?;
    let fs: Vec<Fl> = names.iter().map(|n| read_file(n)).collect();
    let keys: Vec<Vec<String>> = fs
        .iter()
        .map(|f| f.toks.iter().map(|t| fold(&t.text)).collect())
        .collect();
    let mut pick = Vec::new();
    if let Some(first) = fs.first() {
        let mut left: Vec<Vec<&String>> = keys[1..].iter().map(|k| k.iter().collect()).collect();
        for (t, key) in first.toks.iter().zip(&keys[0]) {
            let mut all = true;
            for l in left.iter_mut() {
                match l.iter().position(|x| *x == key) {
                    Some(p) => {
                        l.remove(p);
                    }
                    None => all = false,
                }
            }
            if all {
                pick.push(t.text.as_str());
            }
        }
    }
    let seps = ['_', '-', '.', ' ', '+'];
    let mut count = [0usize; 5];
    for f in &fs {
        for t in &f.toks {
            if let Some(c) = f
                .chars
                .get(t.b)
                .and_then(|c| seps.iter().position(|s| s == c))
            {
                count[c] += 1;
            }
        }
    }
    let best = (0..5).fold(0, |b, i| if count[i] > count[b] { i } else { b });
    let mut o = Out::default();
    o.raw("{\"nome\":")
        .str(&pick.join(&seps[best].to_string()))
        .raw("}");
    Ok(o.0)
}
