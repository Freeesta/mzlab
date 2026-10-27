//! Library presets (`data/preset-1.json`): a named view over the open libraries, never a copy. A preset lists catalog ids (a trailing `*` matches a prefix)
//! and/or library kinds; `mask` turns it into the `u16` bit mask (bit i = open library i) the multi-library search uses.
use serde_json::Value;

const PRESETS: &str = include_str!("../data/preset-1.json");

/// What a preset needs to know about an open library: its catalog id (`massbank-pos`, the name of its `.mzlib`) and its `kind` (`experimental`, `in_silico`).
pub struct OpenLib<'a> {
    pub id: &'a str,
    pub kind: &'a str,
}

fn matches(pat: &str, id: &str) -> bool {
    match pat.strip_suffix('*') {
        Some(prefix) => id.starts_with(prefix),
        None => pat == id,
    }
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// The ids of the presets, in file order.
pub fn ids() -> Vec<String> {
    let doc: Value = serde_json::from_str(PRESETS).unwrap_or(Value::Null);
    doc.get("preset")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|p| p.get("id").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Bit mask of the open libraries a preset selects (at most 16 libraries; later ones are ignored); `None` for an unknown preset.
pub fn mask(preset: &str, libs: &[OpenLib]) -> Option<u16> {
    let doc: Value = serde_json::from_str(PRESETS).ok()?;
    let p = doc
        .get("preset")?
        .as_array()?
        .iter()
        .find(|p| p.get("id").and_then(Value::as_str) == Some(preset))?;
    let (ids, kinds) = (strings(p, "ids"), strings(p, "kinds"));
    let mut m = 0u16;
    for (i, l) in libs.iter().take(16).enumerate() {
        if kinds.iter().any(|k| k == l.kind) || ids.iter().any(|pat| matches(pat, l.id)) {
            m |= 1 << i;
        }
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn libs() -> Vec<OpenLib<'static>> {
        vec![
            OpenLib {
                id: "massbank-pos",
                kind: "experimental",
            },
            OpenLib {
                id: "lipidblast-2022",
                kind: "in_silico",
            },
            OpenLib {
                id: "msdial-gcms-fiehnri",
                kind: "experimental",
            },
            OpenLib {
                id: "isdb-lotus",
                kind: "in_silico",
            },
            OpenLib {
                id: "mine",
                kind: "experimental",
            },
        ]
    }

    #[test]
    fn a_preset_selects_by_id_prefix_and_by_kind() {
        assert_eq!(mask("experimental", &libs()), Some(0b10101));
        assert_eq!(mask("environmental", &libs()), Some(0b00101));
        assert_eq!(mask("metabolomics", &libs()), Some(0b00110));
        assert_eq!(mask("natural_products", &libs()), Some(0b01000));
        assert_eq!(mask("forensic", &libs()), Some(0));
        assert_eq!(mask("nope", &libs()), None);
    }

    #[test]
    fn the_five_presets_are_listed_and_a_mask_stops_at_sixteen_libraries() {
        assert_eq!(
            ids(),
            [
                "experimental",
                "environmental",
                "metabolomics",
                "natural_products",
                "forensic"
            ]
        );
        let many: Vec<OpenLib> = (0..20)
            .map(|_| OpenLib {
                id: "x",
                kind: "experimental",
            })
            .collect();
        assert_eq!(mask("experimental", &many), Some(u16::MAX));
    }
}
