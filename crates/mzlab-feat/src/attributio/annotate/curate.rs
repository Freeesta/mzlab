// Ported from Attributio annotate.py, curation.py, names.py, identity.py.
//! What the cascade asks of the curation of names (the contaminant map, the cleaning of library names, the canonical name of an identity). The tables of
//! the curation belong to another package: the cascade reads them through `Curation`, and `BasicCuration` is the rule without tables (no contaminant map,
//! no synonyms): a name is its own identity and its own canonical name.

/// `(label, category)` of a curated contaminant.
pub type Cec = (String, String);

pub trait Curation {
    /// The contaminant of the first name that matches the map; the names are tried in the order given (most reliable source first).
    fn match_cec(&self, names: &[Option<&str>]) -> Option<Cec>;
    /// Whether a contaminant label is in the headline subset.
    fn is_headline(&self, label: &str) -> bool;
    /// Adduct suffix, collision energy and CCS removed from a raw library name.
    fn clean_library_name(&self, name: &str) -> String;
    /// `(identity, category)` of one annotated row, the rule every count uses.
    fn curate_identity(
        &self,
        cec: Option<&Cec>,
        disp_name: Option<&str>,
        formula: Option<&str>,
    ) -> (String, String);
    /// The canonical name of an identity (`formula` groups the PEG series).
    fn canonical_name(&self, identity: &str, formula: Option<&str>) -> String;
    /// The corrected formula of an identity, by its lowercase key.
    fn formula_override(&self, key: &str) -> Option<String>;
}

/// Curation without tables.
#[derive(Clone, Copy, Debug, Default)]
pub struct BasicCuration;

fn strip_suffix_adduct(s: &str) -> &str {
    // ` [M+H]+`, ` M-H`, ` [M-H]-`: a trailing token that is M, +/- and H
    let t = s.trim_end();
    for tail in [
        "[M+H]+", "[M-H]-", "[M+H]", "[M-H]", "M+H+", "M-H-", "M+H", "M-H", "[M+H+]",
    ] {
        if let Some(head) = t.strip_suffix(tail) {
            if head.ends_with(char::is_whitespace) && !head.trim().is_empty() {
                return head;
            }
        }
    }
    s
}

fn drop_between(s: &str, open: &str, close: char) -> String {
    // removes `\s*` + `open ... close`
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find(open) {
        let (head, tail) = rest.split_at(i);
        let Some(j) = tail.find(close) else {
            break;
        };
        out.push_str(head.trim_end());
        rest = &tail[j + close.len_utf8()..];
    }
    out.push_str(rest);
    out
}

fn drop_collision_energy(s: &str) -> String {
    let low = s.to_lowercase();
    let Some(i) = low.find("collisionenergy") else {
        return s.to_string();
    };
    let head = s[..i].trim_end();
    let tail = &s[i + "collisionenergy".len()..];
    let tail = tail.strip_prefix(':').unwrap_or(tail);
    let rest = tail.trim_start_matches(|c: char| !c.is_whitespace());
    format!("{head}{rest}")
}

fn clean_raw(name: &str) -> String {
    let mut s = name.trim();
    if let Some((a, _)) = s.split_once('|') {
        s = a;
    }
    let low = s.trim_start().to_lowercase();
    let s = if low.starts_with("massbank:") {
        s.trim_start()["massbank:".len()..].trim_start()
    } else {
        s
    };
    let joined = s.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.trim_matches(|c| c == ' ' || c == '-').to_string()
}

/// `identity isomer, RT x.x min` split into the identity and the RT text.
fn split_rt_isomer(raw: &str) -> Option<(&str, &str)> {
    let body = raw.strip_suffix(" min")?;
    let (head, rt) = body.rsplit_once(" isomer, RT ")?;
    let ok = !rt.is_empty()
        && rt.chars().all(|c| c.is_ascii_digit() || c == '.')
        && rt.split('.').count() <= 2
        && !rt.starts_with('.')
        && !rt.ends_with('.');
    (ok && !head.is_empty() && !head.ends_with(char::is_whitespace)).then_some((head, rt))
}

fn looks_like_code(name: &str) -> bool {
    let low = name.trim().to_lowercase();
    [
        "oprea", "acmc", "ncgc", "tocris", "bmse", "acon", "spa-", "epitope", "zinc", "chebi",
        "smr", "stk", "mls", "cas-",
    ]
    .iter()
    .any(|p| low.starts_with(p))
        || low.ends_with("(tentative)")
        || low.ends_with("(sample)")
}

impl Curation for BasicCuration {
    fn match_cec(&self, _names: &[Option<&str>]) -> Option<Cec> {
        None
    }
    fn is_headline(&self, _label: &str) -> bool {
        false
    }
    fn clean_library_name(&self, name: &str) -> String {
        let s = drop_between(name, "[CCS=", ']');
        let s = drop_collision_energy(&s);
        strip_suffix_adduct(&s).trim().to_string()
    }
    fn curate_identity(
        &self,
        cec: Option<&Cec>,
        disp_name: Option<&str>,
        formula: Option<&str>,
    ) -> (String, String) {
        if let Some((label, cat)) = cec.filter(|c| !c.0.trim().is_empty()) {
            return (label.clone(), cat.clone());
        }
        match disp_name.filter(|n| !n.trim().is_empty()) {
            Some(n) => (
                n.to_string(),
                if looks_like_code(n) {
                    "ambiguous"
                } else {
                    "natural"
                }
                .into(),
            ),
            None => match formula.filter(|f| !f.trim().is_empty()) {
                Some(f) => (
                    format!("Unidentified (CSI:FingerID structure, formula {f})"),
                    "ambiguous".into(),
                ),
                None => (
                    "Unidentified (no name available)".into(),
                    "ambiguous".into(),
                ),
            },
        }
    }
    fn canonical_name(&self, identity: &str, formula: Option<&str>) -> String {
        let raw = identity.trim();
        if let Some((head, rt)) = split_rt_isomer(raw) {
            return format!("{} isomer, RT {rt} min", self.canonical_name(head, formula));
        }
        clean_raw(raw)
    }
    fn formula_override(&self, _key: &str) -> Option<String> {
        None
    }
}
