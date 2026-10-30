//! The "Regulatory lists" section of `corsa/1`: the lists a run screened against (header, coverage line, disclaimer) and one row per candidate that a list
//! names, from the `screen/1` (mass path) and `annot/1` (structure path) files of the run folder. The words are the study's (`docs/studi/LISTE_NORMATIVE.md` §7),
//! made here once so the CLI and the page give the same text. A match is a candidate with its evidence: no word here says a sample contains or breaks anything.

use crate::json::{write_value, Out, Value};

/// The disclaimer under the table, in the report and in the panel footer.
pub const DISCLAIMER: &str = "A match means that a candidate for this feature has the mass, formula or structure of a substance named in a regulatory list. It is not an identification, it says nothing about concentration, and it is not an assessment of compliance with any law. Lists are reproduced from the official texts named above; only the official publications are authentic. Confirm candidates with reference standards before any regulatory use.";
/// The column of the table.
pub const COLUMN: &str = "Listed in";

/// What the run folder says of the regulatory lists.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Regulatory {
    /// The manifest object of each list, each once, in the order met.
    pub lists: Vec<Value>,
    /// One JSON row per hit or annotated row that a list names.
    pub rows: Vec<String>,
}

fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::str).unwrap_or("")
}

fn tag<'a>(l: &'a Value, k: &str) -> &'a str {
    l.get("tags").map_or("", |t| text(t, k))
}

fn count(l: &Value, k: &str) -> i64 {
    l.get("coverage")
        .and_then(|c| c.get(k))
        .and_then(Value::f64)
        .unwrap_or(0.0) as i64
}

/// The name of a list in the report: its citation, else its instrument, else its id.
fn label(l: &Value) -> &str {
    [tag(l, "citation"), tag(l, "instrument"), text(l, "id")]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or("")
}

impl Regulatory {
    /// Reads one output file of the run.
    pub fn read(&mut self, v: &Value) {
        let screen = v
            .get("suspect_files")
            .map_or(&[][..], Value::arr)
            .iter()
            .filter_map(|f| f.get("lists"));
        let annot = v.get("regulatory").and_then(|r| r.get("lists"));
        for l in screen.chain(annot).flat_map(Value::arr) {
            if !self.lists.contains(l) {
                self.lists.push(l.clone());
            }
        }
        if text(v, "format") == "screen/1" {
            for h in v.get("hits").map_or(&[][..], Value::arr) {
                self.screen_row(h);
            }
        }
        if text(v, "format") == "annot/1" && v.get("regulatory").is_some() {
            for r in v.get("rows").map_or(&[][..], Value::arr) {
                self.annot_row(r);
            }
        }
    }

    /// The membership line: "<instrument>, <entry> (<the list's own text>), version of <date>".
    fn line(&self, m: &Value) -> String {
        let id = text(m, "list");
        let l = self.lists.iter().find(|l| text(l, "id") == id);
        let name = l.map_or(id, |l| {
            [tag(l, "instrument"), id]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or(id)
        });
        let mut s = format!("{name}, {}", text(m, "entry_ref"));
        if !text(m, "attr").is_empty() {
            s.push_str(&format!(" ({})", text(m, "attr")));
        }
        if let Some(d) = l.map(|l| tag(l, "version_date")).filter(|d| !d.is_empty()) {
            s.push_str(&format!(", version of {d}"));
        }
        s
    }

    fn screen_row(&mut self, h: &Value) {
        let cands: Vec<&Value> = h
            .get("candidates")
            .map_or(&[][..], Value::arr)
            .iter()
            .filter(|c| !c.get("memberships").map_or(&[][..], Value::arr).is_empty())
            .collect();
        if cands.is_empty() {
            return;
        }
        let n = cands.len();
        let cell = format!(
            "Candidate formula {} matches {n} listed substance{}",
            text(h, "formula"),
            if n == 1 { "" } else { "s" }
        );
        let mut o = Out::default();
        o.raw("{\"path\":\"mass\",\"feature\":");
        write_value(&mut o, h.get("feature").unwrap_or(&Value::Null));
        o.raw(",\"adduct\":")
            .str(text(h, "adduct"))
            .raw(",\"formula\":")
            .str(text(h, "formula"));
        o.raw(",\"delta_ppm\":");
        write_value(&mut o, h.get("delta_ppm").unwrap_or(&Value::Null));
        o.raw(",\"level\":");
        write_value(&mut o, h.get("level").unwrap_or(&Value::Null));
        o.raw(",\"listed_in\":").str(&cell).raw(",\"candidates\":[");
        for (k, c) in cands.iter().enumerate() {
            let lines: Vec<String> = c
                .get("memberships")
                .map_or(&[][..], Value::arr)
                .iter()
                .map(|m| self.line(m))
                .collect();
            self.candidate(&mut o, k, text(c, "name"), text(c, "inchikey"), &lines);
        }
        o.raw("]}");
        self.rows.push(o.0);
    }

    fn annot_row(&mut self, r: &Value) {
        let found = r.get("regulatory").map_or(&[][..], Value::arr);
        if found.is_empty() {
            return;
        }
        let full = found.iter().any(|f| text(f, "match") == "InChIKey");
        let cell = if full {
            "Candidate structure matches a listed substance (InChIKey)"
        } else {
            "Candidate structure matches a listed substance (first block: stereo, charge or isotope may differ)"
        };
        let mut o = Out::default();
        o.raw("{\"path\":\"structure\",\"feature\":");
        write_value(&mut o, r.get("id").unwrap_or(&Value::Null));
        o.raw(",\"level\":");
        write_value(&mut o, r.get("level").unwrap_or(&Value::Null));
        o.raw(",\"listed_in\":").str(cell).raw(",\"candidates\":[");
        let lines: Vec<String> = found
            .iter()
            .filter_map(|f| f.get("membership"))
            .map(|m| self.line(m))
            .collect();
        self.candidate(&mut o, 0, text(r, "name"), text(r, "entry"), &lines);
        o.raw("]}");
        self.rows.push(o.0);
    }

    fn candidate(&self, o: &mut Out, k: usize, name: &str, key: &str, lines: &[String]) {
        o.raw(if k > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(name)
            .raw(",\"inchikey\":")
            .str(key)
            .raw(",\"memberships\":[");
        for (j, l) in lines.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).str(l);
        }
        o.raw("]}");
    }

    /// "Screened against: <list> (version <date>, retrieved <date>, SHA-256 <8 hex>…); … N lists in force, M proposals."
    pub fn header(&self) -> String {
        let one = |l: &Value| {
            let mut bits = Vec::new();
            for (word, v) in [
                ("version", tag(l, "version_date")),
                ("retrieved", text(l, "retrieved")),
            ] {
                if !v.is_empty() {
                    bits.push(format!("{word} {v}"));
                }
            }
            let sha = text(l, "csv_sha256");
            if !sha.is_empty() {
                bits.push(format!("SHA-256 {}…", &sha[..sha.len().min(8)]));
            }
            format!("{} ({})", label(l), bits.join(", "))
        };
        let parts: Vec<String> = self.lists.iter().map(one).collect();
        let status = |s: &[&str]| {
            self.lists
                .iter()
                .filter(|l| s.contains(&tag(l, "status")))
                .count()
        };
        let (force, prop) = (
            status(&["in_force", "adopted_not_applicable"]),
            status(&["proposal", "draft"]),
        );
        let plural = |n: usize, w: &str| format!("{n} {w}{}", if n == 1 { "" } else { "s" });
        format!(
            "Screened against: {}. {} in force, {}.",
            parts.join("; "),
            plural(force, "list"),
            plural(prop, "proposal")
        )
    }

    /// One coverage line per list.
    pub fn coverage(&self) -> Vec<String> {
        self.lists
            .iter()
            .map(|l| {
                let mut s = format!(
                    "{}: This list has {} entries: {} screened by structure, {} groups defined by structure or as mixtures, {} inorganic",
                    label(l),
                    count(l, "entries"),
                    count(l, "screenable"),
                    count(l, "groups") + count(l, "uvcb"),
                    count(l, "inorganic")
                );
                if count(l, "unresolved") > 0 {
                    s.push_str(&format!(", {} without a structure", count(l, "unresolved")));
                }
                s.push_str("; not screened by mass.");
                s
            })
            .collect()
    }

    /// The section as `corsa/1` writes it; nothing when the run used no regulatory list.
    pub fn write(&self, o: &mut Out) {
        if self.lists.is_empty() {
            return;
        }
        o.raw(",\"regulatory\":{\"title\":\"Regulatory lists\",\"header\":")
            .str(&self.header())
            .raw(",\"lists\":[");
        for (k, l) in self.lists.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" });
            write_value(o, l);
        }
        o.raw("],\"coverage\":[");
        for (k, c) in self.coverage().iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).str(c);
        }
        o.raw("],\"column\":")
            .str(COLUMN)
            .raw(",\"rows\":[")
            .raw(&self.rows.join(","));
        o.raw("],\"disclaimer\":").str(DISCLAIMER).raw("}");
    }
}
