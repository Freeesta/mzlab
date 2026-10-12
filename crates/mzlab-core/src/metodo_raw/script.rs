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
mod tests {
    use super::*;

    const SCRIPT: &str = "---- Overview ----\nName: M\nRun time: 15.000 [min]\n---- Script ----\ninitial     Instrument Setup\n            ColumnComp.CC.Temperature.Nominal: 40.00 [°C]\n            PumpModule.Pump.Pressure.UpperLimit: 14504 [psi]\n            PumpModule.Pump.%A1_Equate: \"Water\"\n0.000 [min] Run\n            PumpModule.Pump.Flow.Nominal: 0.200 [ml/min]\n            PumpModule.Pump.%B.Value: 10.0 [%]\n4.000 [min]\n            PumpModule.Pump.%B.Value: 60.0 [%]\n";

    #[test]
    fn blocks_are_told_apart() {
        assert_eq!(classify(SCRIPT), Block::Script);
        assert_eq!(
            classify("<?xml version=\"1.0\"?><InstrumentSetupMethod/>"),
            Block::Xml
        );
        assert_eq!(
            classify("Method of X\nOVERALL METHOD SETTINGS"),
            Block::Text
        );
        assert_eq!(
            xml_of("text <?xml version=\"1.0\"?><a/>"),
            "<?xml version=\"1.0\"?><a/>"
        );
    }

    #[test]
    fn script_lines_become_key_value_unit_with_the_time_of_their_row() {
        let p = script_pairs(SCRIPT);
        let find = |k: &str, t: Option<f64>| p.iter().find(|x| x.1 == k && x.0 == t).cloned();
        assert_eq!(
            find("ColumnComp.CC.Temperature.Nominal", None).map(|x| (x.2, x.3)),
            Some(("40.00".into(), Some("°C".into())))
        );
        assert_eq!(
            find("PumpModule.Pump.%A1_Equate", None).map(|x| x.2),
            Some("Water".into())
        );
        assert_eq!(
            find("PumpModule.Pump.%B.Value", Some(4.0)).map(|x| x.2),
            Some("60.0".into())
        );
        assert_eq!(
            setting(&p, "PumpModule.Pump.Pressure.UpperLimit"),
            Some((14504.0, Some("psi".into())))
        );
        assert_eq!(setting(&p, "PumpModule.Pump.Flow.Nominal"), None); // only the settings before the first timed row
    }

    #[test]
    fn the_scan_events_of_the_xml_keep_their_numbers_and_units() {
        let xml = "<?xml version=\"1.0\" encoding=\"utf-16\"?><M><ScanEvent id=\"1\"><First_mass unit=\"m/z\">100</First_mass><Last_mass unit=\"m/z\">1000</Last_mass><Resolution>35000</Resolution></ScanEvent><ScanEvent id=\"2\"><Resolution>17500</Resolution><Name>x</Name></ScanEvent></M>";
        let e = experiments(&xml_items(xml));
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].get("indice"), Some(&V::Num(1.0)));
        assert_eq!(
            e[0].get("mz"),
            Some(&V::Arr(vec![V::Num(100.0), V::Num(1000.0)]))
        );
        assert!(e[1].get("mz").is_none());
        let n = |e: &V| match e.get("parametri") {
            Some(V::Arr(a)) => a.len(),
            _ => 0,
        };
        assert_eq!((n(&e[0]), n(&e[1])), (1, 1)); // the range is `mz`; the text of `Name` is no parameter
    }

    #[test]
    fn the_lc_of_a_script_is_the_model_of_the_dam() {
        let lc = lc_of(SCRIPT).unwrap();
        assert_eq!(
            (lc.oven, lc.flow, lc.b0),
            (Some(40.0), Some(0.2), Some(10.0))
        );
        assert_eq!(lc.gradient.len(), 2);
    }
}
