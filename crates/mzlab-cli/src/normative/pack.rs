//! `normative pack IN OUT`: the committed lists (`IN/lists/*.csv`, `IN/provenance.json`) become the data release: `normative.mzsus` (one row per skeleton, the
//! `listings` extension of `mzlab_feat::suspects`), its zlib pieces of 4 MiB with SHA-256 and `catalogo.json` (`normative/1`). No network, no clock: same
//! input, same bytes. `normative listings PATH` reads a package back (folder or `.mzsus`). Used by `pages.yml` and the app; layout in `docs/agenti/sospetti.md`.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{self, Out, Value};
use mzlab_feat::attributio::reglists::catalog;
use mzlab_feat::suspects::{Suspects, BIT_ANION_CAS, FLAG_LISTINGS, FORM_NAMES, SRC_NORMATIVE};

use super::csvrd::Records;
use super::NormError;

pub const FORMAT: &str = "normative/1";
pub const PIECE: usize = 4 << 20;
pub const RAW_NAME: &str = "normative.mzsus";
const ATTRIBUTION: &str = "Regulatory lists built from the official publications (EU Official Journal, US eCFR and EPA, EU Pesticides Database, NORMAN SusDat); \
only the official publications are authentic. Per-list licences and attributions are in catalogo.json.";

struct Member {
    list: u16,
    form: u8,
    bits: u8,
    entry_ref: String,
    entry: String,
    full_key: String,
    name: String,
}

struct Row {
    mass: f64,
    formula: String,
    members: Vec<Member>,
}

fn io(p: &Path, e: &std::io::Error) -> NormError {
    NormError::io(p, e)
}

fn read(p: &Path) -> Result<Vec<u8>, NormError> {
    std::fs::read(p).map_err(|e| io(p, &e))
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
) -> Result<usize, NormError> {
    let mut skipped = 0;
    let mut rec = Records::new(text.as_bytes(), b';');
    let head = rec
        .next()
        .ok_or_else(|| NormError::parse(id, "empty csv"))?;
    let col = |n: &str| head.iter().position(|h| h == n);
    let need = |n: &str| col(n).ok_or_else(|| NormError::parse(id, &format!("column {n} missing")));
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
    formulas.sort_unstable();
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
        o.raw("}");
    }
    o.raw("]");
}

/// zlib (level 9) of `raw`.
fn deflate(raw: &[u8]) -> Result<Vec<u8>, NormError> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(9));
    e.write_all(raw)
        .and_then(|_| e.finish())
        .map_err(|e| NormError::parse("pack", &e.to_string()))
}

/// Every screenable row of the lists, memberships ordered and deduplicated; the count of rows without mass or formula that are left out.
fn build_rows(input: &Path, order: &[String]) -> Result<(BTreeMap<String, Row>, usize), NormError> {
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    let mut skipped = 0;
    for (i, id) in order.iter().enumerate() {
        let p = input.join("lists").join(format!("{id}.csv"));
        let text = String::from_utf8_lossy(&read(&p)?).into_owned();
        skipped += read_list(&text, i as u16, id, &mut rows)?;
    }
    for r in rows.values_mut() {
        r.members
            .sort_by(|a, b| (a.list, &a.entry_ref, a.form).cmp(&(b.list, &b.entry_ref, b.form)));
        r.members
            .dedup_by(|a, b| (a.list, a.form, &a.entry) == (b.list, b.form, &b.entry));
    }
    Ok((rows, skipped))
}

fn manifest_json(rows: usize, lists: &[&Value]) -> String {
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

/// The compressed package cut into pieces `NNN.bin` of at most `PIECE` bytes: `(file, size, sha256)`.
fn write_pieces(out: &Path, z: &[u8]) -> Result<Vec<(String, usize, String)>, NormError> {
    let mut pieces = Vec::new();
    for (i, b) in z.chunks(PIECE).enumerate() {
        let name = format!("{i:03}.bin");
        std::fs::write(out.join(&name), b).map_err(|e| io(&out.join(&name), &e))?;
        pieces.push((name, b.len(), sha256_hex(b)));
    }
    Ok(pieces)
}

fn catalog_json(
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

pub fn pack(input: &Path, out: &Path) -> Result<Vec<String>, NormError> {
    let prov_path = input.join("provenance.json");
    let prov = json::parse(&read(&prov_path)?)
        .map_err(|_| NormError::parse("provenance.json", "not JSON"))?;
    let all: Vec<&Value> = prov
        .get("lists")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .collect();
    let ids: Vec<&str> = all
        .iter()
        .filter_map(|l| l.get("id").and_then(Value::str))
        .collect();
    let order = catalog::ordered(&ids);
    let lists: Vec<&Value> = order
        .iter()
        .filter_map(|id| {
            all.iter()
                .copied()
                .find(|l| l.get("id").and_then(Value::str) == Some(id))
        })
        .collect();
    if lists.is_empty() {
        return Err(NormError::parse("provenance.json", "no lists"));
    }
    let (rows, skipped) = build_rows(input, &order)?;
    let mut sorted: Vec<(&String, &Row)> = rows.iter().collect();
    sorted.sort_by(|a, b| a.1.mass.total_cmp(&b.1.mass).then_with(|| a.0.cmp(b.0)));

    let man = manifest_json(sorted.len(), &lists);
    let raw = write_mzsus(&sorted, lists.len(), &man);
    Suspects::open(&raw).map_err(|e| NormError::parse("pack", &format!("{e:?}")))?;

    std::fs::create_dir_all(out).map_err(|e| io(out, &e))?;
    let z = deflate(&raw)?;
    let pieces = write_pieces(out, &z)?;
    let cat = catalog_json(&raw, z.len(), sorted.len(), skipped, &lists, &pieces);
    let cp = out.join("catalogo.json");
    std::fs::write(&cp, &cat).map_err(|e| io(&cp, &e))?;
    std::fs::write(out.join(RAW_NAME), &raw).map_err(|e| io(&out.join(RAW_NAME), &e))?;
    Ok(vec![format!(
        "{} rows, {} lists, {} B raw, {} B compressed in {} piece(s); {skipped} screenable row(s) without mass or formula not packed",
        sorted.len(),
        lists.len(),
        raw.len(),
        z.len(),
        pieces.len()
    )])
}

/// `normative listings PATH`: the `listings` JSON of a package folder (with `normative.mzsus`) or of a `.mzsus` file.
pub fn listings(path: &Path) -> Result<String, NormError> {
    let p = if path.is_dir() {
        path.join(RAW_NAME)
    } else {
        path.to_path_buf()
    };
    let s =
        Suspects::open(&read(&p)?).map_err(|e| NormError::parse("listings", &format!("{e:?}")))?;
    s.listings_json()
        .ok_or_else(|| NormError::parse("listings", "the file has no listings extension"))
}
