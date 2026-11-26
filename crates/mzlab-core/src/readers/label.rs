//! The label of a sample of a multi-sample file (`features --sample`, the Flow, the site's names of the samples of a `.wiff`): the column of a table. The sample name, never
//! the file name, with the extension of its file kept so that every table cuts that and not a dot of the name; the characters `/ \ : * ? " < > | #` become `_` and the
//! name is cut at 120 characters; members whose names collide take `NAME (FILE #ID)`. ONE pure function for the CLI, the app and the site (`sample_labels` of the engine).

use crate::error::{Error, Result};
use crate::json::{Out, Value};

/// One member of a series taken from a multi-sample file: its file name, sample id and sample name.
pub struct Member<'a> {
    pub file: &'a str,
    pub id: u32,
    pub sample: &'a str,
}

fn extension(file: &str) -> &str {
    file.rsplit_once('.')
        .filter(|(head, _)| !head.is_empty())
        .map_or("", |(_, e)| e)
}

fn base(file: &str) -> &str {
    file.rsplit_once('.')
        .filter(|(head, _)| !head.is_empty())
        .map_or(file, |(h, _)| h)
}

fn with_extension(name: &str, file: &str) -> String {
    match extension(file) {
        "" => name.to_string(),
        e => format!("{name}.{e}"),
    }
}

/// A sample name as a label: trimmed, the characters a file name cannot hold (and a path separator, which every reader of a table takes for a folder) become `_`, at
/// most 120 characters: the rule of the site for the mzML of a sample (`wiffName` of `conversione.js`; `tests/test_label_parity.py` runs both on the same names).
pub fn clean(name: &str) -> String {
    name.trim()
        .chars()
        .map(|c| if "/\\:*?\"<>|#".contains(c) { '_' } else { c })
        .take(120)
        .collect()
}

/// How a label came about: the sample name as it is, the tie-break `NAME (FILE #ID)` (two samples have the name), or a ` [N]` number (the same file name twice).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Own,
    Tie,
    Numbered,
}

/// The labels of `members` (one per member, in order) and how each came about. An empty sample name is `FILE #ID`.
pub fn labels(members: &[Member]) -> Vec<(String, Kind)> {
    let plain: Vec<String> = members
        .iter()
        .map(|m| {
            let n = clean(m.sample);
            if n.is_empty() {
                format!("{} #{}", base(m.file), m.id)
            } else {
                n
            }
        })
        .collect();
    let mut out: Vec<(String, Kind)> = members
        .iter()
        .zip(&plain)
        .map(|(m, p)| {
            let clash = plain.iter().filter(|q| q.as_str() == p).count() > 1;
            if clash {
                (
                    with_extension(&format!("{p} ({} #{})", base(m.file), m.id), m.file),
                    Kind::Tie,
                )
            } else {
                (with_extension(p, m.file), Kind::Own)
            }
        })
        .collect();
    // the same file given twice (two folders): the later ones are numbered, so no two columns share a name
    for k in 1..out.len() {
        let seen = out[..k].iter().filter(|(n, _)| *n == out[k].0).count();
        if seen > 0 {
            let n = &out[k].0;
            let name = n
                .rsplit_once('.')
                .map_or(format!("{n} [{}]", seen + 1), |(h, e)| {
                    format!("{h} [{}].{e}", seen + 1)
                });
            out[k] = (name, Kind::Numbered);
        }
    }
    out
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Kind::Own => "own",
            Kind::Tie => "tie",
            Kind::Numbered => "numbered",
        }
    }
}

/// The route `sample_labels`: `{"members":[{"file","id","sample"}]}` (in order) → `{"schema":"sample-labels/1","labels":[{"label","kind"}]}`, `kind` is `own`, `tie` or
/// `numbered` (the choice the CLI logs). The bytes the engine .wasm, the app and `mzlab-cli` give for the same members.
pub fn labels_json(input: &str) -> Result<String> {
    let v = crate::json::parse(input.as_bytes())?;
    let Some(Value::Arr(rows)) = v.get("members") else {
        return Err(Error::new("error.badSamples"));
    };
    let members: Vec<Member> = rows
        .iter()
        .map(|r| Member {
            file: r.get("file").and_then(|x| x.str()).unwrap_or(""),
            id: r.get("id").and_then(|x| x.f64()).map_or(0, |x| x as u32),
            sample: r.get("sample").and_then(|x| x.str()).unwrap_or(""),
        })
        .collect();
    let mut o = Out::default();
    o.raw("{\"schema\":\"sample-labels/1\",\"labels\":[");
    for (k, (label, kind)) in labels(&members).iter().enumerate() {
        o.raw(if k > 0 { ",{\"label\":" } else { "{\"label\":" })
            .str(label)
            .raw(",\"kind\":")
            .str(kind.word())
            .raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_route_gives_the_labels_and_how_they_came_about() {
        let got = labels_json(
            r#"{"members":[{"file":"a.wiff","id":1,"sample":"S"},{"file":"b.wiff","id":2,"sample":"S"},{"file":"a.wiff","id":3,"sample":"A/B"},{"file":"a.wiff","id":4,"sample":" "}]}"#,
        )
        .unwrap();
        assert_eq!(
            got,
            r#"{"schema":"sample-labels/1","labels":[{"label":"S (a #1).wiff","kind":"tie"},{"label":"S (b #2).wiff","kind":"tie"},{"label":"A_B.wiff","kind":"own"},{"label":"a #4.wiff","kind":"own"}]}"#
        );
        assert!(labels_json("{}").is_err());
    }
}
