//! The roles of the files of a table for the transformation-product steps: `order_like` when the samples and the table hold the same files, else the
//! pairing that never stops on the difference. A sample the table does not carry is left out and a table file with no sample gets the role its name
//! proposes; each choice comes back as a `PairNote` with its evidence, so the user can overrule it by editing the `samples/1`.

use super::order::{base, order_like, pair};
use super::positions::role_word;
use super::{from_json, role_and_minutes, to_json, Sample};
use crate::error::Result;

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
    let mut out: Vec<Sample> = Vec::with_capacity(names.len());
    for (i, &k) in pick.iter().enumerate() {
        let mut s = if k != usize::MAX {
            samples[k].clone()
        } else {
            // the role and the minutes of the name alone: the series and the replicate stay empty, so the file is shared by every condition
            let name = base(&names[i]);
            let (role, minutes) = role_and_minutes(name);
            let mut s = Sample::new(name, "", 0);
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
            s
        };
        s.order = i as u32 + 1;
        out.push(s);
    }
    Ok(Paired {
        text: to_json(&out),
        notes,
    })
}
