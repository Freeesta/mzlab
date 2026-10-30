//! The regulatory extension of `.mzsus` (flag bit 0): per row the full InChIKey, a name and the laws naming it (layout: `docs/agenti/sospetti.md`).
//! Read by `Suspects::open`; `to_json` is the text the `.wasm`, the app and the CLI must give byte for byte.

use mzlab_core::json::Out;

use crate::suspects::{le32, u32s, Cur, Hit, Suspects, SuspectsError};

/// Header flag bit 0: the file carries the `listings` extension after the formula blob.
pub const FLAG_LISTINGS: u32 = 1;

/// `Membership::form`: the form of the compound a law names (0 = not stated).
pub const FORM_NAMES: [&str; 5] = ["", "as_listed", "parent", "salt", "anion"];
/// `Membership::bits`.
pub const BIT_GROUP: u8 = 1;
pub const BIT_ISOMER: u8 = 2;
pub const BIT_ANION_CAS: u8 = 4;

/// One law naming a row: the list (index into the manifest's `normative/1.lists`), the form, the flags and the entry (index into `Listings::entry`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Membership {
    pub list: u16,
    pub form: u8,
    pub bits: u8,
    pub entry: u32,
}

/// The regulatory extension: per row the full InChIKey, a display name and its memberships; the entries are compact JSON objects.
#[derive(Debug, Default)]
pub struct Listings {
    lists: usize,
    keys: Vec<u8>,
    names: Vec<String>,
    member_offs: Vec<u32>,
    members: Vec<Membership>,
    entries: Vec<String>,
}

impl Listings {
    /// The whole extension as one JSON text (rows in order, each with its memberships): the text the `.wasm` and the native reader must give byte for byte.
    pub fn to_json(&self, rows: &[Hit]) -> String {
        listings_text(self, rows)
    }

    /// Number of lists the memberships index (the manifest names them).
    pub fn list_count(&self) -> usize {
        self.lists
    }

    /// Full InChIKey of the representative listed form of `row` (empty when the row has none).
    pub fn full_key(&self, row: usize) -> &str {
        std::str::from_utf8(&self.keys[27 * row..27 * row + 27])
            .unwrap_or("")
            .trim_end()
    }

    pub fn name(&self, row: usize) -> &str {
        &self.names[row]
    }

    pub fn memberships(&self, row: usize) -> &[Membership] {
        &self.members[self.member_offs[row] as usize..self.member_offs[row + 1] as usize]
    }

    /// The compact JSON object of an entry (`ref`, `name`, `cas`, `ec`, `attr`).
    pub fn entry(&self, i: u32) -> &str {
        &self.entries[i as usize]
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Suspects {
    /// `Listings::to_json` over the rows of this file; `None` without the extension.
    pub fn listings_json(&self) -> Option<String> {
        self.listings.as_ref().map(|l| l.to_json(&self.rows))
    }
}

/// Strings of a `count+1` offsets table and its blob.
fn strings(c: &mut Cur, count: usize) -> Result<Vec<String>, SuspectsError> {
    let offs: Vec<u32> = u32s(c.take(4 * (count + 1))?).collect();
    let blob = c.take(offs[count] as usize)?;
    (0..count)
        .map(|i| {
            let (a, b) = (offs[i] as usize, offs[i + 1] as usize);
            if a > b || b > blob.len() {
                return Err(SuspectsError::Corrupt("listings offsets"));
            }
            std::str::from_utf8(&blob[a..b])
                .map(str::to_owned)
                .map_err(|_| SuspectsError::Corrupt("listings utf-8"))
        })
        .collect()
}

pub(crate) fn parse_listings(c: &mut Cur, n: usize) -> Result<Listings, SuspectsError> {
    let head = c.take(16)?;
    let (nl, m, ne) = (
        le32(head, 0) as usize,
        le32(head, 4) as usize,
        le32(head, 8) as usize,
    );
    let keys = c.take(27 * n)?.to_vec();
    let names = strings(c, n)?;
    let member_offs: Vec<u32> = u32s(c.take(4 * (n + 1))?).collect();
    if member_offs.windows(2).any(|w| w[0] > w[1]) || member_offs[n] as usize != m {
        return Err(SuspectsError::Corrupt("membership offsets"));
    }
    let members: Vec<Membership> = c
        .take(8 * m)?
        .chunks_exact(8)
        .map(|r| Membership {
            list: u16::from_le_bytes([r[0], r[1]]),
            form: r[2],
            bits: r[3],
            entry: le32(r, 4),
        })
        .collect();
    let entries = strings(c, ne)?;
    if members
        .iter()
        .any(|x| x.entry as usize >= ne || x.list as usize >= nl)
    {
        return Err(SuspectsError::Corrupt("membership index"));
    }
    Ok(Listings {
        lists: nl,
        keys,
        names,
        member_offs,
        members,
        entries,
    })
}

fn memberships_json(l: &Listings, o: &mut Out, row: usize) {
    for (j, m) in l.memberships(row).iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" })
            .raw("{\"list\":")
            .int(m.list as i64)
            .raw(",\"form\":")
            .str(FORM_NAMES.get(m.form as usize).copied().unwrap_or(""))
            .raw(",\"bits\":")
            .int(m.bits as i64)
            .raw(",\"entry\":")
            .raw(l.entry(m.entry))
            .raw("}");
    }
}

/// The whole extension as one JSON text (rows in order, each with its memberships): the text the `.wasm` and the native reader must give byte for byte.
fn listings_text(l: &Listings, rows: &[Hit]) -> String {
    let mut o = Out::default();
    o.raw("{\"lists\":").int(l.lists as i64).raw(",\"rows\":[");
    for (i, h) in rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"key\":")
            .str(std::str::from_utf8(&h.key).unwrap_or(""))
            .raw(",\"full_key\":")
            .str(l.full_key(i))
            .raw(",\"name\":")
            .str(l.name(i))
            .raw(",\"in\":[");
        memberships_json(l, &mut o, i);
        o.raw("]}");
    }
    o.raw("]}");
    o.0
}
