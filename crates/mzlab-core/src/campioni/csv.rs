//! The metadata of `campioni/3` as a CSV table (`,` `;` or tab): one row per file, the header the names of the fields (`file`, `sha256`, `campione_id`,
//! `polarita`, ..., then one column per free column); `condition`, `replicate`, `injection_order`, `role` (`control` = dark, `standard` = std),
//! `time` (minutes), `dose` (`25 ppm`) and the like are read too, and an unknown column becomes a free column (a number when every value is one).
//! What `csv_scrivi` writes `csv_leggi` reads back unchanged (`tempo_min` is derived, `dati_file` is not in the table).

use std::borrow::Cow;

use super::colonne::infer;
use super::json::legacy_kind;
use super::{
    number_replicates, tabella_da_value, to_json3, Campione, Cell, Colonna, Tabella, TipoColonna,
};
use crate::error::{Error, Result};
use crate::json::{parse, Out};

/// The fixed columns, in the order written.
const FIXED: [&str; 23] = [
    "file",
    "campione",
    "sha256",
    "campione_id",
    "polarita",
    "matrice",
    "lotto",
    "acquisito",
    "gruppo",
    "tipo",
    "tipo_bianco",
    "tempo",
    "unita_tempo",
    "tempo_confermato",
    "serie",
    "reiniezione",
    "replica",
    "concentrazione",
    "unita",
    "ordine_iniezione",
    "istd",
    "note",
    "composto",
];

fn bad(row: usize, col: &str) -> Error {
    Error::new("error.badCsv")
        .with("riga", row)
        .with("colonna", col)
}

/// `v.to_lowercase()`, without a copy when `v` is already lower-case ASCII (the usual cell).
fn lower(v: &str) -> Cow<'_, str> {
    if v.bytes().any(|b| b.is_ascii_uppercase() || !b.is_ascii()) {
        Cow::Owned(v.to_lowercase())
    } else {
        Cow::Borrowed(v)
    }
}

/// The field a header names (the fixed name, or a synonym), None for a free column.
fn field_of(h: &str) -> Option<&'static str> {
    let l = h.trim().to_lowercase();
    if let Some(f) = FIXED.iter().find(|f| **f == l) {
        return Some(f);
    }
    Some(match l.as_str() {
        "name" | "nome" | "filename" => "file",
        "sample_id" | "sample" => "campione_id",
        "polarity" => "polarita",
        "matrix" => "matrice",
        "batch" => "lotto",
        "acquired" => "acquisito",
        "group" => "gruppo",
        "role" | "type" => "tipo",
        "condition" => "serie",
        "replicate" => "replica",
        "injection_order" => "ordine_iniezione",
        "tempo_min" | "time" => "tempo_min",
        "dose" => "dose",
        _ => return None,
    })
}

type Cells<'a> = Vec<(Cow<'a, str>, bool)>;

/// The field being read: a run of `text` still borrowed (`start..`), the text that came before it when the field stopped being one slice (`""`, text after a closing quote), and the flags.
struct Field {
    owned: String,
    has_owned: bool,
    start: usize,
    /// End of the run when a closing quote ended it (the quote itself is not content).
    stop: Option<usize>,
    quoted: bool,
}

impl Field {
    /// Whether what the field holds so far, with the run ending at `end`, is blank (a quote opens a quoted field only then).
    fn blank(&self, text: &str, end: usize) -> bool {
        self.owned.trim().is_empty() && text[self.start..end].trim().is_empty()
    }

    /// Whether the field holds nothing at all (before any trimming), with the run ending at `end`.
    fn empty(&self, end: usize) -> bool {
        self.owned.is_empty() && self.start >= end
    }

    /// The finished field, its run ending at `end` (the closing quote's position when there was one), and the start of the next.
    fn take<'a>(&mut self, text: &'a str, end: usize, next: usize) -> (Cow<'a, str>, bool) {
        let end = self.stop.unwrap_or(end);
        let run = &text[self.start..end];
        let v = if self.has_owned {
            self.owned.push_str(run);
            Cow::Owned(std::mem::take(&mut self.owned))
        } else if self.quoted {
            Cow::Borrowed(run)
        } else {
            Cow::Borrowed(run.trim())
        };
        let quoted = std::mem::take(&mut self.quoted);
        self.has_owned = false;
        self.stop = None;
        self.start = next;
        (v, quoted)
    }
}

/// The records of `text`: quoted fields keep their separators and newlines (`""` is a quote); a field is trimmed unless quoted. A field is a slice of `text` unless it holds a `""` or text after
/// its closing quote. Scans bytes: `"`, `\n`, `\r` and the separator are the only bytes that matter and none of them occurs inside a multi-byte character.
fn records(text: &str, sep: char) -> Result<Vec<Cells<'_>>> {
    let text = text.trim_start_matches('\u{feff}');
    let bytes = text.as_bytes();
    let mut sb = [0u8; 4];
    let sepb = sep.encode_utf8(&mut sb).as_bytes();
    let special = |b: u8| matches!(b, b'"' | b'\n' | b'\r') || b == sepb[0];
    let mut out: Vec<Cells> = Vec::new();
    let (mut row, mut width): (Cells, usize) = (Vec::new(), 0);
    let mut f = Field {
        owned: String::new(),
        has_owned: false,
        start: 0,
        stop: None,
        quoted: false,
    };
    let (mut at, mut inq) = (0, false);
    while let Some(d) = bytes[at..].iter().position(|&b| special(b)) {
        let j = at + d;
        let b = bytes[j];
        at = j + 1;
        if inq {
            if b != b'"' {
                continue;
            }
            if bytes.get(j + 1) == Some(&b'"') {
                // `""`: the first quote is content, the second is skipped
                f.owned.push_str(&text[f.start..=j]);
                (f.has_owned, f.start, at) = (true, j + 2, j + 2);
                continue;
            }
            inq = false;
            let ends = match bytes.get(j + 1) {
                None => true,
                Some(&n) => matches!(n, b'\n' | b'\r') || bytes[j + 1..].starts_with(sepb),
            };
            if ends {
                f.stop = Some(j);
            } else {
                f.owned.push_str(&text[f.start..j]);
                (f.has_owned, f.start) = (true, j + 1);
            }
        } else if b == b'"' && f.blank(text, j) {
            f.owned.clear();
            (f.has_owned, f.start, f.quoted, inq) = (false, j + 1, true, true);
        } else if b == sepb[0] && bytes[j..].starts_with(sepb) {
            at = j + sepb.len();
            row.push(f.take(text, j, at));
        } else if b == b'\n' || b == b'\r' {
            if b == b'\r' && bytes.get(j + 1) == Some(&b'\n') {
                at += 1;
            }
            row.push(f.take(text, j, at));
            let blank = row.len() == 1 && row[0].0.is_empty() && !row[0].1;
            if blank {
                row.clear();
            } else {
                width = width.max(row.len());
                out.push(std::mem::replace(&mut row, Vec::with_capacity(width)));
            }
        }
    }
    if inq {
        return Err(bad(out.len() + 1, ""));
    }
    let end = f.stop.unwrap_or(bytes.len());
    if !f.empty(end) || !row.is_empty() {
        row.push(f.take(text, bytes.len(), bytes.len()));
        out.push(row);
    }
    Ok(out)
}

/// The separator of the first line: the one of `,` `;` tab used most outside quotes (`,` when none).
fn detect(text: &str) -> char {
    let mut n = [0usize; 3];
    let mut inq = false;
    for c in text.trim_start_matches('\u{feff}').chars() {
        match c {
            '"' => inq = !inq,
            '\n' | '\r' if !inq => break,
            ',' if !inq => n[0] += 1,
            ';' if !inq => n[1] += 1,
            '\t' if !inq => n[2] += 1,
            _ => {}
        }
    }
    let best = (0..3).max_by_key(|&i| (n[i], 3 - i)).unwrap_or(0);
    if n[best] == 0 {
        ','
    } else {
        [',', ';', '\t'][best]
    }
}

fn num(v: &str) -> Option<f64> {
    v.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite())
}

fn flag(v: &str) -> Option<bool> {
    match lower(v).as_ref() {
        "true" | "1" | "si" | "sì" | "yes" | "y" | "vero" => Some(true),
        "false" | "0" | "no" | "n" | "falso" => Some(false),
        _ => None,
    }
}

fn unit_of(v: &str) -> Option<&'static str> {
    super::UNITA_TEMPO.iter().copied().find(|u| *u == lower(v))
}

fn whole(v: &str) -> Option<u32> {
    v.parse::<u32>().ok().filter(|&n| n >= 1)
}

/// `25 ppm` or `25ppm` → (25, Some("ppm")).
fn dose(v: &str) -> Option<(f64, Option<String>)> {
    let cut = v
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+' | 'e' | 'E')))
        .unwrap_or(v.len());
    let (n, u) = v.split_at(cut);
    let u = u.trim();
    num(n).map(|x| (x, (!u.is_empty()).then(|| u.to_string())))
}

/// Gives `c` the value of one fixed column; false when the value is not one the column takes.
fn put(c: &mut Campione, f: &str, v: &str) -> bool {
    let pol = |p: &str| match lower(p).as_ref() {
        "1" | "+1" | "+" | "pos" | "positive" => Some(1),
        "-1" | "-" | "neg" | "negative" => Some(-1),
        _ => None,
    };
    match f {
        "file" => c.file = v.into(),
        "sha256" => c.sha256 = v.into(),
        "campione" => c.sub = v.into(),
        "campione_id" => c.sample_id = v.into(),
        "matrice" => c.matrix = v.into(),
        "lotto" => c.batch = v.into(),
        "gruppo" => c.group = v.into(),
        "serie" => c.series = v.into(),
        "note" => c.note = v.into(),
        "composto" => c.compound = v.into(),
        "acquisito" => c.acquired = (!v.is_empty()).then(|| v.into()),
        "unita" => c.unit = (!v.is_empty()).then(|| v.into()),
        "polarita" => return v.is_empty() || pol(v).map(|p| c.polarity = Some(p)).is_some(),
        "tipo" => {
            return v.is_empty() || legacy_kind(&lower(v)).map(|k| c.kind = k.into()).is_some()
        }
        "tipo_bianco" => {
            return v.is_empty()
                || super::TIPI_BIANCO
                    .contains(&v)
                    .then(|| c.blank_kind = Some(v.into()))
                    .is_some()
        }
        "istd" => return v.is_empty() || flag(v).map(|b| c.istd = b).is_some(),
        "reiniezione" => return v.is_empty() || flag(v).map(|b| c.reinjection = b).is_some(),
        "replica" => return v.is_empty() || whole(v).map(|n| c.replicate = n).is_some(),
        "ordine_iniezione" => return v.is_empty() || whole(v).map(|n| c.order = n).is_some(),
        "concentrazione" => return v.is_empty() || num(v).map(|x| c.conc = Some(x)).is_some(),
        "dose" => {
            return v.is_empty()
                || dose(v)
                    .map(|(x, u)| {
                        c.conc = Some(x);
                        c.unit = u;
                    })
                    .is_some()
        }
        _ => {}
    }
    true
}

/// The positions of the columns of the times in the header.
struct TimeCols {
    unit: Option<usize>,
    time: Option<usize>,
    minutes: Option<usize>,
    confirmed: Option<usize>,
}

/// The times of one row: `tempo` + `unita_tempo`, else `tempo_min`; confirmed as written, else when there is one.
fn time_of<'a>(
    c: &mut Campione,
    row: usize,
    tc: &TimeCols,
    cell: &dyn Fn(usize) -> &'a str,
) -> Result<()> {
    let get = |i: Option<usize>| i.map(cell).filter(|v| !v.is_empty());
    let unit = match get(tc.unit) {
        Some(u) => unit_of(u).ok_or_else(|| bad(row, "unita_tempo"))?,
        None => "min",
    };
    let (t, u, col) = match (get(tc.time), get(tc.minutes)) {
        (Some(t), _) => (Some(t), unit, "tempo"),
        (None, Some(t)) => (Some(t), "min", "tempo_min"),
        _ => (None, unit, "tempo"),
    };
    let t = t.map(|t| num(t).ok_or_else(|| bad(row, col))).transpose()?;
    c.set_time(t, u);
    if c.kind == "t0" && t.is_none() {
        c.time_min = Some(0.0);
    }
    c.time_confirmed = match get(tc.confirmed) {
        Some(v) => flag(v).ok_or_else(|| bad(row, "tempo_confermato"))?,
        None => t.is_some(),
    };
    Ok(())
}

/// Reads `text`; `sep` None = detected; `declared` gives the type of the free columns it names (the others are inferred).
pub fn leggi(text: &str, sep: Option<char>, declared: &[Colonna]) -> Result<Tabella> {
    let recs = records(text, sep.unwrap_or_else(|| detect(text)))?;
    let Some((head, rows)) = recs.split_first() else {
        return Ok(Tabella {
            campioni: Vec::new(),
            colonne: declared.to_vec(),
        });
    };
    // each header: Ok(fixed field) or Err(free column name)
    let mut heads: Vec<std::result::Result<&'static str, String>> = Vec::new();
    for (h, _) in head {
        let f = field_of(h).ok_or_else(|| h.trim().to_string());
        if f.as_ref().err().is_some_and(|n| n.is_empty()) || heads.contains(&f) {
            return Err(bad(1, h));
        }
        heads.push(f);
    }
    let mut free: Vec<(String, Vec<Option<&str>>)> = (heads.iter())
        .filter_map(|h| h.as_ref().err())
        .map(|h| (h.clone(), Vec::new()))
        .collect();
    let mut cs = Vec::with_capacity(rows.len());
    let mut written: Vec<Option<u32>> = Vec::with_capacity(rows.len());
    let at = |f: &str| heads.iter().position(|h| h.as_ref().ok() == Some(&f));
    let tc = TimeCols {
        unit: at("unita_tempo"),
        time: at("tempo"),
        minutes: at("tempo_min"),
        confirmed: at("tempo_confermato"),
    };
    let replica = at("replica");
    // the free column of each header position (the names are distinct, in header order)
    let free_at: Vec<usize> = heads
        .iter()
        .scan(0, |k, h| {
            let i = *k;
            *k += usize::from(h.is_err());
            Some(i)
        })
        .collect();
    for (r, cells) in rows.iter().enumerate() {
        let row = r + 2;
        if cells.len() > head.len() {
            return Err(bad(row, ""));
        }
        let cell = |i: usize| cells.get(i).map(|c| c.0.as_ref()).unwrap_or("");
        let mut c = Campione::new("", "", r as u32 + 1);
        for (i, h) in heads.iter().enumerate() {
            match h {
                Ok(f)
                    if !matches!(
                        *f,
                        "tempo" | "unita_tempo" | "tempo_confermato" | "tempo_min"
                    ) =>
                {
                    if !put(&mut c, f, cell(i)) {
                        return Err(bad(row, f));
                    }
                }
                Err(_) => {
                    let q = cells.get(i).is_some_and(|c| c.1);
                    free[free_at[i]]
                        .1
                        .push((!cell(i).is_empty() || q).then(|| cell(i)));
                }
                _ => {}
            }
        }
        time_of(&mut c, row, &tc, &cell)?;
        written.push(replica.and_then(|i| whole(cell(i))));
        cs.push(c);
    }
    number_replicates(&mut cs);
    for (c, w) in cs.iter_mut().zip(written) {
        if let Some(n) = w {
            c.replicate = n;
        }
    }
    let colonne = place_free(&mut cs, &free, declared);
    Ok(Tabella {
        campioni: cs,
        colonne,
    })
}

/// Gives each free column its type (declared, else inferred) and stores its values as extras; returns the columns of the table.
fn place_free(
    cs: &mut [Campione],
    free: &[(String, Vec<Option<&str>>)],
    declared: &[Colonna],
) -> Vec<Colonna> {
    let mut colonne = declared.to_vec();
    for (name, values) in free {
        let col = match declared.iter().find(|d| &d.name == name) {
            Some(d) => d.clone(),
            None => {
                let c = Colonna::new(name, infer(values));
                colonne.push(c.clone());
                c
            }
        };
        for (c, v) in cs.iter_mut().zip(values) {
            if let Some(v) = v {
                let cell = match (col.kind, num(v)) {
                    (TipoColonna::Numero, Some(x)) => Cell::Numero(x),
                    _ => Cell::Testo((*v).to_string()),
                };
                c.set_extra(name, cell);
            }
        }
    }
    colonne
}

fn quote(s: &str, sep: char, force: bool) -> String {
    let t = s.trim();
    if force || s.contains([sep, '"', '\n', '\r']) || t.len() != s.len() {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn fixed(c: &Campione, f: &str) -> String {
    let n = |x: Option<f64>| x.map(|v| format!("{v}")).unwrap_or_default();
    match f {
        "file" => c.file.clone(),
        "sha256" => c.sha256.clone(),
        "campione" => c.sub.clone(),
        "campione_id" => c.sample_id.clone(),
        "polarita" => c.polarity.map(|p| p.to_string()).unwrap_or_default(),
        "matrice" => c.matrix.clone(),
        "lotto" => c.batch.clone(),
        "acquisito" => c.acquired.clone().unwrap_or_default(),
        "gruppo" => c.group.clone(),
        "tipo" => c.kind.clone(),
        "tipo_bianco" => c.blank_kind.clone().unwrap_or_default(),
        "tempo" => n(c.time),
        "unita_tempo" => c.time_unit.clone(),
        "tempo_confermato" => c.time_confirmed.to_string(),
        "serie" => c.series.clone(),
        "reiniezione" => c.reinjection.to_string(),
        "replica" => c.replicate.to_string(),
        "concentrazione" => n(c.conc),
        "unita" => c.unit.clone().unwrap_or_default(),
        "ordine_iniezione" => c.order.to_string(),
        "istd" => c.istd.to_string(),
        "note" => c.note.clone(),
        _ => c.compound.clone(),
    }
}

/// The table as CSV, the files in the order given, the free columns sorted by name after the fixed ones; `\n` ends every line.
pub fn scrivi(t: &Tabella, sep: char) -> String {
    let mut names: Vec<&str> = t.colonne.iter().map(|c| c.name.as_str()).collect();
    names.extend(
        t.campioni
            .iter()
            .flat_map(|c| c.extra.iter().map(|(k, _)| k.as_str())),
    );
    crate::sortc::slice_ord(&mut names);
    names.dedup();
    let sep_s = sep.to_string();
    let head: Vec<String> = FIXED
        .iter()
        .chain(names.iter())
        .map(|h| quote(h, sep, false))
        .collect();
    let mut out = head.join(&sep_s);
    for c in &t.campioni {
        let mut cells: Vec<String> = FIXED
            .iter()
            .map(|f| quote(&fixed(c, f), sep, false))
            .collect();
        for n in &names {
            cells.push(match c.extra_of(n) {
                None => String::new(),
                Some(Cell::Numero(x)) => format!("{x}"),
                Some(Cell::Testo(s)) => quote(s, sep, s.is_empty()),
            });
        }
        out.push('\n');
        out.push_str(&cells.join(&sep_s));
    }
    out.push('\n');
    out
}

fn sep_of(v: &crate::json::Value) -> Result<Option<char>> {
    match v.get("separatore").and_then(|s| s.str()) {
        None | Some("") => Ok(None),
        Some(",") => Ok(Some(',')),
        Some(";") => Ok(Some(';')),
        Some("\t") | Some("tab") => Ok(Some('\t')),
        Some(_) => Err(Error::new("error.badCsv")),
    }
}

/// `{"testo", "separatore"?, "colonne"?: [declared columns]}` → a `campioni/3`.
pub fn csv_leggi(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let text = v
        .get("testo")
        .and_then(|x| x.str())
        .ok_or_else(|| Error::new("error.badCsv"))?;
    let declared = match v.get("colonne") {
        Some(c) => {
            let mut o = Out::default();
            crate::json::write_value(&mut o, c);
            let doc = format!(
                "{{\"campioni\":[],\"colonne\":{},\"schema\":\"campioni/3\"}}",
                o.0
            );
            tabella_da_value(&parse(doc.as_bytes())?)?.colonne
        }
        None => Vec::new(),
    };
    Ok(to_json3(&leggi(text, sep_of(&v)?, &declared)?))
}

/// `{"campioni": <campioni/2 or /3>, "separatore"?}` → `{"testo": "..."}`.
pub fn csv_scrivi(input: &str) -> Result<String> {
    let v = parse(input.as_bytes())?;
    let doc = v
        .get("campioni")
        .ok_or_else(|| Error::new("error.badCsv"))?;
    let t = tabella_da_value(doc)?;
    let text = scrivi(&t, sep_of(&v)?.unwrap_or(','));
    let mut j = Out::default();
    j.raw("{\"testo\":").str(&text).raw("}");
    Ok(j.0)
}

#[cfg(test)]
mod prove {
    use super::*;

    /// The first parser, char by char, kept as the reference the byte scanner is compared with.
    /// (was) quoted fields keep their separators and newlines (`""` is a quote); a field is trimmed unless quoted.
    fn records_ref(text: &str, sep: char) -> Result<Vec<Vec<(String, bool)>>> {
        let mut out: Vec<Vec<(String, bool)>> = Vec::new();
        let (mut row, mut cur, mut quoted, mut inq) =
            (Vec::<(String, bool)>::new(), String::new(), false, false);
        let mut it = text.trim_start_matches('\u{feff}').chars().peekable();
        let end = |row: &mut Vec<(String, bool)>, cur: &mut String, quoted: &mut bool| {
            let v = if *quoted {
                std::mem::take(cur)
            } else {
                cur.trim().to_string()
            };
            cur.clear();
            row.push((v, std::mem::take(quoted)));
        };
        while let Some(c) = it.next() {
            if inq {
                match (c, it.peek()) {
                    ('"', Some('"')) => {
                        cur.push('"');
                        it.next();
                    }
                    ('"', _) => inq = false,
                    _ => cur.push(c),
                }
            } else if c == '"' && cur.trim().is_empty() {
                cur.clear();
                inq = true;
                quoted = true;
            } else if c == sep {
                end(&mut row, &mut cur, &mut quoted);
            } else if c == '\n' || c == '\r' {
                if c == '\r' && it.peek() == Some(&'\n') {
                    it.next();
                }
                end(&mut row, &mut cur, &mut quoted);
                let blank = row.len() == 1 && row[0].0.is_empty() && !row[0].1;
                let r = std::mem::take(&mut row);
                if !blank {
                    out.push(r);
                }
            } else {
                cur.push(c);
            }
        }
        if inq {
            return Err(bad(out.len() + 1, ""));
        }
        if !cur.is_empty() || !row.is_empty() {
            end(&mut row, &mut cur, &mut quoted);
            out.push(row);
        }
        Ok(out)
    }

    fn owned(r: Result<Vec<Cells<'_>>>) -> Result<Vec<Vec<(String, bool)>>> {
        r.map(|rows| {
            rows.into_iter()
                .map(|r| r.into_iter().map(|(v, q)| (v.into_owned(), q)).collect())
                .collect()
        })
    }

    fn same(text: &str, sep: char) {
        assert_eq!(
            owned(records(text, sep)),
            records_ref(text, sep),
            "{text:?} sep {sep:?}"
        );
    }

    /// Every text up to 6 symbols of an alphabet made of the awkward ones, for the three separators.
    #[test]
    fn byte_scanner_equals_char_parser_on_every_short_text() {
        let alpha = ['a', ' ', '"', ',', ';', '\n', '\r', 'é'];
        for sep in [',', ';', '\t'] {
            let mut texts = vec![String::new()];
            let mut level = vec![String::new()];
            for _ in 0..6 {
                level = level
                    .iter()
                    .flat_map(|t| alpha.iter().map(move |c| format!("{t}{c}")))
                    .collect();
                texts.extend(level.iter().cloned());
            }
            for t in &texts {
                same(t, sep);
            }
        }
    }

    #[test]
    fn byte_scanner_equals_char_parser_on_random_texts() {
        let alpha = [
            'a', 'b', ' ', ' ', '"', '"', ',', ';', '\t', '\n', '\r', 'é', '€', '\u{a0}',
            '\u{feff}',
        ];
        let mut s = 0x9e3779b97f4a7c15u64;
        for _ in 0..200_000 {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let len = (s >> 60) as usize + (s >> 20 & 0x1f) as usize;
            let mut t = String::new();
            let mut x = s;
            for _ in 0..len {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                t.push(alpha[(x >> 33) as usize % alpha.len()]);
            }
            same(&t, [',', ';', '\t', 'é'][(s >> 7) as usize % 4]);
        }
    }

    #[test]
    fn records_of_the_awkward_cases() {
        let cells = |t: &str| owned(records(t, ',')).unwrap();
        let c = |v: &str, q: bool| (v.to_string(), q);
        assert_eq!(
            cells("a,\"x,y\",c\n"),
            vec![vec![c("a", false), c("x,y", true), c("c", false)]]
        );
        assert_eq!(
            cells("\"l1\nl2\",z"),
            vec![vec![c("l1\nl2", true), c("z", false)]]
        );
        assert_eq!(
            cells("\"say \"\"hi\"\"\""),
            vec![vec![c("say \"hi\"", true)]]
        );
        assert_eq!(
            cells("ab\"c,d"),
            vec![vec![c("ab\"c", false), c("d", false)]]
        );
        assert_eq!(
            cells("\"abc\" ,d"),
            vec![vec![c("abc ", true), c("d", false)]]
        );
        assert_eq!(
            cells("a\rb\r\nc\n\n\nd"),
            vec![
                vec![c("a", false)],
                vec![c("b", false)],
                vec![c("c", false)],
                vec![c("d", false)]
            ]
        );
        assert_eq!(
            cells("\u{feff}a,é€\n"),
            vec![vec![c("a", false), c("é€", false)]]
        );
        assert_eq!(
            cells("  x  ,\"  y  \""),
            vec![vec![c("x", false), c("  y  ", true)]]
        );
        assert!(records("\"open", ',').is_err());
    }

    /// 10^5 rows of 12 columns: the two parsers, best of 5.
    #[test]
    #[ignore = "timing"]
    fn timing_of_the_parsers() {
        let mut t = String::from("file,sha256,campione_id,polarita,gruppo,tipo,tempo,unita_tempo,replica,concentrazione,unita,note\n");
        for i in 0..100_000 {
            t += &format!(
                "run_{i:06}.mzML,{:064x},S{i},pos,g{},sample,{}.5,min,{},{}.25,ppm,\"note, {i}\"\n",
                i * 7919,
                i % 9,
                i % 40,
                i % 3 + 1,
                i % 50
            );
        }
        let best = |f: &dyn Fn() -> usize| {
            (0..5)
                .map(|_| {
                    let t0 = std::time::Instant::now();
                    let n = f();
                    assert_eq!(n, 100_001);
                    t0.elapsed().as_secs_f64() * 1e3
                })
                .fold(f64::MAX, f64::min)
        };
        let old = best(&|| records_ref(&t, ',').unwrap().len());
        let new = best(&|| records(&t, ',').unwrap().len());
        let all = best(&|| leggi(&t, None, &[]).unwrap().campioni.len() + 1);
        eprintln!("records old {old:.2} ms · new {new:.2} ms · leggi (whole) {all:.2} ms");
    }
}
