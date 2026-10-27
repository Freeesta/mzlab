//! `libmatch/1` as JSON text with a fixed field order (docs/schemi/libmatch-1.json).

use super::*;
use std::fmt::Write;

impl MatchResult {
    /// The result as `libmatch/1` JSON.
    pub fn to_json(&self) -> String {
        let mut o = String::from("{");
        field(&mut o, "schema");
        text(&mut o, "libmatch/1");
        field(&mut o, "hr");
        o.push_str(if self.hr { "true" } else { "false" });
        field(&mut o, "mask");
        let _ = write!(o, "{}", self.mask);
        field(&mut o, "top_compounds");
        let _ = write!(o, "{}", self.top_compounds);
        field(&mut o, "libraries");
        o.push('[');
        for (i, l) in self.libraries.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            o.push('{');
            field(&mut o, "index");
            let _ = write!(o, "{}", l.index);
            field(&mut o, "name");
            text(&mut o, &l.name);
            field(&mut o, "kind");
            text(&mut o, l.kind);
            o.push('}');
        }
        o.push(']');
        field(&mut o, "compounds");
        o.push('[');
        for (i, c) in self.compounds.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            o.push('{');
            field(&mut o, "inchikey14");
            text(&mut o, &c.inchikey14);
            field(&mut o, "name");
            text(&mut o, &c.name);
            field(&mut o, "formula");
            text(&mut o, &c.formula);
            field(&mut o, "smiles");
            text(&mut o, &c.smiles);
            field(&mut o, "badge");
            text(&mut o, c.badge.as_str());
            field(&mut o, "experimental");
            o.push_str(if c.experimental { "true" } else { "false" });
            field(&mut o, "confidence");
            o.push_str("null");
            field(&mut o, "spectra");
            o.push('[');
            for (j, m) in c.spectra.iter().enumerate() {
                if j > 0 {
                    o.push(',');
                }
                spectrum(&mut o, m);
            }
            o.push_str("]}");
        }
        o.push_str("]}");
        o
    }
}

fn num(o: &mut String, v: f64) {
    match serde_json::Number::from_f64(v) {
        Some(n) => o.push_str(&n.to_string()),
        None => o.push_str("null"),
    }
}

fn text(o: &mut String, s: &str) {
    o.push_str(&serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into()));
}

fn field(o: &mut String, name: &str) {
    if !(o.ends_with("{") || o.ends_with("[")) {
        o.push(',');
    }
    let _ = write!(o, "\"{name}\":");
}

fn int(o: &mut String, name: &str, v: impl std::fmt::Display) {
    field(o, name);
    let _ = write!(o, "{v}");
}

fn flt(o: &mut String, name: &str, v: f64) {
    field(o, name);
    num(o, v);
}

fn txt(o: &mut String, name: &str, v: &str) {
    field(o, name);
    text(o, v);
}

fn pairs(o: &mut String, pairs: &[PeakPair]) {
    field(o, "pairs");
    o.push('[');
    for (i, p) in pairs.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        o.push('{');
        flt(o, "query_mz", p.query_mz);
        flt(o, "library_mz", p.library_mz);
        txt(o, "kind", p.kind.as_str());
        o.push('}');
    }
    o.push(']');
}

fn spectrum(o: &mut String, m: &SpectrumMatch) {
    o.push('{');
    int(o, "library", m.lib);
    int(o, "index", m.index);
    txt(o, "accession", &m.accession);
    flt(o, "score", m.score);
    txt(o, "score_kind", m.score_kind);
    flt(o, "reverse", m.reverse);
    flt(o, "explained", m.explained);
    int(o, "matched", m.matched);
    int(o, "of", m.of);
    int(o, "direct", m.direct);
    int(o, "shifted", m.shifted);
    flt(o, "delta_ppm", m.delta_ppm);
    flt(o, "delta_mz", m.delta_mz);
    txt(o, "adduct", &m.adduct);
    txt(o, "ce", &m.ce);
    txt(o, "instrument", &m.instrument);
    txt(o, "licence", &m.licence);
    int(o, "libraries", m.libs);
    pairs(o, &m.pairs);
    o.push('}');
}
