//! Names and plain-words columns of the Tabui table (Dig): net change against the parent, readable modification and the TP names from the m/z.
//! Pure functions on formulas, rule names and rows; the Python engine (`tpmine/nomi.py`) does the same and the tests hold the same cases.

use std::collections::BTreeMap;

use mzlab_core::elements::parse_formula;

use super::Entry;

const MINUS: char = '\u{2212}';
const NONE: &str = "\u{2014}";

fn rank(el: &str) -> (u8, &str) {
    match el {
        "C" => (0, ""),
        "H" => (1, ""),
        _ => (2, el),
    }
}

/// `−C2H3 +O3`: what the neutral formula loses and gains against the parent (C, H, then alphabetical); `—` when equal, empty when unknown.
pub fn net_change(formula: &str, parent: &str) -> String {
    let (Ok(f), Ok(p)) = (parse_formula(formula), parse_formula(parent)) else {
        return String::new();
    };
    if formula.is_empty() || parent.is_empty() {
        return String::new();
    }
    let mut d: BTreeMap<&str, i64> = BTreeMap::new();
    for (e, n) in &f {
        *d.entry(e.as_str()).or_default() += *n as i64;
    }
    for (e, n) in &p {
        *d.entry(e.as_str()).or_default() -= *n as i64;
    }
    let part = |sign: char, pos: bool| -> Option<String> {
        let mut els: Vec<(&str, i64)> = d
            .iter()
            .filter(|(_, n)| if pos { **n > 0 } else { **n < 0 })
            .map(|(e, n)| (*e, n.abs()))
            .collect();
        els.sort_by(|a, b| rank(a.0).cmp(&rank(b.0)));
        (!els.is_empty()).then(|| {
            let mut s = sign.to_string();
            for (e, n) in els {
                s.push_str(e);
                if n != 1 {
                    s.push_str(&n.to_string());
                }
            }
            s
        })
    };
    let out: Vec<String> = [part(MINUS, false), part('+', true)]
        .into_iter()
        .flatten()
        .collect();
    if out.is_empty() {
        NONE.to_string()
    } else {
        out.join(" ")
    }
}

/// `oxidation_to_carbonyl` -> `Oxidation to carbonyl`; `loss of C4H8 + loss of SO2` -> `Loss of C4H8 and SO2`; the parent gives `—`,
/// an unexpected ion an empty text. The ` @ 13.57 min` of an isomer is not part of the rule.
pub fn modification(name: &str, unexpected: bool) -> String {
    if name == "progenitore" || name == "parent" {
        return NONE.to_string();
    }
    if unexpected || name.is_empty() {
        return String::new();
    }
    let name = match name.rfind(" @ ") {
        Some(i) if name[i + 3..].ends_with(" min") => &name[..i],
        _ => name,
    };
    let steps: Vec<String> = name
        .split(" + ")
        .map(|s| {
            s.replace('_', " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|s| !s.is_empty())
        .collect();
    let mut out: Vec<String> = Vec::new();
    for s in steps {
        let loss = |t: &str| t.to_lowercase().starts_with("loss of ");
        match out.last_mut() {
            Some(last) if loss(&s) && loss(last) && !last.contains(" + ") => {
                last.push_str(" and ");
                last.push_str(&s[8..]);
            }
            Some(_) => {
                let acronym = s.chars().take(2).all(char::is_uppercase);
                let mut c = s.chars();
                let t = if acronym {
                    s.clone()
                } else {
                    c.next()
                        .map(|f| f.to_lowercase().collect::<String>() + c.as_str())
                        .unwrap_or_default()
                };
                out.push(t);
            }
            None => out.push(s),
        }
    }
    let txt = out.join(" + ");
    let mut c = txt.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

fn letters(i: usize) -> String {
    let (mut s, mut i) = (String::new(), i + 1);
    while i > 0 {
        let r = (i - 1) % 26;
        s.insert(0, (b'A' + r as u8) as char);
        i = (i - 1) / 26;
    }
    s
}

/// `"net_change"` and `"modification"` of a row (`—` for the parent, empty for an unexpected ion).
pub fn words_json(o: &mut mzlab_core::json::Out, e: &Entry, parent: &str) {
    let net = if e.unexpected { String::new() } else { net_change(&e.formula, parent) };
    o.raw(",\"net_change\":").str(&net).raw(",\"modification\":").str(&modification(&e.name, e.unexpected));
}

/// Names the entries of one integer m/z: the formulas of `byf` (key -> indexes of `es`) and their isomers.
fn name_group(es: &[Entry], nom: i64, byf: &BTreeMap<String, Vec<usize>>, out: &mut [String]) {
    let min_mz = |v: &Vec<usize>| v.iter().map(|&i| es[i].mz).fold(f64::INFINITY, f64::min);
    let mut keys: Vec<(&String, f64)> = byf.iter().map(|(k, v)| (k, min_mz(v))).collect();
    keys.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(b.0)));
    let mut n = 2;
    if keys.len() > 1 {
        while n < 6 {
            let mut d: Vec<String> = keys.iter().map(|k| digits(k.1, n)).collect();
            d.sort();
            d.dedup();
            if d.len() == keys.len() {
                break;
            }
            n += 1;
        }
    }
    for (k, mz) in &keys {
        let base = if keys.len() > 1 {
            format!("TP {nom}_{}", digits(*mz, n))
        } else {
            format!("TP {nom}")
        };
        let mut iso = byf[*k].clone();
        iso.sort_by(|&a, &b| {
            let (x, y) = (es[a].ref_rt, es[b].ref_rt);
            x.is_none()
                .cmp(&y.is_none())
                .then(x.unwrap_or(0.0).total_cmp(&y.unwrap_or(0.0)))
                .then(es[a].id.cmp(&es[b].id))
        });
        for (j, &i) in iso.iter().enumerate() {
            out[i] = if iso.len() > 1 {
                format!("{base} {}", letters(j))
            } else {
                base.clone()
            };
        }
    }
}

/// The `tp_name` of every entry, in the order of `es`. `TP 362`: the ion m/z rounded to an integer. Another formula with the same integer (without a formula,
/// another m/z at 0.01) adds `_` and the digits after the point of its m/z (2, or more when two collide): `TP 362_31`. The same formula with another RT
/// (isomers) adds ` A`, ` B`... in the order of RT. The parent is `Parent`; a candidate searched but not detected gets no TP name (`Candidate`) and does not count in the groups.
pub fn tp_names(es: &[Entry], detected: &[bool]) -> Vec<String> {
    let mut out = vec![String::new(); es.len()];
    let mut groups: BTreeMap<i64, BTreeMap<String, Vec<usize>>> = BTreeMap::new();
    for (i, e) in es.iter().enumerate() {
        if e.label == "progenitore" {
            out[i] = "Parent".into();
            continue;
        }
        if !detected[i] {
            out[i] = "Candidate".into();
            continue;
        }
        let key = if e.formula.is_empty() {
            format!("mz{:.2}", e.mz)
        } else {
            e.formula.clone()
        };
        groups
            .entry((e.mz + 0.5).floor() as i64)
            .or_default()
            .entry(key)
            .or_default()
            .push(i);
    }
    for (nom, byf) in groups {
        name_group(es, nom, &byf, &mut out);
    }
    out
}

fn digits(mz: f64, n: usize) -> String {
    let s = format!("{mz:.6}");
    s.split('.').nth(1).unwrap_or("").chars().take(n).collect()
}

#[cfg(test)]
#[path = "nomi_tests.rs"]
mod tests;
