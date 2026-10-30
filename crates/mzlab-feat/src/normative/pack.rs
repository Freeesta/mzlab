//! The writer of the regulatory-lists package: the A1+ CSV of each list becomes one row per skeleton of the `listings` extension of `mzlab_feat::suspects`
//! (`write_mzsus`), with its manifest (`manifest_json`) and the `normative/1` catalogue (`catalog_json`). No clock, no disk: same input, same bytes. Used by
//! `mzlab-cli normative pack` and by `official::import`. Layout: `docs/agenti/sospetti.md`.

use std::collections::BTreeMap;

use crate::sortc::SortC;
use crate::suspects::{Suspects, BIT_ANION_CAS, FLAG_LISTINGS, FORM_NAMES, SRC_NORMATIVE};
use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{self, Out, Value};

use super::csvrd::Records;
use super::Error;

pub const FORMAT: &str = "normative/1";
pub const PIECE: usize = 4 << 20;
pub const RAW_NAME: &str = "normative.mzsus";
pub const ATTRIBUTION: &str = "Regulatory lists built from the official publications (EU Official Journal, US eCFR and EPA, EU Pesticides Database, NORMAN SusDat); \
only the official publications are authentic. Per-list licences and attributions are in catalogo.json.";

pub struct Member {
    list: u16,
    form: u8,
    bits: u8,
    entry_ref: String,
    entry: String,
    full_key: String,
    name: String,
}

pub struct Row {
    mass: f64,
    formula: String,
    members: Vec<Member>,
}

fn pad8(v: &mut Vec<u8>) {
    while !v.len().is_multiple_of(8) {
        v.push(0);
    }
}

fn section(v: &mut Vec<u8>, bytes: &[u8]) {
    v.extend_from_slice(bytes);
    pad8(v);
}

fn u32s(xs: &[u32]) -> Vec<u8> {
    xs.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// An offsets table (`count+1` u32) followed by the blob, each section padded to 8.
fn strings(v: &mut Vec<u8>, items: &[&str]) {
    let mut offs = vec![0u32];
    let mut blob = Vec::new();
    for s in items {
        blob.extend_from_slice(s.as_bytes());
        offs.push(blob.len() as u32);
    }
    section(v, &u32s(&offs));
    section(v, &blob);
}

fn entry_json(c: &[String]) -> String {
    let mut o = Out::default();
    for (i, (k, v)) in ["ref", "name", "cas", "ec", "attr"]
        .iter()
        .zip(c)
        .enumerate()
    {
        o.raw(if i > 0 { "," } else { "{" }).str(k).raw(":").str(v);
    }
    o.raw("}");
    o.0
}

fn valid_key14(k: &str) -> bool {
    k.len() == 14 && k.bytes().all(|b| b.is_ascii_uppercase())
}

/// The screenable rows of one list, grouped by skeleton into `rows`; returns how many screenable rows had no mass or formula (not packed).
fn read_list(
    text: &str,
    list: u16,
    id: &str,
    rows: &mut BTreeMap<String, Row>,
) -> Result<usize, Error> {
    let mut skipped = 0;
    let mut rec = Records::new(text.as_bytes(), b';');
    let head = rec.next().ok_or_else(|| Error::parse(id, "empty csv"))?;
    let col = |n: &str| head.iter().position(|h| h == n);
    let need = |n: &str| col(n).ok_or_else(|| Error::parse(id, &format!("column {n} missing")));
    let (c_key, c_full, c_name, c_cas, c_form, c_mass, c_attr) = (
        need("inchikey_skeleton")?,
        need("inchikey")?,
        need("name")?,
        need("cas")?,
        need("formula")?,
        need("mono_mass")?,
        need("list_attribute")?,
    );
    let (c_ref, c_ec, c_how, c_status) = (
        need("entry_ref")?,
        need("ec")?,
        need("form")?,
        need("status")?,
    );
    for r in rec {
        let g = |i: usize| r.get(i).map(String::as_str).unwrap_or("");
        let (key, mass) = (g(c_key), g(c_mass).parse::<f64>());
        if g(c_status) != "screenable" {
            continue;
        }
        let Ok(mass) = mass else {
            skipped += 1;
            continue;
        };
        if !valid_key14(key) || g(c_form).is_empty() {
            skipped += 1;
            continue;
        }
        let form = FORM_NAMES.iter().position(|f| *f == g(c_how)).unwrap_or(0) as u8;
        let bits = if matches!(g(c_how), "anion" | "salt") {
            BIT_ANION_CAS
        } else {
            0
        };
        let row = rows.entry(key.to_string()).or_insert_with(|| Row {
            mass,
            formula: g(c_form).to_string(),
            members: Vec::new(),
        });
        row.members.push(Member {
            list,
            form,
            bits,
            entry_ref: g(c_ref).to_string(),
            entry: entry_json(&[
                g(c_ref).into(),
                g(c_name).into(),
                g(c_cas).into(),
                g(c_ec).into(),
                g(c_attr).into(),
            ]),
            full_key: g(c_full).to_string(),
            name: g(c_name).to_string(),
        });
    }
    Ok(skipped)
}

/// The raw `.mzsus` of the rows, with `manifest` as its manifest JSON.
/// The distinct entries (in first-seen order), the per-row membership offsets and the memberships `(list, form, bits, entry)`.
/// `(list, form, bits, entry)` of one membership.
type Packed = (u16, u8, u8, u32);

fn intern(rows: &[(&String, &Row)]) -> (Vec<String>, Vec<u32>, Vec<Packed>) {
    let mut entries: Vec<String> = Vec::new();
    let mut entry_ix: BTreeMap<String, u32> = BTreeMap::new();
    let (mut member_offs, mut members) = (vec![0u32], Vec::new());
    for (_, r) in rows {
        for m in &r.members {
            let ix = *entry_ix.entry(m.entry.clone()).or_insert_with(|| {
                entries.push(m.entry.clone());
                entries.len() as u32 - 1
            });
            members.push((m.list, m.form, m.bits, ix));
        }
        member_offs.push(members.len() as u32);
    }
    (entries, member_offs, members)
}

/// The `listings` extension after the formula blob (layout: `docs/agenti/sospetti.md`).
fn write_listings(
    v: &mut Vec<u8>,
    rows: &[(&String, &Row)],
    n_lists: usize,
    entries: &[String],
    member_offs: &[u32],
    members: &[Packed],
) {
    let blob: usize = entries.iter().map(String::len).sum();
    section(
        v,
        &u32s(&[
            n_lists as u32,
            members.len() as u32,
            entries.len() as u32,
            blob as u32,
        ]),
    );
    let full: Vec<u8> = rows
        .iter()
        .flat_map(|(_, r)| {
            let k = r.members.first().map_or("", |m| m.full_key.as_str());
            let mut b = if k.len() == 27 {
                k.bytes().collect()
            } else {
                Vec::new()
            };
            b.resize(27, b' ');
            b
        })
        .collect();
    section(v, &full);
    let names: Vec<&str> = rows
        .iter()
        .map(|(_, r)| r.members.first().map_or("", |m| m.name.as_str()))
        .collect();
    strings(v, &names);
    section(v, &u32s(member_offs));
    let mb: Vec<u8> = members
        .iter()
        .flat_map(|&(l, f, b, e)| {
            let mut x = l.to_le_bytes().to_vec();
            x.extend_from_slice(&[f, b]);
            x.extend_from_slice(&e.to_le_bytes());
            x
        })
        .collect();
    section(v, &mb);
    let refs: Vec<&str> = entries.iter().map(String::as_str).collect();
    strings(v, &refs);
}

fn write_mzsus(rows: &[(&String, &Row)], n_lists: usize, manifest: &str) -> Vec<u8> {
    let mut formulas: Vec<&str> = rows.iter().map(|(_, r)| r.formula.as_str()).collect();
    formulas.sortc();
    formulas.dedup();
    let (entries, member_offs, members) = intern(rows);
    let n = rows.len();
    let mut v = Vec::new();
    v.extend_from_slice(b"MZSU");
    for x in [1u16, 1] {
        v.extend_from_slice(&x.to_le_bytes());
    }
    let head = [
        n as u32,
        formulas.len() as u32,
        0,
        manifest.len() as u32,
        FLAG_LISTINGS,
        0,
    ];
    v.extend_from_slice(&u32s(&head));
    section(&mut v, manifest.as_bytes());
    let masses: Vec<u8> = rows
        .iter()
        .flat_map(|(_, r)| r.mass.to_le_bytes())
        .collect();
    section(&mut v, &masses);
    let keys: Vec<u8> = rows.iter().flat_map(|(k, _)| k.bytes()).collect();
    section(&mut v, &keys);
    let fidx: Vec<u32> = rows
        .iter()
        .map(|(_, r)| formulas.binary_search(&r.formula.as_str()).unwrap_or(0) as u32)
        .collect();
    section(&mut v, &u32s(&fidx));
    section(&mut v, &vec![SRC_NORMATIVE; n]);
    section(&mut v, &vec![0u8; 2 * n]);
    section(&mut v, &vec![0u8; 2 * n]);
    section(&mut v, &[]);
    strings(&mut v, &formulas);
    write_listings(&mut v, rows, n_lists, &entries, &member_offs, &members);
    v
}

fn lists_json(o: &mut Out, lists: &[&Value]) {
    o.raw("[");
    for (i, l) in lists.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"id\":");
        let id = l.get("id").and_then(Value::str).unwrap_or("");
        o.str(id);
        for k in ["tags", "coverage", "csv_sha256", "stale"] {
            if let Some(v) = l.get(k) {
                o.raw(",").str(k).raw(":");
                json::write_value(o, v);
            }
        }
        // the latest retrieval date of the inputs: the report quotes it beside the version date
        let retrieved = l
            .get("inputs")
            .map_or(&[][..], Value::arr)
            .iter()
            .filter_map(|i| i.get("retrieved").and_then(Value::str))
            .max();
        if let Some(r) = retrieved {
            o.raw(",\"retrieved\":").str(r);
        }
        o.raw("}");
    }
    o.raw("]");
}

/// Every screenable row of the lists (`(id, csv text)` in the order of the package), memberships ordered and deduplicated; the count of rows without mass or formula that are left out.
pub fn build_rows(lists: &[(&str, &str)]) -> Result<(BTreeMap<String, Row>, usize), Error> {
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    let mut skipped = 0;
    for (i, (id, text)) in lists.iter().enumerate() {
        skipped += read_list(text, i as u16, id, &mut rows)?;
    }
    for r in rows.values_mut() {
        r.members
            .sortc_by(|a, b| (a.list, &a.entry_ref, a.form).cmp(&(b.list, &b.entry_ref, b.form)));
        r.members
            .dedup_by(|a, b| (a.list, a.form, &a.entry) == (b.list, b.form, &b.entry));
    }
    Ok((rows, skipped))
}

/// The rows sorted by mass and key, as they are written.
pub fn sorted(rows: &BTreeMap<String, Row>) -> Vec<(&String, &Row)> {
    let mut v: Vec<(&String, &Row)> = rows.iter().collect();
    v.sortc_by(|a, b| a.1.mass.total_cmp(&b.1.mass).then_with(|| a.0.cmp(b.0)));
    v
}

/// The raw `.mzsus` of the lists (checked by reading it back) and the rows written.
pub fn raw_of(rows: &[(&String, &Row)], lists: &[&Value]) -> Result<Vec<u8>, Error> {
    let man = manifest_json(rows.len(), lists);
    let raw = write_mzsus(rows, lists.len(), &man);
    Suspects::open(&raw).map_err(|e| Error::parse("pack", &format!("{e:?}")))?;
    Ok(raw)
}

pub fn manifest_json(rows: usize, lists: &[&Value]) -> String {
    let mut man = Out::default();
    man.raw("{\"formato\":\"mzsus/1\",\"righe\":")
        .int(rows as i64)
        .raw(",\"attribuzione\":")
        .str(ATTRIBUTION)
        .raw(",\"fonti\":[{\"id\":\"normative\",\"bit\":")
        .int(SRC_NORMATIVE as i64)
        .raw("}],\"normative/1\":{\"lists\":");
    lists_json(&mut man, lists);
    man.raw("}}");
    man.0
}

pub fn catalog_json(
    raw: &[u8],
    z_len: usize,
    rows: usize,
    skipped: usize,
    lists: &[&Value],
    pieces: &[(String, usize, String)],
) -> String {
    let mut attrib: Vec<String> = Vec::new();
    for l in lists {
        let a = l
            .get("tags")
            .and_then(|t| t.get("attribution"))
            .and_then(Value::str);
        if let Some(a) = a.filter(|a| !a.is_empty() && !attrib.iter().any(|x| x == a)) {
            attrib.push(a.to_string());
        }
    }
    let mut cat = Out::default();
    cat.raw("{\"format\":")
        .str(FORMAT)
        .raw(",\"piece\":")
        .int(PIECE as i64)
        .raw(",\"compression\":\"zlib\",\"raw\":")
        .str(RAW_NAME)
        .raw(",\"size\":")
        .int(raw.len() as i64)
        .raw(",\"size_compressed\":")
        .int(z_len as i64)
        .raw(",\"sha256\":")
        .str(&sha256_hex(raw))
        .raw(",\"rows\":")
        .int(rows as i64)
        .raw(",\"skipped_no_mass\":")
        .int(skipped as i64)
        .raw(",\"notice\":")
        .str(ATTRIBUTION)
        .raw(",\"attributions\":[");
    for (i, a) in attrib.iter().enumerate() {
        cat.raw(if i > 0 { "," } else { "" }).str(a);
    }
    cat.raw("],\"lists\":");
    lists_json(&mut cat, lists);
    cat.raw(",\"pieces\":[");
    for (i, (f, n, h)) in pieces.iter().enumerate() {
        cat.raw(if i > 0 { "," } else { "" })
            .raw("{\"file\":")
            .str(f)
            .raw(",\"size\":")
            .int(*n as i64)
            .raw(",\"sha256\":")
            .str(h)
            .raw("}");
    }
    cat.raw("]}\n");
    cat.0
}
