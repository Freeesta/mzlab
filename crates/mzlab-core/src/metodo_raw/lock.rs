//! The lock mass (internal calibrant) of a Thermo Orbitrap `.raw`, for `metodo/1.ms.calibrazione.lock_mass`: what the readable method says (`Use lock masses best`, the
//! `Lock Masses:` block with the `Pos`/`Neg List Name` and `Mass List`) and what the scan trailer records (`Number of Lock Masses`, `LM m/z-Correction (ppm)`). It only describes: the data are never corrected here.

use crate::metodo::{num, obj, text, V};
use opentfraw::RawFileReader;

/// A list of the method: polarity, name and masses (`(none)` is an empty list).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct List {
    pub polarity: &'static str,
    pub name: String,
    pub masses: Vec<f64>,
}

/// What a method text says about the lock masses.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FromMethod {
    /// `best`, `off`, … after `Use lock masses`.
    pub mode: Option<String>,
    pub lists: Vec<List>,
}

fn value_of<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.trim_start().strip_prefix(key)?;
    let v = rest.trim();
    (!v.is_empty()).then_some(v)
}

/// The lock mass lines of the method text `t` (`None`: the text does not mention them).
pub fn from_method(t: &str) -> Option<FromMethod> {
    let mut out = FromMethod::default();
    let mut lines = t.lines().map(|l| l.trim_end_matches('\r')).peekable();
    while let Some(l) = lines.next() {
        if out.mode.is_none() {
            if let Some(v) = value_of(l, "Use lock masses") {
                out.mode = text(v.trim_matches('―').trim()).and_then(|v| match v {
                    V::Str(s) => Some(s),
                    _ => None,
                });
            }
        }
        if l.trim() != "Lock Masses:" {
            continue;
        }
        // up to two lists, each of `<Pos|Neg> List Name:`, `Source:` and `Mass List:`
        let mut cur: Option<List> = None;
        while let Some(n) = lines.peek().copied() {
            let polarity = if n.trim_start().starts_with("Pos List Name:") {
                Some("positive")
            } else if n.trim_start().starts_with("Neg List Name:") {
                Some("negative")
            } else {
                None
            };
            if let Some(p) = polarity {
                out.lists.extend(cur.take());
                cur = Some(List {
                    polarity: p,
                    name: n.split_once(':').map_or("", |x| x.1).trim().to_string(),
                    ..Default::default()
                });
            } else if n.trim_start().starts_with("Source:") {
                // `API Source`, the same in every file read
            } else if let (Some(c), Some(v)) = (cur.as_mut(), value_of(n, "Mass List:")) {
                c.masses = v
                    .split([',', ';', ' '])
                    .filter_map(|x| x.trim().parse::<f64>().ok())
                    .filter(|x| x.is_finite() && *x > 0.0)
                    .collect();
            } else if !n.trim().is_empty() {
                break;
            } else if cur.is_some() && out.lists.len() + 1 >= 2 {
                break;
            }
            lines.next();
        }
        out.lists.extend(cur.take());
        break;
    }
    (out.mode.is_some() || !out.lists.is_empty()).then_some(out)
}

/// `[min, max]` of the numbers of the trailer field named `label` over the scans.
fn range(raw: &RawFileReader, label: &str) -> Option<(f64, f64)> {
    let i = raw
        .scan_parameters_header
        .fields
        .iter()
        .position(|d| d.label.trim().trim_end_matches(':').trim() == label)?;
    let mut r: Option<(f64, f64)> = None;
    for row in &raw.scan_parameters {
        let Some(x) = row.values.get(i).and_then(|v| v.1.as_f64()) else {
            continue;
        };
        if x.is_finite() {
            r = Some(r.map_or((x, x), |(a, b)| (a.min(x), b.max(x))));
        }
    }
    r
}

fn pair(r: Option<(f64, f64)>) -> Option<V> {
    r.map(|(a, b)| V::Arr(vec![V::Num(a), V::Num(b)]))
}

/// The `lock_mass` object of `ms.calibrazione`: the mode and the lists of the method, how many lock masses the trailer says and the range of the correction. The masses
/// and the search window of the trailer stay in `campi` (`Lock Mass #n (m/z)`, `LM Search Window (ppm)`).
pub fn section(raw: &RawFileReader, blocks: &[String]) -> Option<V> {
    let m = blocks.iter().find_map(|b| from_method(b));
    let lists: Vec<V> = m
        .iter()
        .flat_map(|m| m.lists.iter())
        .filter_map(|l| {
            obj(vec![
                ("polarita", Some(V::s(l.polarity))),
                ("nome", text(&l.name).filter(|_| l.name != "N/A")),
                (
                    "masse_mz",
                    Some(V::Arr(l.masses.iter().map(|x| V::Num(*x)).collect())),
                ),
            ])
        })
        .collect();
    obj(vec![
        (
            "modo",
            m.as_ref().and_then(|m| m.mode.as_deref()).and_then(text),
        ),
        ("elenchi", (!lists.is_empty()).then(|| V::Arr(lists))),
        (
            "numero_nel_trailer",
            num(range(raw, "Number of Lock Masses").map(|r| r.1)),
        ),
        (
            "correzione_ppm",
            pair(range(raw, "LM m/z-Correction (ppm)")),
        ),
    ])
}

#[cfg(test)]
mod tests;
