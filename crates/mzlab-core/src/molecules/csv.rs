//! The CSV of the molecules: the delimiter is `;` (or tab / `,` when the header has no `;`), `#` lines and empty lines are skipped, a field may be in double
//! quotes (also across lines). The header names the columns in any order (`name` and `molecular_formula` required); a list in a cell is separated by `|`; a
//! property is the column `prop:<key>[<unit>]` with `prop:<key>:source` (required) and `prop:<key>:conditions`. Every wrong row gives its own error and the
//! file is refused if there is one. `to_csv` writes the canonical columns, the same bytes that `from_csv` reads back.

use super::{finalizza, repeated, Molecola, Proprieta, RowError};

/// Canonical columns, in order.
pub const COLUMNS: [&str; 16] = [
    "id",
    "name",
    "synonyms",
    "molecular_formula",
    "charge",
    "monoisotopic_mass",
    "smiles",
    "inchikey",
    "adducts",
    "collections",
    "classes",
    "cas",
    "pubchem_cid",
    "notes",
    "origin",
    "catalog",
];

fn delimiter(text: &str) -> char {
    let head = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .unwrap_or("");
    [';', '\t', ',']
        .into_iter()
        .find(|d| head.contains(*d))
        .unwrap_or(';')
}

/// The records of a text with the 1-based line where each starts; empty records and `#` records are left out.
fn records(text: &str, d: char) -> Vec<(usize, Vec<String>)> {
    let (mut out, mut fields, mut cur) = (Vec::new(), Vec::new(), String::new());
    let (mut line, mut start, mut quoted) = (1, 1, false);
    let mut it = text.trim_start_matches('\u{feff}').chars().peekable();
    let mut end = |fields: &mut Vec<String>, cur: &mut String, start: usize| {
        fields.push(std::mem::take(cur).trim().to_string());
        let rec = std::mem::take(fields);
        if !(rec.len() == 1 && rec[0].is_empty()) && !rec[0].starts_with('#') {
            out.push((start, rec));
        }
    };
    while let Some(c) = it.next() {
        match c {
            '"' if quoted && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' if quoted || cur.trim().is_empty() => {
                quoted = !quoted;
                if quoted {
                    cur.clear();
                }
            }
            c if quoted => {
                line += usize::from(c == '\n');
                cur.push(c);
            }
            c if c == d => fields.push(std::mem::take(&mut cur).trim().to_string()),
            '\n' => {
                end(&mut fields, &mut cur, start);
                line += 1;
                start = line;
            }
            '\r' => {}
            c => cur.push(c),
        }
    }
    end(&mut fields, &mut cur, start);
    out
}

fn err(line: usize, key: &'static str, detail: impl Into<String>) -> RowError {
    RowError {
        line,
        key,
        detail: detail.into(),
    }
}

/// A property column of the header.
struct Prop {
    key: String,
    unit: String,
    value: usize,
    source: Option<usize>,
    conditions: Option<usize>,
}

struct Header {
    cols: Vec<Option<usize>>,
    props: Vec<Prop>,
}

/// `prop:key[unit]` or `prop:key[unit]:source`... as (key, unit, kind) where kind is 0 value, 1 source, 2 conditions.
fn prop_name(h: &str) -> Option<(String, Option<String>, u8)> {
    let rest = h.strip_prefix("prop:")?;
    let (body, kind) = match rest.rsplit_once(':') {
        Some((b, "source")) => (b, 1),
        Some((b, "conditions")) => (b, 2),
        _ => (rest, 0),
    };
    let (key, unit) = match body.split_once('[') {
        Some((k, u)) => (k, Some(u.strip_suffix(']')?.to_string())),
        None => (body, None),
    };
    (!key.is_empty()).then(|| (key.to_string(), unit, kind))
}

fn header(line: usize, names: &[String]) -> Result<Header, RowError> {
    let mut cols: Vec<Option<usize>> = vec![None; COLUMNS.len()];
    let mut props: Vec<Prop> = Vec::new();
    let mut extra: Vec<(usize, String, Option<String>, u8)> = Vec::new();
    for (i, raw) in names.iter().enumerate() {
        let h = raw.to_lowercase();
        if let Some(k) = COLUMNS.iter().position(|c| *c == h) {
            if cols[k].replace(i).is_some() {
                return Err(err(line, "error.molecules.badColumn", h));
            }
        } else if let Some((key, unit, kind)) = prop_name(&h) {
            match (kind, unit) {
                (0, Some(u))
                    if !u.is_empty() && !props.iter().any(|p| p.key == key && p.unit == u) =>
                {
                    props.push(Prop {
                        key,
                        unit: u,
                        value: i,
                        source: None,
                        conditions: None,
                    })
                }
                (0, _) => return Err(err(line, "error.molecules.badColumn", h)),
                (k, u) => extra.push((i, key, u, k)),
            }
        } else {
            return Err(err(line, "error.molecules.badColumn", h));
        }
    }
    for (i, key, unit, kind) in extra {
        // a source without a unit belongs to the only column of its key
        let alone = props.iter().filter(|p| p.key == key).count() == 1;
        let p = props
            .iter_mut()
            .find(|p| p.key == key && unit.as_ref().map_or(alone, |u| *u == p.unit));
        let slot = p.map(|p| {
            if kind == 1 {
                &mut p.source
            } else {
                &mut p.conditions
            }
        });
        match slot {
            Some(s) if s.is_none() => *s = Some(i),
            _ => return Err(err(line, "error.molecules.badColumn", names[i].clone())),
        }
    }
    for (k, need) in [(1, "name"), (3, "molecular_formula")] {
        if cols[k].is_none() {
            return Err(err(line, "error.molecules.noColumn", need));
        }
    }
    if let Some(p) = props.iter().find(|p| p.source.is_none()) {
        return Err(err(
            line,
            "error.molecules.noColumn",
            format!("prop:{}:source", p.key),
        ));
    }
    Ok(Header { cols, props })
}

fn items(s: &str) -> Vec<String> {
    s.split('|').map(str::to_string).collect()
}

fn row(
    h: &Header,
    cells: &[String],
    tipo: &str,
    file: Option<&str>,
) -> Result<Molecola, (&'static str, String)> {
    let at = |k: usize| {
        h.cols[k]
            .and_then(|i| cells.get(i))
            .map_or("", String::as_str)
    };
    let charge = match at(4) {
        "" => 0,
        c => c
            .parse::<i32>()
            .map_err(|_| ("error.molecules.badCharge", c.to_string()))?,
    };
    let control = match at(5) {
        "" => None,
        c => Some(
            c.parse::<f64>()
                .ok()
                .filter(|x| x.is_finite())
                .ok_or(("error.molecules.badMass", c.to_string()))?,
        ),
    };
    let cell = |i: Option<usize>| {
        i.and_then(|i| cells.get(i))
            .map_or("", String::as_str)
            .to_string()
    };
    let mut m = Molecola {
        id: at(0).into(),
        nome: at(1).into(),
        sinonimi: items(at(2)),
        formula: at(3).into(),
        carica: charge,
        smiles: Some(at(6).into()),
        inchikey: Some(at(7).into()),
        addotti: items(at(8)),
        raccolte: items(at(9)),
        classi: items(at(10)),
        cas: Some(at(11).into()),
        pubchem_cid: Some(at(12).into()),
        note: at(13).into(),
        ..Molecola::default()
    };
    m.origine.tipo = if at(14).is_empty() {
        tipo.into()
    } else {
        at(14).to_lowercase()
    };
    m.origine.catalogo = items(at(15));
    m.origine.file = file
        .filter(|_| m.origine.tipo == "importata")
        .map(str::to_string);
    for p in &h.props {
        let v = cell(Some(p.value));
        if v.is_empty() {
            continue;
        }
        let valore = v
            .parse::<f64>()
            .map_err(|_| ("error.molecules.badProperty", format!("{}: {v}", p.key)))?;
        m.proprieta.push(Proprieta {
            chiave: p.key.clone(),
            valore,
            unita: p.unit.clone(),
            fonte: cell(p.source),
            condizioni: cell(p.conditions),
        });
    }
    finalizza(&mut m, control)?;
    Ok(m)
}

/// The molecules of a CSV, in the order of the rows, or an error for every wrong row. A row without `origin` gets the level `tipo` (and `file` when it is
/// imported).
pub fn from_csv(
    text: &str,
    tipo: &str,
    file: Option<&str>,
) -> Result<Vec<Molecola>, Vec<RowError>> {
    let recs = records(text, delimiter(text));
    let Some(((hline, head), body)) = recs.split_first().map(|(f, b)| ((f.0, &f.1), b)) else {
        return Ok(Vec::new());
    };
    let h = header(hline, head).map_err(|e| vec![e])?;
    let (mut out, mut errors) = (Vec::new(), Vec::new());
    for (line, cells) in body {
        match row(&h, cells, tipo, file) {
            Ok(m) => out.push((*line, m)),
            Err((key, detail)) => errors.push(err(*line, key, detail)),
        }
    }
    repeated(&out, &mut errors);
    if errors.is_empty() {
        Ok(out.into_iter().map(|r| r.1).collect())
    } else {
        Err(errors)
    }
}

fn quote(s: &str) -> String {
    if s.contains([';', '"', '\n', '\r']) || s.starts_with('#') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The (key, unit) pairs of the properties of the records, sorted; the header names the unit with the key.
fn prop_columns(ms: &[Molecola]) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = ms
        .iter()
        .flat_map(|m| {
            m.proprieta
                .iter()
                .map(|p| (p.chiave.clone(), p.unita.clone()))
        })
        .collect();
    crate::sortc::slice_ord(&mut v);
    v.dedup();
    v
}

fn fixed_cells(m: &Molecola) -> Vec<String> {
    let opt = |s: &Option<String>| s.clone().unwrap_or_default();
    vec![
        m.id.clone(),
        m.nome.clone(),
        m.sinonimi.join("|"),
        m.formula.clone(),
        if m.carica == 0 {
            String::new()
        } else {
            m.carica.to_string()
        },
        format!("{:.6}", m.massa),
        opt(&m.smiles),
        opt(&m.inchikey),
        m.addotti.join("|"),
        m.raccolte.join("|"),
        m.classi.join("|"),
        opt(&m.cas),
        opt(&m.pubchem_cid),
        m.note.clone(),
        m.origine.tipo.clone(),
        m.origine.catalogo.join("|"),
    ]
}

/// The canonical CSV of `ms` in their order: the columns of `COLUMNS`, then every property as value, `:source` and (when any record has one) `:conditions`.
pub fn to_csv(ms: &[Molecola]) -> String {
    let pc = prop_columns(ms);
    let cond = |k: &str, u: &str| {
        ms.iter().any(|m| {
            m.proprieta
                .iter()
                .any(|p| p.chiave == k && p.unita == u && !p.condizioni.is_empty())
        })
    };
    let mut head: Vec<String> = COLUMNS.iter().map(|c| c.to_string()).collect();
    for (k, u) in &pc {
        let tag = if pc.iter().filter(|(k2, _)| k2 == k).count() > 1 {
            format!("{k}[{u}]")
        } else {
            k.clone()
        };
        head.push(format!("prop:{k}[{u}]"));
        head.push(format!("prop:{tag}:source"));
        if cond(k, u) {
            head.push(format!("prop:{tag}:conditions"));
        }
    }
    let mut out = head.join(";") + "\n";
    for m in ms {
        let mut cells = fixed_cells(m);
        for (k, u) in &pc {
            let p = m.proprieta.iter().find(|p| p.chiave == *k && p.unita == *u);
            cells.push(p.map_or(String::new(), |p| p.valore.to_string()));
            cells.push(p.map_or(String::new(), |p| p.fonte.clone()));
            if cond(k, u) {
                cells.push(p.map_or(String::new(), |p| p.condizioni.clone()));
            }
        }
        out += &cells.iter().map(|c| quote(c)).collect::<Vec<_>>().join(";");
        out.push('\n');
    }
    out
}
