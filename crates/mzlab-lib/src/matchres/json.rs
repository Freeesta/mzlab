//! `libmatch/1` as JSON text with a fixed field order (docs/schemi/libmatch-1.json).

use super::*;
use std::fmt::Write;

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
    if !o.ends_with(['{', '[']) {
        o.push(',');
    }
    let _ = write!(o, "\"{name}\":");
}

fn spectrum(o: &mut String, m: &SpectrumMatch) {
    o.push('{');
    field(o, "library");
    let _ = write!(o, "{}", m.lib);
    field(o, "index");
    let _ = write!(o, "{}", m.index);
    field(o, "accession");
    text(o, &m.accession);
    field(o, "score");
    num(o, m.score);
    field(o, "score_kind");
    text(o, m.score_kind);
    field(o, "reverse");
    num(o, m.reverse);
    field(o, "explained");
    num(o, m.explained);
    field(o, "matched");
    let _ = write!(o, "{}", m.matched);
    field(o, "of");
    let _ = write!(o, "{}", m.of);
    field(o, "direct");
    let _ = write!(o, "{}", m.direct);
    field(o, "shifted");
    let _ = write!(o, "{}", m.shifted);
    field(o, "delta_ppm");
    num(o, m.delta_ppm);
    field(o, "delta_mz");
    num(o, m.delta_mz);
    field(o, "adduct");
    text(o, &m.adduct);
    field(o, "ce");
    text(o, &m.ce);
    field(o, "instrument");
    text(o, &m.instrument);
    field(o, "licence");
    text(o, &m.licence);
    field(o, "libraries");
    let _ = write!(o, "{}", m.libs);
    field(o, "pairs");
    o.push('[');
    for (i, p) in m.pairs.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        o.push('{');
        field(o, "query_mz");
        num(o, p.query_mz);
        field(o, "library_mz");
        num(o, p.library_mz);
        field(o, "kind");
        text(o, p.kind.as_str());
        o.push('}');
    }
    o.push_str("]}");
}

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
