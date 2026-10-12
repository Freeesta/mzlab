//! The text of the instrument method embedded in a `.raw`, in the blocks `opentfraw` finds: the LC script (`---- Overview ----` … `Key: value [unit]`
//! lines and the timed rows), the readable text of the mass spectrometer method and its XML. What the XML says goes through `agilent::meta::collect`
//! (every element and attribute becomes an item, nothing dropped); the script lines become `(key, value, unit)`.

use crate::agilent::meta::{self, Item};
use crate::methodinfo::{GPoint, Lc};
use crate::metodo::{num, obj, text, V};

/// A setting of the LC script: the time of its row (`None` before the first timed row), key, value, unit.
pub type Pair = (Option<f64>, String, String, Option<String>);

/// A numeric setting of a scan event: element name, value, unit.
type Setting = (String, f64, String);

/// The role of a text block of the method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    /// The LC script (`---- Overview ----`).
    Script,
    /// The XML of the mass spectrometer method.
    Xml,
    /// The readable text of the mass spectrometer method.
    Text,
}

pub fn classify(b: &str) -> Block {
    if b.contains("<InstrumentSetupMethod") || b.trim_start().starts_with("<?xml") {
        Block::Xml
    } else if b.contains("---- Overview ----") || b.contains("---- Script ----") {
        Block::Script
    } else {
        Block::Text
    }
}

/// The XML part of a block (the readable text before it may be glued to it by the extraction).
pub fn xml_of(b: &str) -> &str {
    b.find("<?xml")
        .or_else(|| b.find("<InstrumentSetupMethod"))
        .map_or(b, |p| b.get(p..).unwrap_or(b))
}

/// `Key: value [unit]` of one script line (the key starts with a letter and has no blank: `PumpModule.Pump.Flow.Nominal`).
fn pair(line: &str) -> Option<(String, String, Option<String>)> {
    let (k, v) = line.trim().split_once(':')?;
    let k = k.trim();
    let ok = !k.is_empty()
        && k.chars().next().is_some_and(char::is_alphabetic)
        && k.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '.' | '_' | '%'));
    if !ok {
        return None;
    }
    let v = v.trim();
    let (v, unit) = match v.rfind(" [").filter(|_| v.ends_with(']')) {
        Some(p) => (v.get(..p)?.trim(), v.get(p + 2..v.len() - 1)),
        None => (v, None),
    };
    Some((
        k.to_string(),
        v.trim_matches('"').to_string(),
        unit.map(String::from),
    ))
}

/// The settings of the script: the `Key: value [unit]` lines, with the time of the row they belong to (`None` before the first timed row).
pub fn script_pairs(script: &str) -> Vec<Pair> {
    let mut at = None;
    let mut out = Vec::new();
    for l in script.lines() {
        let head = l.split_whitespace().next().unwrap_or("");
        if !l.starts_with(' ') && l.contains("[min]") {
            at = head.replace(',', ".").parse::<f64>().ok();
        } else if let Some((k, v, u)) = pair(l) {
            out.push((at, k, v, u));
        }
    }
    out
}

/// The XML items in document order.
pub fn xml_items(xml: &str) -> Vec<Item> {
    meta::collect(&[("InstrumentMethod.xml", xml.as_bytes())])
}

/// The first value of a script key before the first timed row, as a number.
pub fn setting(p: &[Pair], key: &str) -> Option<(f64, Option<String>)> {
    p.iter()
        .find(|(t, k, _, _)| t.is_none() && k == key)
        .and_then(|(_, _, v, u)| Some((v.replace(',', ".").parse::<f64>().ok()?, u.clone())))
}

/// The scan events of the XML method as `ms.periodi[0].esperimenti`: for each `ScanEvent` its `id` (`indice`), the mass range (`mz`) and its other numeric
/// settings (`parametri`, with the unit the XML gives).
pub fn experiments(items: &[Item]) -> Vec<V> {
    let mut out: Vec<(f64, Vec<Setting>)> = Vec::new();
    let units: Vec<(&str, &str)> = items
        .iter()
        .filter_map(|it| Some((it.name.strip_suffix("@unit")?, it.value.as_str())))
        .collect();
    for it in items {
        let Some(rest) = it.name.split("/ScanEvent").nth(1) else {
            continue;
        };
        if rest == "@id" {
            out.push((it.value.parse().unwrap_or(0.0), Vec::new()));
        } else if let (Some(leaf), Some(cur)) = (rest.strip_prefix('/'), out.last_mut()) {
            if let Ok(v) = it.value.parse::<f64>() {
                let unit = units
                    .iter()
                    .find(|(b, _)| *b == it.name)
                    .map_or("", |u| u.1);
                cur.1.push((leaf.to_string(), v, unit.to_string()));
            }
        }
    }
    out.into_iter()
        .filter_map(|(id, p)| {
            let get = |k: &str| p.iter().find(|x| x.0 == k).map(|x| x.1);
            let range = get("First_mass")
                .zip(get("Last_mass"))
                .map(|(a, b)| V::Arr(vec![V::Num(a), V::Num(b)]));
            let params: Vec<V> = p
                .iter()
                .filter(|x| !matches!(x.0.as_str(), "First_mass" | "Last_mass"))
                .filter_map(|(k, v, u)| {
                    obj(vec![
                        ("id", text(k)),
                        ("valore", num(Some(*v))),
                        ("unita", text(u)),
                    ])
                })
                .collect();
            obj(vec![
                ("indice", num(Some(id))),
                ("mz", range),
                ("parametri", Some(V::Arr(params))),
            ])
        })
        .collect()
}

/// The LC of the script as the shared model (`metodo::lc_sections`): gradient, flow, initial %B, oven, run time.
pub fn lc_of(script: &str) -> Option<Lc> {
    let g = crate::chromeleon::parse(script).ok()?;
    let p = script_pairs(script);
    let gradient: Vec<GPoint> = g
        .points
        .iter()
        .map(|x| GPoint {
            t: x.t,
            b: Some(x.b),
            flow: x.flow,
        })
        .collect();
    Some(Lc {
        flow: g.points.first().and_then(|x| x.flow),
        b0: g.points.first().map(|x| x.b),
        run_time: Some(g.run_time),
        oven: setting(&p, "ColumnComp.CC.Temperature.Nominal").map(|x| x.0),
        gradient,
        pda: Default::default(),
        channels: vec![],
    })
}

#[cfg(test)]
mod tests;
