//! The suspect lists of `screen.suspects`: a `.mzsus` (`suspects.rs`) or a CSV of the user, both read into one `Source` that answers a mass search with `Entry` rows.
//! Names, RT and fragments exist only in a CSV; a `.mzsus` v1 has the first block, the formula and the mass; one with the `listings` extension (the regulatory
//! package) also has the full key, a name and the laws naming the row, cut to the lists of the mask.

use crate::regulatory::{Member, Package};
use crate::suspects::Suspects;
use mzlab_core::budget::sha256_hex;
use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out, Value};

/// One suspect with what the list says of it. `skeleton` is the InChIKey first block (the match key); a CSV row without a key has none and is told apart by its name.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub skeleton: String,
    pub inchikey: String,
    pub name: String,
    pub smiles: String,
    /// The formula as the Hill text (`elements::fmt`), the key of the isomer group.
    pub formula: String,
    pub mass: f64,
    pub rt: Option<f64>,
    pub fragments: Vec<f64>,
    /// Lists of the regulatory package that name the structure and pass the mask (`[]` for a file without the `listings` extension).
    pub memberships: Vec<Member>,
}

enum Rows {
    Mzsus(Suspects),
    /// Sorted by mass, then skeleton.
    Csv(Vec<Entry>),
}

/// A suspect file with its provenance.
pub struct Source {
    pub name: String,
    pub sha256: String,
    rows: Rows,
    /// The manifest of the `.mzsus` (sources, licence, attribution) as the JSON of the output header; `null` for a CSV.
    pub provenance: String,
    pub skipped: usize,
    /// The lists of a regulatory package and the mask; `None` for any other file.
    pub package: Option<Package>,
}

impl Source {
    /// A file of the plan: a `.mzsus` when it starts with the magic, else a CSV. `lists` and `include` are the mask of a regulatory package.
    pub fn open(name: &str, bytes: &[u8], (lists, include): (&str, &str)) -> Result<Source> {
        let sha256 = sha256_hex(bytes);
        if bytes.starts_with(b"MZSU") {
            let s = Suspects::open(bytes).map_err(|e| {
                Error::new("err.flow.screen.mzsus")
                    .with("file", name)
                    .with("reason", format!("{e:?}"))
            })?;
            let provenance = manifest_json(s.manifest());
            let package = Package::open(&s, lists, include)?;
            return Ok(Source {
                name: name.into(),
                sha256,
                rows: Rows::Mzsus(s),
                provenance,
                skipped: 0,
                package,
            });
        }
        let (rows, skipped) = read_csv(name, bytes)?;
        Ok(Source {
            name: name.into(),
            sha256,
            rows: Rows::Csv(rows),
            provenance: "null".into(),
            skipped,
            package: None,
        })
    }

    pub fn kind(&self) -> &'static str {
        match self.rows {
            Rows::Mzsus(_) => "mzsus",
            Rows::Csv(_) => "csv",
        }
    }

    pub fn len(&self) -> usize {
        match &self.rows {
            Rows::Mzsus(s) => s.len(),
            Rows::Csv(r) => r.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn hill(formula: &str) -> String {
    match elements::parse_formula(formula) {
        Ok(f) => elements::fmt(&f),
        Err(_) => formula.trim().to_string(),
    }
}

impl Source {
    /// The entries with `|m - mass| <= mass * ppm * 1e-6` (the window of `Suspects::search`).
    pub fn search(&self, mass: f64, ppm: f64) -> Vec<Entry> {
        match &self.rows {
            Rows::Mzsus(s) => s
                .search_range(mass, ppm)
                .filter_map(|i| {
                    let h = s.row(i);
                    let key = String::from_utf8_lossy(&h.key).into_owned();
                    let (mut inchikey, mut name, mut memberships) =
                        (key.clone(), String::new(), Vec::new());
                    if let (Some(pk), Some(l)) = (&self.package, s.listings()) {
                        memberships = pk.members(s, i);
                        // a row named only by lists outside the mask is not a suspect of this run
                        if memberships.is_empty() {
                            return None;
                        }
                        if !l.full_key(i).is_empty() {
                            inchikey = l.full_key(i).to_string();
                        }
                        name = l.name(i).to_string();
                    }
                    Some(Entry {
                        inchikey,
                        skeleton: key,
                        name,
                        smiles: String::new(),
                        formula: hill(s.formula(h)),
                        mass: h.mass,
                        rt: None,
                        fragments: Vec::new(),
                        memberships,
                    })
                })
                .collect(),
            Rows::Csv(rows) => {
                let d = mass * ppm * 1e-6;
                let a = rows.partition_point(|e| e.mass < mass - d);
                let b = rows.partition_point(|e| e.mass <= mass + d);
                rows[a..b.max(a)].to_vec()
            }
        }
    }
}

fn manifest_json(text: &str) -> String {
    let Ok(v) = json::parse(text.as_bytes()) else {
        return "null".into();
    };
    let mut o = Out::default();
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::str).unwrap_or("").to_string();
    o.raw("{\"sources\":[");
    for (i, f) in v
        .get("fonti")
        .map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        if i > 0 {
            o.raw(",");
        }
        o.raw("{\"id\":").str(&s(f, "id"));
        for (key, from) in [
            ("name", "nome"),
            ("version", "versione"),
            ("licence", "licenza"),
            ("doi", "doi"),
        ] {
            o.raw(&format!(",\"{key}\":")).str(&s(f, from));
        }
        o.raw("}");
    }
    o.raw("],\"attribution\":")
        .str(&s(&v, "attribuzione"))
        .raw("}");
    o.0
}

/// A CSV line split at `,`; a field in double quotes may hold commas and `""`.
fn split(line: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if quoted && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn is_key(s: &str) -> bool {
    s.len() >= 14 && s.as_bytes()[..14].iter().all(u8::is_ascii_alphabetic)
}

/// The rows of a user CSV: `name`, `inchikey` or `smiles`, `formula` or `mass` (neutral, monoisotopic), optional `rt` (min) and `fragments` (m/z separated by `;`).
/// A row without a usable mass is skipped and counted.
fn read_csv(name: &str, bytes: &[u8]) -> Result<(Vec<Entry>, usize)> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let head: Vec<String> = split(lines.next().unwrap_or("").trim_start_matches('\u{feff}'))
        .iter()
        .map(|h| h.to_ascii_lowercase())
        .collect();
    let col = |names: &[&str]| head.iter().position(|h| names.contains(&h.as_str()));
    let (cname, ckey, csmiles) = (
        col(&["name"]),
        col(&["inchikey", "inchi_key"]),
        col(&["smiles"]),
    );
    let (cform, cmass) = (
        col(&["formula", "neutral_formula"]),
        col(&["mass", "neutral_mass", "monoisotopic_mass"]),
    );
    let (crt, cfrag) = (col(&["rt", "rt_min"]), col(&["fragments", "fragment_mz"]));
    if (ckey.is_none() && csmiles.is_none()) || (cform.is_none() && cmass.is_none()) {
        return Err(Error::new("err.flow.screen.csv").with("file", name));
    }
    let mut rows = Vec::new();
    let mut skipped = 0;
    for line in lines {
        let r = split(line);
        let get = |c: Option<usize>| c.and_then(|i| r.get(i)).map(String::as_str).unwrap_or("");
        let number = |c: Option<usize>| get(c).parse::<f64>().ok().filter(|v| v.is_finite());
        let formula = get(cform);
        let mass = number(cmass).or_else(|| {
            elements::parse_formula(formula)
                .ok()
                .map(|f| elements::mass(&f))
        });
        let key = get(ckey).to_ascii_uppercase();
        let Some(mass) = mass.filter(|m| *m > 0.0) else {
            skipped += 1;
            continue;
        };
        rows.push(Entry {
            skeleton: if is_key(&key) {
                key[..14].to_string()
            } else {
                String::new()
            },
            inchikey: if is_key(&key) { key } else { String::new() },
            name: get(cname).to_string(),
            smiles: get(csmiles).to_string(),
            formula: if formula.is_empty() {
                String::new()
            } else {
                hill(formula)
            },
            mass,
            rt: number(crt),
            fragments: get(cfrag)
                .split(';')
                .filter_map(|v| v.trim().parse().ok())
                .collect(),
            memberships: Vec::new(),
        });
    }
    rows.sort_unstable_by(|a, b| {
        a.mass
            .total_cmp(&b.mass)
            .then_with(|| a.skeleton.cmp(&b.skeleton))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.smiles.cmp(&b.smiles))
            .then_with(|| a.formula.cmp(&b.formula))
    });
    Ok((rows, skipped))
}
