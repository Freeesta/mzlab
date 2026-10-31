//! The text of `annot.regulatory@1` (`list_membership`): the identities of an `annot/1` text against the regulatory package (`.mzsus` with `listings`), through
//! B1 `match_campaign`: the same full InChIKey is an `InChIKey` match, the same first block only a flagged `first block` match, a name or a formula never.
//! Each row gets its `regulatory` field; the rest of the text is copied. `flow run` (`mzlab-cli`) and the browser (`attributio-all` .wasm) run this one function.

use crate::attributio::reglists::b1::{match_campaign, B1Config, IdentityIn, StructRow};
use crate::attributio::tox::snapshot::MembershipRow;
use crate::regulatory::{write_member, write_words, Member, Package};
use crate::suspects::Suspects;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, write_value, Out, Value};

/// The parameters of the step.
pub struct Params {
    pub lists: String,
    pub include: String,
    /// Monoisotopic mass range of the listed rows kept (Da); `None`: all.
    pub window: Option<(f64, f64)>,
}

impl Params {
    /// From the resolved parameters of the step, or the object the page sends.
    pub fn from_params(p: &[(String, Value)]) -> Params {
        let get = |n: &str| p.iter().find(|(k, _)| k == n).map(|(_, v)| v);
        let text = |n: &str| get(n).and_then(Value::str).unwrap_or("").to_string();
        let window = get("window").and_then(|v| {
            let a = v.arr();
            let (lo, hi) = (a.first()?.f64()?, a.get(1)?.f64()?);
            (lo.is_finite() && hi.is_finite() && hi > lo).then_some((lo, hi))
        });
        Params {
            lists: text("lists"),
            include: text("include"),
            window,
        }
    }

    /// The same from the parameters as JSON (the browser).
    pub fn from_json(text: &str) -> Result<Params> {
        match json::parse(text.as_bytes())? {
            Value::Obj(o) => Ok(Params::from_params(&o)),
            _ => Ok(Params::from_params(&[])),
        }
    }
}

/// The InChIKey a row names (`entry` of `annot.level`, else `inchikey`), when it has the shape of one: the first block and the full key.
fn key_of(row: &Value) -> Option<(String, Option<String>)> {
    let k = ["entry", "inchikey"]
        .iter()
        .find_map(|f| row.get(f).and_then(Value::str))?
        .trim()
        .to_ascii_uppercase();
    let b = k.as_bytes();
    if b.len() < 14
        || !b[..14].iter().all(u8::is_ascii_uppercase)
        || (b.len() > 14 && b[14] != b'-')
    {
        return None;
    }
    Some((k[..14].to_string(), (k.len() == 27).then_some(k)))
}

/// The memberships per row (match basis, listed key, membership).
type Found = Vec<Vec<(&'static str, String, Member)>>;

fn find(rows: &[Value], s: &Suspects, pk: &Package, window: Option<(f64, f64)>) -> Found {
    let l = s.listings();
    let keyed: Vec<Option<(String, Option<String>)>> = rows.iter().map(key_of).collect();
    let (mut idents, mut structs) = (Vec::new(), Vec::new());
    for (k, (row, key)) in rows.iter().zip(&keyed).enumerate() {
        let Some((block, full)) = key else { continue };
        let id = format!("{k}");
        idents.push(IdentityIn {
            nome_comune: id.clone(),
            ..IdentityIn::default()
        });
        structs.push(StructRow {
            nome_comune: id,
            identity: row
                .get("name")
                .and_then(Value::str)
                .unwrap_or("")
                .to_string(),
            first_block: Some(block.clone()),
            inchikey: full.clone(),
            ..StructRow::default()
        });
    }
    // the memberships of the listed rows whose first block an identity has; `list_attribute` carries the index into `members`
    let mut members: Vec<Member> = Vec::new();
    let mut mem: Vec<MembershipRow> = Vec::new();
    for i in 0..s.len() {
        let h = s.row(i);
        let block = String::from_utf8_lossy(&h.key).into_owned();
        if window.is_some_and(|(lo, hi)| h.mass < lo || h.mass > hi)
            || !keyed.iter().flatten().any(|(b, _)| *b == block)
        {
            continue;
        }
        for m in pk.members(s, i) {
            mem.push(MembershipRow {
                first_block: block.clone(),
                list_id: m.list.clone(),
                name: l.map_or("", |l| l.name(i)).to_string(),
                list_attribute: members.len().to_string(),
                inchikey: l.map_or("", |l| l.full_key(i)).to_string(),
                ..MembershipRow::default()
            });
            members.push(m);
        }
    }
    let used = pk.used();
    let b1 = match_campaign(&idents, &structs, &mem, &B1Config::default(), Some(&used));
    let mut out: Found = vec![Vec::new(); rows.len()];
    for r in &b1.long {
        let (Ok(k), Ok(m)) = (
            r.nome_comune.parse::<usize>(),
            r.list_attribute.parse::<usize>(),
        ) else {
            continue;
        };
        out[k].push((r.matched, r.list_inchikey.clone(), members[m].clone()));
    }
    out
}

/// The `imported` record of the `regulatory` block: name, SHA-256 and lists of each imported file; nothing without one.
fn write_imported(o: &mut Out, files: &[(&str, &[u8])], opened: &[(Suspects, Package)]) {
    if opened.is_empty() {
        return;
    }
    o.raw(",\"imported\":[");
    for (k, ((name, bytes), (_, ipk))) in files.iter().zip(opened).enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(name)
            .raw(",\"sha256\":")
            .str(&mzlab_core::budget::sha256_hex(bytes))
            .raw(",\"lists\":");
        ipk.write_lists(o);
        o.raw("}");
    }
    o.raw("]");
}

/// An imported list file, opened: every list of it is compared (the user chose the file, not the lists).
fn open_imported(name: &str, bytes: &[u8]) -> Result<(Suspects, Package)> {
    let bad = || Error::new("err.flow.regulatory.imported").with("file", name);
    let s = Suspects::open(bytes).map_err(|_| bad())?;
    let pk = Package::open(&s, "", "")?.ok_or_else(bad)?;
    Ok((s, pk))
}

/// Runs the step on an `annot/1` text, the package bytes and the imported list files (`.mzsus` each, after the package); returns the `annot/1` text with the `regulatory` field of every row.
pub fn regulatory_text(
    p: &Params,
    annot: &[u8],
    package: &[u8],
    imported: &[(&str, &[u8])],
) -> Result<String> {
    let s = Suspects::open(package)
        .map_err(|e| Error::new("err.flow.regulatory.package").with("reason", format!("{e:?}")))?;
    let pk = Package::open(&s, &p.lists, &p.include)?
        .ok_or_else(|| Error::new("err.flow.regulatory.package").with("reason", "no listings"))?;
    let v = json::parse(annot)?;
    if v.get("format").and_then(Value::str) != Some("annot/1") {
        return Err(Error::new("err.flow.regulatory.annot"));
    }
    let rows = v.get("rows").map(Value::arr).unwrap_or(&[]);
    let mut found = find(rows, &s, &pk, p.window);
    let opened: Vec<(Suspects, Package)> = imported
        .iter()
        .map(|(n, b)| open_imported(n, b))
        .collect::<Result<_>>()?;
    for (is, ipk) in &opened {
        for (all, more) in found.iter_mut().zip(find(rows, is, ipk, p.window)) {
            all.extend(more);
        }
    }
    let mut o = Out::default();
    o.raw("{");
    let mut first = true;
    for (k, val) in v.entries() {
        if matches!(k.as_str(), "rows" | "regulatory") {
            continue;
        }
        o.raw(if first { "" } else { "," }).str(k).raw(":");
        first = false;
        write_value(&mut o, val);
    }
    o.raw(",\"regulatory\":{\"step\":\"annot.regulatory@1\",\"algorithm\":\"list_membership\",\"lists_asked\":");
    write_words(&mut o, &p.lists);
    o.raw(",\"include\":");
    write_words(&mut o, &p.include);
    o.raw(",\"package_sha256\":")
        .str(&mzlab_core::budget::sha256_hex(package))
        .raw(",\"lists\":");
    pk.write_lists(&mut o);
    write_imported(&mut o, imported, &opened);
    o.raw("},\"rows\":[");
    for (k, row) in rows.iter().enumerate() {
        o.raw(if k > 0 { ",{" } else { "{" });
        let mut first = true;
        for (f, val) in row.entries() {
            if f == "regulatory" {
                continue;
            }
            o.raw(if first { "" } else { "," }).str(f).raw(":");
            first = false;
            write_value(&mut o, val);
        }
        o.raw(if first {
            "\"regulatory\":["
        } else {
            ",\"regulatory\":["
        });
        for (j, (basis, key, m)) in found[k].iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" })
                .raw("{\"match\":")
                .str(basis)
                .raw(",\"listed_inchikey\":")
                .str(key)
                .raw(",\"membership\":");
            write_member(&mut o, m);
            o.raw("}");
        }
        o.raw("]}");
    }
    o.raw("]}");
    Ok(o.0)
}
