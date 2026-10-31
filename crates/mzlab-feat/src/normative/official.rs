//! `import_official`: the user's own copy of an official list (ECHA Candidate List, OEHHA Proposition 65; CSV) becomes a private `.mzsus` with the `listings`
//! extension, one list, labelled «imported by you on <date> from <file>, SHA-256 …». The reader is `import_csv`, the structures come from the CAS sidecar
//! only (`sidecar.rs`): a number the sidecar does not hold stays `unresolved` and is counted. Same bytes from the app route, the CLI and the `.wasm`.

use std::collections::BTreeSet;

use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{self, Out};

use super::entry::cas_tokens;
use super::import_csv;
use super::pack::{build_rows, raw_of, sorted};
use super::resolve::Resolver;
use super::rows::{build_list, Built, ListSrc};
use super::{sidecar, Error};
use crate::suspects::Suspects;

/// What the import gives back: the package bytes and what the panel shows.
pub struct Imported {
    pub mzsus: Vec<u8>,
    pub built: Built,
    /// The label of the list: file name, date and SHA-256 of the file.
    pub citation: String,
}

/// (catalogue id, jurisdiction, instrument) of a reader.
fn kind(reader: &str) -> Result<(&'static str, &'static str, &'static str), Error> {
    match reader {
        "svhc" => Ok(("SVHC", "EU", "ECHA Candidate List")),
        "ca_prop65" => Ok(("CA_PROP65", "US-CA", "California Proposition 65")),
        o => Err(Error::parse("import", &format!("no reader called {o}"))),
    }
}

/// `label` is `{"file": name, "date": "YYYY-MM-DD"}`; the date comes from the caller (the core has no clock).
pub fn import(
    reader: &str,
    csv: &[u8],
    sidecar_text: &str,
    label: &str,
) -> Result<Imported, Error> {
    let (id, jurisdiction, instrument) = kind(reader)?;
    let lab =
        json::parse(label.as_bytes()).map_err(|_| Error::parse("import", "label is not JSON"))?;
    let file = lab.get("file").and_then(json::Value::str).unwrap_or("");
    let date = lab.get("date").and_then(json::Value::str).unwrap_or("");
    let text = String::from_utf8_lossy(csv);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut entries = Vec::new();
    import_csv::entries(id, reader == "svhc", text, &mut entries)?;
    if entries.is_empty() {
        return Err(Error::parse(id, "the file gave no entries"));
    }
    let wanted: BTreeSet<String> = entries.iter().flat_map(|e| cas_tokens(&e.cas)).collect();
    let mut res = Resolver::default();
    sidecar::read(sidecar_text, &wanted, &mut res)?;
    let sha = sha256_hex(csv);
    let citation = format!("imported by you on {date} from {file}, SHA-256 {sha}");
    let tags = vec![
        ("jurisdiction".to_string(), jurisdiction.to_string()),
        ("instrument".into(), format!("{instrument} (your file)")),
        ("citation".into(), citation.clone()),
        ("status".into(), "in_force".into()),
        ("version_date".into(), date.to_string()),
    ];
    let src = ListSrc { id, tags: &tags };
    let built = build_list(&src, &entries, date, &res, true);
    let prov = provenance(&built, &tags, date);
    let val = json::parse(prov.as_bytes()).map_err(|_| Error::parse("import", "provenance"))?;
    let (rows, _) = build_rows(&[(id, built.csv.as_str())])?;
    let sorted = sorted(&rows);
    let mzsus = raw_of(&sorted, &[&val])?;
    Ok(Imported {
        mzsus,
        built,
        citation,
    })
}

/// The list object of the manifest (`normative/1`), as `normative build` writes it in `provenance.json`.
fn provenance(b: &Built, tags: &[(String, String)], retrieved: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"id\":").str(&b.id).raw(",\"tags\":{");
    for (i, (k, v)) in tags.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
    }
    let c = b.coverage;
    o.raw("},\"inputs\":[{\"retrieved\":")
        .str(retrieved)
        .raw("}],\"coverage\":{");
    for (i, (k, v)) in [
        ("entries", c.entries),
        ("screenable", c.screenable),
        ("groups", c.groups),
        ("uvcb", c.uvcb),
        ("inorganic", c.inorganic),
        ("unresolved", c.unresolved),
    ]
    .into_iter()
    .enumerate()
    {
        o.raw(if i > 0 { "," } else { "" })
            .str(k)
            .raw(":")
            .int(v as i64);
    }
    o.raw("},\"csv_sha256\":")
        .str(&sha256_hex(b.csv.as_bytes()))
        .raw(",\"stale\":false}");
    o.0
}

/// The answer of `import_official`: `u32` little-endian length of the summary JSON, the summary, then the `.mzsus`. The summary has the label, the coverage and the
/// unresolved rows (`[list, entry_ref, name, cas, why]`), so the panel can show what was not resolved.
pub fn import_official(
    reader: &str,
    csv: &[u8],
    sidecar_text: &str,
    label: &str,
) -> Result<Vec<u8>, Error> {
    let r = import(reader, csv, sidecar_text, label)?;
    let c = r.built.coverage;
    let mut o = Out::default();
    o.raw("{\"id\":")
        .str(&r.built.id)
        .raw(",\"citation\":")
        .str(&r.citation);
    o.raw(",\"entries\":").int(c.entries as i64);
    o.raw(",\"screenable\":").int(c.screenable as i64);
    o.raw(",\"unresolved\":").int(c.unresolved as i64);
    o.raw(",\"rows\":[");
    for (i, u) in r.built.unresolved.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("[");
        for (j, x) in u.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).str(x);
        }
        o.raw("]");
    }
    o.raw("]}");
    let mut out = (o.0.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(o.0.as_bytes());
    out.extend_from_slice(&r.mzsus);
    Ok(out)
}

/// The lists of an imported file as a JSON array (the `lists` of `normative/1` in its manifest, the shape of the catalogue of the package); `None` when `raw` is not a
/// readable `.mzsus`. The Library window shows them next to the installed lists, in the app and in the browser.
pub fn lists_of(raw: &[u8]) -> Option<String> {
    let s = Suspects::open(raw).ok()?;
    let m = json::parse(s.manifest().as_bytes()).ok()?;
    let l = m.get("normative/1")?.get("lists")?;
    let mut o = Out::default();
    json::write_value(&mut o, l);
    Some(o.0)
}
