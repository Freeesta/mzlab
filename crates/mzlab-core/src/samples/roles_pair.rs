//! The roles of the files of a table for the transformation-product steps: `order_like` when the samples and the table hold the same files, else the
//! pairing that never stops on the difference. A sample the table does not carry is left out and a table file with no sample gets the role its name
//! proposes; each choice comes back as a `PairNote` with its evidence (and as the `pairing` array of the result, `notes_json`), so the user can overrule it
//! by editing the `samples/1`. The document is edited as it was written: the fields this module does not know stay.

use super::order::{base, order_like, pair};
use super::positions::role_word;
use super::{from_json, role_and_minutes, to_json, Sample};
use crate::error::Result;
use crate::json::{parse, write_value, Out, Value};

/// One automatic choice of `order_for_roles`.
#[derive(Debug, Clone, PartialEq)]
pub struct PairNote {
    /// `left_out` (a sample whose file is not in the table) or `proposed` (a table file with no sample: the role comes from its name).
    pub action: &'static str,
    pub file: String,
    pub role: String,
    /// `proposed`: the part of the name that decided the role ("" when the name has no role word; the minutes for a time zero); empty for `left_out`.
    pub evidence: String,
}

/// The samples text in the order of the table, and what was chosen on the way.
pub struct Paired {
    pub text: String,
    pub notes: Vec<PairNote>,
}

/// `order_like` for the steps that read the roles of a table. Equal sets of files give its text unchanged and no note.
pub fn order_for_roles(text: &str, names: &[String]) -> Result<Paired> {
    if let Ok(t) = order_like(text, names) {
        return Ok(Paired {
            text: t,
            notes: Vec::new(),
        });
    }
    let samples = from_json(text.as_bytes())?;
    let mut doc = parse(text.as_bytes())?;
    let Value::Obj(fields) = &mut doc else {
        return order_like(text, names).map(|text| Paired {
            text,
            notes: Vec::new(),
        });
    };
    let Some((_, Value::Arr(rows))) = fields.iter_mut().find(|(k, _)| k == "samples") else {
        return order_like(text, names).map(|text| Paired {
            text,
            notes: Vec::new(),
        });
    };
    let files: Vec<String> = samples.iter().map(|s| base(&s.file).to_string()).collect();
    let (pick, used) = pair(&files, names);
    let mut notes = Vec::new();
    for (k, s) in samples.iter().enumerate().filter(|(k, _)| !used[*k]) {
        notes.push(PairNote {
            action: "left_out",
            file: files[k].clone(),
            role: s.role().into(),
            evidence: String::new(),
        });
    }
    let mut old = std::mem::take(rows);
    for (i, &k) in pick.iter().enumerate() {
        if k != usize::MAX {
            rows.push(std::mem::replace(&mut old[k], Value::Null));
            continue;
        }
        // the role and the minutes of the name alone: the series and the replicate stay empty, so the file is shared by every condition
        let name = base(&names[i]);
        let (role, minutes) = role_and_minutes(name);
        let mut s = Sample::new(name, "", i as u32 + 1);
        s.kind = role.into();
        s.set_time(minutes, "min");
        // a time zero has no word: the minutes decided it
        let evidence = if role == "t0" {
            minutes.map(|m| m.to_string()).unwrap_or_default()
        } else {
            role_word(name, role)
        };
        notes.push(PairNote {
            action: "proposed",
            file: name.to_string(),
            role: role.into(),
            evidence,
        });
        // the row as `to_json` writes it
        let one = parse(to_json(&[s]).as_bytes())?;
        rows.push(
            one.get("samples")
                .and_then(|a| a.arr().first())
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    let mut o = Out::default();
    write_value(&mut o, &doc);
    Ok(Paired { text: o.0, notes })
}

/// The `pairing` array of a result: `[{"action","file","role","evidence"}]`, in the order of the notes; "" when there is no note.
pub fn notes_json(notes: &[PairNote]) -> String {
    if notes.is_empty() {
        return String::new();
    }
    let mut o = Out::default();
    o.raw("[");
    for (i, n) in notes.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"action\":")
            .str(n.action)
            .raw(",\"file\":")
            .str(&n.file)
            .raw(",\"role\":")
            .str(&n.role)
            .raw(",\"evidence\":")
            .str(&n.evidence)
            .raw("}");
    }
    o.raw("]");
    o.0
}

/// `json` (an object) with the key `pairing` added at its end; the same text when there is no note, so a result without choices keeps its bytes.
pub fn with_pairing(json: &str, notes: &[PairNote]) -> String {
    let (pairing, body) = (notes_json(notes), json.trim_end());
    match body.strip_suffix('}') {
        Some(head) if !pairing.is_empty() => format!("{head},\"pairing\":{pairing}}}"),
        _ => json.to_string(),
    }
}
