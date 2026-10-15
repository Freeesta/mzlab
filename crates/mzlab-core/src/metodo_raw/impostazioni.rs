//! The settings of the readable text of an Orbitrap/Exactive method that stand outside the scan events: `Label: value [unit]` lines such as the width of the
//! chromatographic peak, the apex trigger, the loop count (Top-N), the intensity threshold, the minimum AGC target, the dynamic exclusion, the exclusion of the isotopes, the
//! charge states, the isolation window and the (stepped) normalised collision energy. A value is a list of numbers (`20, 35, 60`) or a text, as the file writes it.
//! The numbers of the XML scan events (`script::experiments`) are completed from these: a stepped energy is the list of its steps (the XML holds their mean),
//! and the isolation window is a width in m/z (the XML names it `Isolation_high` next to an `Isolation_low` that is no limit).

use super::inclusione::number_list;
use crate::metodo::{obj, text, V};

/// A setting: id, label as written, numbers (when the value is a list of numbers), text (otherwise), unit.
#[derive(Debug, Clone, PartialEq)]
pub struct Setting {
    pub id: &'static str,
    pub label: String,
    pub numbers: Vec<f64>,
    pub text: String,
    pub unit: String,
}

/// Label (letters and digits only, lower case) → id. The first needle contained in the label wins.
const KNOWN: [(&str, &str); 14] = [
    ("chrompeakwidth", "chrom_peak_width"),
    ("apextrigger", "apex_trigger"),
    ("loopcount", "loop_count"),
    ("topn", "loop_count"),
    ("intensitythreshold", "intensity_threshold"),
    ("minimumagctarget", "min_agc_target"),
    ("minagctarget", "min_agc_target"),
    ("dynamicexclusion", "dynamic_exclusion"),
    ("excludeisotopes", "exclude_isotopes"),
    ("excludecharge", "exclude_charge_states"),
    ("chargestate", "charge_states"),
    ("isolationwindow", "isolation_window"),
    ("nce", "nce"),
    ("normalizedcollisionenergy", "nce"),
];

fn key(label: &str) -> String {
    label
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// `Label [unit]: value [unit]` of a line.
fn parse_line(line: &str) -> Option<Setting> {
    let (label, value) = line.split_once(':')?;
    let (label, value) = (label.trim(), value.trim());
    let k = key(label);
    let id = KNOWN.iter().find(|(n, _)| k.contains(n))?.1;
    if value.is_empty() || label.chars().count() > 60 {
        return None;
    }
    let (value, unit) = match value.rfind(" [").filter(|_| value.ends_with(']')) {
        Some(p) => (
            value.get(..p)?.trim(),
            value.get(p + 2..value.len() - 1)?.trim(),
        ),
        None => (value, ""),
    };
    let numbers = number_list(value);
    Some(Setting {
        id,
        label: label.to_string(),
        text: if numbers.is_empty() {
            value.to_string()
        } else {
            String::new()
        },
        numbers,
        unit: unit.to_string(),
    })
}

/// The settings of the readable texts, in file order, without a setting that repeats an earlier one exactly.
pub fn parse(blocks: &[&str]) -> Vec<Setting> {
    let mut out: Vec<Setting> = Vec::new();
    for l in blocks.iter().flat_map(|b| b.lines()).take(20_000) {
        if let Some(s) = parse_line(l) {
            if !out.contains(&s) && out.len() < 64 {
                out.push(s);
            }
        }
    }
    out
}

/// `ms.impostazioni`.
pub fn section(settings: &[Setting]) -> Option<V> {
    let rows: Vec<V> = settings
        .iter()
        .filter_map(|s| {
            obj(vec![
                ("id", text(s.id)),
                ("etichetta", text(&s.label)),
                (
                    "valori",
                    Some(V::Arr(s.numbers.iter().map(|x| V::Num(*x)).collect())),
                ),
                ("valore", text(&s.text)),
                ("unita", text(&s.unit)),
            ])
        })
        .collect();
    (!rows.is_empty()).then_some(V::Arr(rows))
}

fn param(e: &V, id: &str) -> Option<f64> {
    match e.get("parametri") {
        Some(V::Arr(a)) => a.iter().find_map(|p| match (p.get("id"), p.get("valore")) {
            (Some(V::Str(i)), Some(V::Num(v))) if i.eq_ignore_ascii_case(id) => Some(*v),
            _ => None,
        }),
        _ => None,
    }
}

/// Completes the experiments with what the text says: an experiment whose parameter `NCE` is the mean of the steps of the text's stepped energy gets `nce`
/// (the steps) and loses the mean; an experiment with `Isolation_high` (a small width in m/z) gets `isolamento_mz`, and `Isolation_low`/`Isolation_high` (limits
/// they are not) leave `parametri`. Nothing is lost: every XML item stays in `campi`.
pub fn refine(exps: Vec<V>, settings: &[Setting]) -> Vec<V> {
    let steps = settings
        .iter()
        .find(|s| s.id == "nce" && s.numbers.len() > 1)
        .map(|s| s.numbers.clone());
    exps.into_iter()
        .map(|e| {
            let nce = steps.as_ref().filter(|st| {
                let mean = st.iter().sum::<f64>() / st.len() as f64;
                param(&e, "NCE").is_some_and(|v| (v - mean).abs() < 0.01)
            });
            let window = param(&e, "Isolation_high").filter(|w| *w > 0.0 && *w <= 100.0);
            let V::Obj(mut f) = e else { return e };
            if nce.is_none() && window.is_none() {
                return V::Obj(f);
            }
            let drop: &[&str] = match (nce.is_some(), window.is_some()) {
                (true, true) => &["nce", "isolation_high", "isolation_low"],
                (true, false) => &["nce"],
                _ => &["isolation_high", "isolation_low"],
            };
            for (k, v) in &mut f {
                if let (&mut "parametri", V::Arr(a)) = (&mut *k, v) {
                    a.retain(|p| {
                        !matches!(p.get("id"), Some(V::Str(i)) if drop.contains(&i.to_ascii_lowercase().as_str()))
                    });
                }
            }
            f.retain(|(_, v)| !matches!(v, V::Arr(a) if a.is_empty()));
            if let Some(st) = nce {
                f.push(("nce", V::Arr(st.iter().map(|x| V::Num(*x)).collect())));
            }
            if let Some(w) = window {
                f.push(("isolamento_mz", V::Num(w)));
            }
            V::Obj(f)
        })
        .collect()
}

/// The solvents of the LC script: `PumpModule.Pump.%A1_Equate: "Water + 0.1% FA"` → channel `A`, name (the digit after the letter is the position of the bottle).
pub fn solvents(pairs: &[super::script::Pair]) -> Option<V> {
    let rows: Vec<V> = pairs
        .iter()
        .filter(|(t, k, _, _)| t.is_none() && k.ends_with("_Equate"))
        .filter_map(|(_, k, v, _)| {
            let ch = k.rsplit('.').next()?.strip_prefix('%')?.chars().next()?;
            ch.is_ascii_alphabetic().then_some(())?;
            obj(vec![
                ("canale", text(&ch.to_ascii_uppercase().to_string())),
                ("nome", text(v)),
            ])
        })
        .collect();
    (!rows.is_empty()).then_some(V::Arr(rows))
}

#[cfg(test)]
mod tests;
