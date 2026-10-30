//! The rows of one regulatory list: an entry becomes one A1+ row per CAS number, resolved or not, and the list gets its coverage counts and its
//! unresolved log. Used by `normative build` (CLI) and by `official::import` (your own file).

use crate::attributio::frame::to_csv;
use crate::attributio::reglists::schema::SCHEMA;

use super::entry::{cas_tokens, Entry, Kind};
use super::resolve::{Found, Res, Resolver};

/// What the rows need of a list: its id and its tags (`citation`, `version_date`, ...).
pub struct ListSrc<'a> {
    pub id: &'a str,
    pub tags: &'a [(String, String)],
}

/// The columns after schema A1.
pub const EXTRA: [&str; 5] = ["entry_ref", "ec", "dtxsid", "form", "status"];

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub struct Coverage {
    pub entries: usize,
    pub screenable: usize,
    pub groups: usize,
    pub uvcb: usize,
    pub inorganic: usize,
    pub unresolved: usize,
}

pub struct Built {
    pub id: String,
    pub csv: String,
    pub coverage: Coverage,
    pub unresolved: Vec<[String; 5]>,
}

/// Where an entry comes from, for the columns that do not depend on the structure.
struct RowCtx<'a> {
    src: &'a ListSrc<'a>,
    e: &'a Entry,
    version: &'a str,
    access: &'a str,
}

fn make_row(c: &RowCtx, cas: &str, f: Option<&Found>, status: &str, group: &str) -> Vec<String> {
    let (src, e, list_version, access) = (c.src, c.e, c.version, c.access);
    let tag = |t: &str| {
        src.tags
            .iter()
            .find(|(k, _)| k == t)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    };

    let mut r = vec![String::new(); SCHEMA.len() + EXTRA.len()];
    let mut set = |c: &str, v: &str| {
        let i = SCHEMA
            .iter()
            .chain(EXTRA.iter())
            .position(|x| *x == c)
            .expect("column");
        r[i] = v.to_string();
    };
    set("list_id", src.id);
    set("list_version", list_version);
    set("access_date", access);
    set("source", tag("citation"));
    set("name", &e.name);
    set("cas", cas);
    set("list_attribute", &e.attr);
    set("resolved_by", f.map_or("unresolved", |f| f.by));
    set("entry_ref", &e.entry_ref);
    set("ec", &e.ec);
    set("dtxsid", &e.dtxsid);
    set("status", status);
    if let Some(f) = f {
        for (c, v) in [
            ("pubchem_cid", f.cid.as_str()),
            ("inchikey", &f.inchikey),
            ("inchikey_skeleton", &f.skeleton),
            ("smiles", &f.smiles),
            ("formula", &f.formula),
            ("mono_mass", &f.mono_mass),
            ("form", f.form),
        ] {
            set(c, v);
        }
    }
    if !group.is_empty() {
        set("parent", "");
        set("tp_reference", group);
    }
    r
}

pub fn rows_of(
    src: &ListSrc,
    e: &Entry,
    res: &Resolver,
    list_version: &str,
    access: &str,
    resolve: bool,
) -> Vec<(Vec<String>, Option<String>)> {
    let ctx = RowCtx {
        src,
        e,
        version: list_version,
        access,
    };
    let row = |cas: &str, f: Option<&Found>, status: &str, group: &str| {
        make_row(&ctx, cas, f, status, group)
    };
    let tokens = cas_tokens(&e.cas);
    let structure_rows = matches!(e.kind, Kind::Structure | Kind::Group)
        && (!tokens.is_empty() || e.kind == Kind::Structure);
    if !structure_rows {
        return vec![(row(&e.cas, None, e.kind.word(), ""), None)];
    }
    let group = if e.kind == Kind::Group {
        e.name.as_str()
    } else {
        e.group_entry.as_str()
    };
    let one = |cas: &str| -> (Vec<String>, Option<String>) {
        if !resolve {
            return (
                row(cas, None, "unresolved", group),
                Some("resolution skipped".into()),
            );
        }
        match res.resolve(src.id, &e.entry_ref, cas, &e.dtxsid, &e.law_key) {
            Res::Found(f) => (row(cas, Some(&f), "screenable", group), None),
            Res::Unresolved(why) => (row(cas, None, "unresolved", group), Some(why)),
        }
    };
    if tokens.is_empty() {
        vec![one("")]
    } else {
        tokens.iter().map(|t| one(t)).collect()
    }
}

pub fn build_list(
    src: &ListSrc,
    entries: &[Entry],
    access: &str,
    res: &Resolver,
    resolve: bool,
) -> Built {
    let version = src
        .tags
        .iter()
        .find(|(k, _)| k == "version_date")
        .map_or("", |(_, v)| v.as_str());
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut unresolved = Vec::new();
    let mut cov = Coverage {
        entries: entries.len(),
        ..Coverage::default()
    };
    for e in entries {
        match e.kind {
            Kind::Group => cov.groups += 1,
            Kind::Uvcb => cov.uvcb += 1,
            Kind::Inorganic => cov.inorganic += 1,
            Kind::Structure => {}
        }
        for (r, why) in rows_of(src, e, res, version, access, resolve) {
            let status = r[SCHEMA.len() + 4].clone();
            if status == "screenable" {
                cov.screenable += 1;
            } else if status == "unresolved" {
                cov.unresolved += 1;
                unresolved.push([
                    src.id.to_string(),
                    e.entry_ref.clone(),
                    e.name.clone(),
                    r[5].clone(),
                    why.unwrap_or_default(),
                ]);
            }
            rows.push(r);
        }
    }
    let key = |r: &Vec<String>| {
        (
            r[SCHEMA.len()].clone(),
            r[8].clone(),
            r[4].clone(),
            r[5].clone(),
        )
    };
    rows.sort_by_key(key);
    rows.dedup();
    unresolved.sort();
    unresolved.dedup();
    let cols: Vec<String> = SCHEMA
        .iter()
        .chain(EXTRA.iter())
        .map(|s| s.to_string())
        .collect();
    Built {
        id: src.id.to_string(),
        csv: to_csv(&cols, &rows, ';'),
        coverage: cov,
        unresolved,
    }
}
