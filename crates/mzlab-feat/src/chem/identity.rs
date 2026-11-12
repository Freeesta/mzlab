//! `molid/1`: is a drawing in the molecule catalogue? The drawing (molfile or SMILES) becomes a `mol/1` record; its `lookup_key()` (the `can()`
//! blind to the Kekulé form written) is looked up among the catalogue structures (exact matches), and its skeleton too (near matches: same
//! constitution, other stereo or isotopes, said which).
//! It names the drawing, never the data. The keys of a catalogue text are kept once per text, so a second lookup reads no SMILES.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out};
use mzlab_core::molecules::{from_csv, Molecule};
use mzlab_mol::Mol;
use std::cell::RefCell;

pub const FORMAT: &str = "molid/1";

/// The two identities read for every catalogue structure: `lookup_key()` and the skeleton; the record stays for the rarer near matches.
struct Entry {
    mol: Molecule,
    rec: Mol,
    key: String,
    skel: String,
}

/// The catalogue last read: its text fingerprint and the entries with a readable structure.
struct Cache {
    print: (usize, u64),
    entries: Vec<Entry>,
}

thread_local! {
    static CACHE: RefCell<Option<Cache>> = const { RefCell::new(None) };
}

/// FNV-1a of the text with its length: tells one catalogue text from another, nothing more (not a checksum anyone stores).
fn fingerprint(t: &str) -> (usize, u64) {
    let h = t.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
    });
    (t.len(), h)
}

fn load(csv: &str) -> Result<Vec<Entry>> {
    let rows = from_csv(csv, "catalogo", None).map_err(|e| {
        Error::new("error.molid.catalogue").with("row", e.first().map_or(0, |r| r.line as i64))
    })?;
    Ok(rows
        .into_iter()
        .filter_map(|m| {
            let rec = Mol::from_smiles(m.smiles.as_deref()?).ok()?;
            Some(Entry {
                key: rec.lookup_key(),
                skel: rec.skeleton(),
                rec,
                mol: m,
            })
        })
        .collect())
}

/// The record of the drawing: a molfile (V2000 or V3000) when given, else a SMILES; no atoms is an error, not «nothing found».
fn drawing(molfile: Option<&str>, smiles: Option<&str>) -> Result<Mol> {
    let r = match (
        molfile.filter(|t| !t.trim().is_empty()),
        smiles.filter(|t| !t.trim().is_empty()),
    ) {
        (Some(t), _) => Mol::from_molfile(t),
        (None, Some(t)) => Mol::from_smiles(t.trim()),
        (None, None) => return Err(Error::new("error.molid.empty")),
    };
    let m = r.map_err(|e| Error::new("error.molid.read").with("detail", e.to_string()))?;
    if m.atoms.is_empty() {
        return Err(Error::new("error.molid.empty"));
    }
    Ok(m)
}

fn entry(j: &mut Out, m: &Molecule) {
    j.raw("{\"id\":").str(&m.id).raw(",\"nome\":").str(&m.nome);
    j.raw(",\"formula\":")
        .str(&m.formula)
        .raw(",\"carica\":")
        .int(m.load as i64);
    j.raw(",\"massa\":")
        .num(m.massa)
        .raw(",\"smiles\":")
        .str(m.smiles.as_deref().unwrap_or(""));
}

/// What a near match differs in: isotopes only when the isotope-free keys agree, stereo only when the stereo-free ones do, else both.
fn differs(a: &Mol, b: &Mol) -> &'static str {
    if a.without_isotopes().lookup_key() == b.without_isotopes().lookup_key() {
        "[\"isotopes\"]"
    } else if a.without_stereo().lookup_key() == b.without_stereo().lookup_key() {
        "[\"stereo\"]"
    } else {
        "[\"stereo\",\"isotopes\"]"
    }
}

/// `{"molfile"|"smiles", "catalogo": csv}` → `{"format":"molid/1","key","skeleton","compared","exact":[..],"near":[..]}`; entries in name order,
/// each `{id, nome, formula, carica, massa, smiles}`, a near one with `differs` (`stereo`, `isotopes`). Same bytes as `mzlab-cli mol-id`.
pub fn identity_json(request: &str) -> Result<String> {
    let v = parse(request.as_bytes()).map_err(|_| Error::new("error.molid.request"))?;
    let s = |k: &str| v.get(k).and_then(|x| x.str());
    let me = drawing(s("molfile"), s("smiles"))?;
    let (key, skel) = (me.lookup_key(), me.skeleton());
    let csv = s("catalogo").unwrap_or("");
    let print = fingerprint(csv);
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.as_ref().is_none_or(|x| x.print != print) {
            *c = Some(Cache {
                print,
                entries: load(csv)?,
            });
        }
        let entries = &c.as_ref().map_or(&[][..], |x| &x.entries[..]);
        let mut exact: Vec<&Entry> = entries.iter().filter(|e| e.key == key).collect();
        let mut near: Vec<&Entry> = entries
            .iter()
            .filter(|e| e.key != key && e.skel == skel)
            .collect();
        let by_name = |a: &&Entry, b: &&Entry| {
            (a.mol.nome.as_str(), a.mol.id.as_str()).cmp(&(b.mol.nome.as_str(), b.mol.id.as_str()))
        };
        crate::sortc::by(&mut exact, by_name);
        crate::sortc::by(&mut near, by_name);
        let mut j = Out::default();
        j.raw("{\"format\":")
            .str(FORMAT)
            .raw(",\"key\":")
            .str(&key)
            .raw(",\"skeleton\":")
            .str(&skel);
        j.raw(",\"compared\":")
            .int(entries.len() as i64)
            .raw(",\"exact\":[");
        for (i, e) in exact.iter().enumerate() {
            j.raw(if i > 0 { "," } else { "" });
            entry(&mut j, &e.mol);
            j.raw("}");
        }
        j.raw("],\"near\":[");
        for (i, e) in near.iter().enumerate() {
            j.raw(if i > 0 { "," } else { "" });
            entry(&mut j, &e.mol);
            j.raw(",\"differs\":").raw(differs(&me, &e.rec)).raw("}");
        }
        j.raw("]}");
        Ok(j.0)
    })
}
