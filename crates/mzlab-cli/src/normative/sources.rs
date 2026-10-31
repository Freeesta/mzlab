//! `normative/sources.json`: what is read (the pinned URLs of every list and of the resolvers) and the tags of each list. The pins are SHA-256 of the bytes
//! read: `null` the first time, filled by `--update-pins`, checked by every other build.

use std::path::Path;

use mzlab_core::json::{self, Value};

use super::NormError;

impl Sources {
    pub fn load(path: &Path) -> Result<Sources, NormError> {
        let b = std::fs::read(path).map_err(|e| NormError::io(path, &e))?;
        Sources::parse(&b)
    }

    pub fn parse(b: &[u8]) -> Result<Sources, NormError> {
        let root = json::parse(b).map_err(|e| bad(&e.to_string()))?;
        if root.get("schema").and_then(Value::str) != Some(SCHEMA) {
            return Err(bad(&format!("schema is not {SCHEMA}")));
        }
        let s = Sources { root };
        s.lists()?;
        Ok(s)
    }

    pub fn lists(&self) -> Result<Vec<ListSrc>, NormError> {
        let arr = self
            .root
            .get("lists")
            .map(Value::arr)
            .ok_or_else(|| bad("no lists"))?;
        let mut out: Vec<ListSrc> = Vec::new();
        for l in arr {
            let id = l
                .get("id")
                .and_then(Value::str)
                .ok_or_else(|| bad("a list without id"))?
                .to_string();
            if out.iter().any(|o| o.id == id) {
                return Err(bad(&format!("{id} twice")));
            }
            let reader = l
                .get("reader")
                .and_then(Value::str)
                .ok_or_else(|| bad(&format!("{id}: no reader")))?
                .to_string();
            let urls = l
                .get("urls")
                .map(Value::arr)
                .unwrap_or(&[])
                .iter()
                .filter_map(|u| u.get("url").and_then(Value::str))
                .map(String::from)
                .collect();
            let tags = TAGS
                .iter()
                .map(|t| {
                    (
                        t.to_string(),
                        l.get("tags")
                            .and_then(|g| g.get(t))
                            .and_then(Value::str)
                            .unwrap_or("")
                            .to_string(),
                    )
                })
                .collect();
            out.push(ListSrc {
                id,
                reader,
                urls,
                params: l.get("params").cloned().unwrap_or(Value::Obj(Vec::new())),
                tags,
            });
        }
        Ok(out)
    }

    /// A resolver source of `resolve`: its URL.
    pub fn resolver_url(&self, key: &str) -> Option<String> {
        let v = self.root.get("resolve")?.get(key)?;
        v.str()
            .or_else(|| v.get("url").and_then(Value::str))
            .map(String::from)
    }

    /// Column names of the DSSTox file, if the file is pinned.
    pub fn resolve_params(&self, key: &str) -> Option<&Value> {
        self.root.get("resolve")?.get(key)?.get("columns")
    }

    /// Every pinned URL with its pin.
    pub fn pins(&self) -> Vec<(String, Option<String>)> {
        let mut out = Vec::new();
        let mut one = |v: &Value| {
            if let Some(u) = v.get("url").and_then(Value::str) {
                out.push((
                    u.to_string(),
                    v.get("sha256").and_then(Value::str).map(String::from),
                ));
            }
        };
        if let Some(Value::Obj(r)) = self.root.get("resolve") {
            for (_, v) in r {
                one(v);
            }
        }
        for l in self.root.get("lists").map(Value::arr).unwrap_or(&[]) {
            for u in l.get("urls").map(Value::arr).unwrap_or(&[]) {
                one(u);
            }
        }
        out
    }

    /// Writes `hashes` (url, sha256) into the pins.
    pub fn set_pins(&mut self, hashes: &[(String, String)]) {
        fn fix(v: &mut Value, hashes: &[(String, String)]) {
            if let Value::Obj(m) = v {
                let url = m
                    .iter()
                    .find(|(k, _)| k == "url")
                    .and_then(|(_, u)| u.str().map(String::from));
                if let Some(h) =
                    url.and_then(|u| hashes.iter().find(|(x, _)| *x == u).map(|(_, h)| h.clone()))
                {
                    match m.iter_mut().find(|(k, _)| k == "sha256") {
                        Some((_, s)) => *s = Value::Str(h),
                        None => m.push(("sha256".into(), Value::Str(h))),
                    }
                }
                for (_, x) in m.iter_mut() {
                    fix(x, hashes);
                }
            } else if let Value::Arr(a) = v {
                for x in a.iter_mut() {
                    fix(x, hashes);
                }
            }
        }
        fix(&mut self.root, hashes);
    }

    pub fn to_text(&self) -> String {
        let mut s = String::new();
        pretty(&self.root, 0, &mut s);
        s.push('\n');
        s
    }
}

pub const SCHEMA: &str = "normative-sources/1";

/// The tags of study §4 kept per list, in the order of the provenance.
pub const TAGS: [&str; 10] = [
    "jurisdiction",
    "instrument",
    "citation",
    "annex",
    "status",
    "in_force_from",
    "applies_from",
    "version_date",
    "licence",
    "attribution",
];

pub struct ListSrc {
    pub id: String,
    pub reader: String,
    pub urls: Vec<String>,
    pub params: Value,
    pub tags: Vec<(String, String)>,
}

pub struct Sources {
    pub root: Value,
}

fn bad(msg: &str) -> NormError {
    NormError::sources(msg)
}

fn scalar(v: &Value) -> bool {
    !matches!(v, Value::Arr(_) | Value::Obj(_))
}

fn pretty(v: &Value, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth + 1);
    match v {
        Value::Arr(a) if a.iter().all(scalar) => {
            let items: Vec<String> = a
                .iter()
                .map(|x| {
                    let mut o = json::Out::default();
                    json::write_value(&mut o, x);
                    o.0
                })
                .collect();
            out.push_str(&format!("[{}]", items.join(", ")));
        }
        Value::Arr(a) => {
            out.push_str("[\n");
            for (i, x) in a.iter().enumerate() {
                out.push_str(&pad);
                pretty(x, depth + 1, out);
                out.push_str(if i + 1 < a.len() { ",\n" } else { "\n" });
            }
            out.push_str(&"  ".repeat(depth));
            out.push(']');
        }
        Value::Obj(m) if m.is_empty() => out.push_str("{}"),
        Value::Obj(m) => {
            out.push_str("{\n");
            for (i, (k, x)) in m.iter().enumerate() {
                let mut o = json::Out::default();
                o.str(k);
                out.push_str(&pad);
                out.push_str(&o.0);
                out.push_str(": ");
                pretty(x, depth + 1, out);
                out.push_str(if i + 1 < m.len() { ",\n" } else { "\n" });
            }
            out.push_str(&"  ".repeat(depth));
            out.push('}');
        }
        _ => {
            let mut o = json::Out::default();
            json::write_value(&mut o, v);
            out.push_str(&o.0);
        }
    }
}
