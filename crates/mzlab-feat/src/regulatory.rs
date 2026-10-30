//! The regulatory package as the matching steps read it: the lists of a `.mzsus` with the `listings` extension (manifest `normative/1`), the mask the user
//! chose (`lists`, `include`) and the memberships of a row that pass it. `screen.suspects` (mass path) and `annot.regulatory` (structure path) share it, so
//! both write the same list provenance. Lists are data: nothing here knows a law.

use crate::suspects::Suspects;
use crate::suspects_listings::{BIT_GROUP, FORM_NAMES};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out, Value};

/// The statuses screened by default: the lists in force and the ones adopted but not yet applicable.
pub const DEFAULT_STATUS: [&str; 2] = ["in_force", "adopted_not_applicable"];
/// The statuses `include` may add.
pub const INCLUDE: [&str; 3] = ["proposal", "draft", "reference"];

/// One list of the package: its id, its status and its manifest object as the package wrote it.
pub struct ListInfo {
    pub id: String,
    pub status: String,
    pub manifest: Value,
}

impl ListInfo {
    fn of(v: Value) -> ListInfo {
        let text = |k: &str| v.get(k).and_then(Value::str).unwrap_or("").to_string();
        let status = v.get("tags").map_or(String::new(), |t| {
            t.get("status")
                .and_then(Value::str)
                .unwrap_or("")
                .to_string()
        });
        ListInfo {
            id: text("id"),
            status,
            manifest: v,
        }
    }
}

/// One law naming a row, after the mask.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub list: String,
    pub entry_ref: String,
    pub form: String,
    pub group: bool,
    /// The law's own text for the entry (a limit, a note), quoted, never compared with a measured value.
    pub attr: String,
}

impl Member {
    fn of(list: &str, entry: &str, form: u8, bits: u8) -> Member {
        let e = json::parse(entry.as_bytes()).unwrap_or(Value::Null);
        let text = |k: &str| e.get(k).and_then(Value::str).unwrap_or("").to_string();
        Member {
            list: list.to_string(),
            entry_ref: text("ref"),
            form: FORM_NAMES
                .get(usize::from(form))
                .copied()
                .unwrap_or("")
                .to_string(),
            group: bits & BIT_GROUP != 0,
            attr: text("attr"),
        }
    }
}

/// Which lists pass the mask, in the order of the package; an unknown id or status is an error.
fn mask(infos: &[ListInfo], lists: &str, include: &str) -> Result<Vec<bool>> {
    let wanted = words(lists);
    let extra = words(include);
    if let Some(bad) = extra.iter().find(|w| !INCLUDE.contains(&w.as_str())) {
        return Err(Error::new("err.flow.regulatory.include").with("status", bad));
    }
    if let Some(bad) = wanted.iter().find(|w| !infos.iter().any(|i| &i.id == *w)) {
        return Err(Error::new("err.flow.regulatory.list").with("list", bad));
    }
    Ok(infos
        .iter()
        .map(|i| {
            let status = DEFAULT_STATUS.contains(&i.status.as_str()) || extra.contains(&i.status);
            status && (wanted.is_empty() || wanted.contains(&i.id))
        })
        .collect())
}

/// The lists of a package and the ones the mask keeps.
pub struct Package {
    pub lists: Vec<ListInfo>,
    pub on: Vec<bool>,
}

impl Package {
    /// The lists of `s` (`None` without the extension) with the mask: `lists` = ids (empty: every list), `include` = statuses added to the default ones.
    pub fn open(s: &Suspects, lists: &str, include: &str) -> Result<Option<Package>> {
        let Some(l) = s.listings() else {
            return Ok(None);
        };
        let man = json::parse(s.manifest().as_bytes()).unwrap_or(Value::Null);
        let given = man
            .get("normative/1")
            .and_then(|n| n.get("lists"))
            .map(Value::arr)
            .unwrap_or(&[]);
        let infos: Vec<ListInfo> = (0..l.list_count())
            .map(|i| ListInfo::of(given.get(i).cloned().unwrap_or(Value::Null)))
            .collect();
        let on = mask(&infos, lists, include)?;
        Ok(Some(Package { lists: infos, on }))
    }

    /// The memberships of `row` whose list passes the mask, in the order of the package.
    pub fn members(&self, s: &Suspects, row: usize) -> Vec<Member> {
        let Some(l) = s.listings() else {
            return Vec::new();
        };
        l.memberships(row)
            .iter()
            .filter(|m| self.on.get(usize::from(m.list)).copied().unwrap_or(false))
            .map(|m| {
                Member::of(
                    &self.lists[usize::from(m.list)].id,
                    l.entry(m.entry),
                    m.form,
                    m.bits,
                )
            })
            .collect()
    }

    /// The ids the mask keeps, in the order of the package.
    pub fn used(&self) -> Vec<&str> {
        self.lists
            .iter()
            .zip(&self.on)
            .filter(|(_, on)| **on)
            .map(|(l, _)| l.id.as_str())
            .collect()
    }

    /// The lists the mask keeps as their manifest objects (tags with the version date, coverage, `csv_sha256`): the provenance of the output.
    pub fn write_lists(&self, o: &mut Out) {
        o.raw("[");
        let mut first = true;
        for (l, _) in self.lists.iter().zip(&self.on).filter(|(_, on)| **on) {
            o.raw(if first { "" } else { "," });
            first = false;
            json::write_value(o, &l.manifest);
        }
        o.raw("]");
    }
}

/// A membership as `screen/1` and `annot/1` write it.
pub fn write_member(o: &mut Out, m: &Member) {
    o.raw("{\"list\":")
        .str(&m.list)
        .raw(",\"entry_ref\":")
        .str(&m.entry_ref)
        .raw(",\"form\":")
        .str(&m.form)
        .raw(",\"group\":")
        .raw(if m.group { "true" } else { "false" })
        .raw(",\"attr\":")
        .str(&m.attr)
        .raw("}");
}

/// The words of `text` as a JSON array.
pub fn write_words(o: &mut Out, text: &str) {
    o.raw("[");
    for (i, w) in words(text).iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(w);
    }
    o.raw("]");
}

fn words(text: &str) -> Vec<String> {
    text.split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}
