//! `tray/1`: the structure tray of Draw, structures kept with a name and written out as a suspect list (CSV with `name`, `smiles`,
//! `formula`, `mass`, `charge`, `adducts`, the columns `screen.suspects` and mzFinder read), and read back from such a file. Formula and
//! monoisotopic mass come from the drawn atoms (`etich/1`, isotopes kept); the adducts are names of the m/z calculator catalogue. A suspect
//! list is a list of candidates to look for, never of what a sample contains.

use mzlab_core::adduct_table;
use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};

use super::labels;

/// The adducts ticked when the tray is first shown: one per polarity.
const ON: [&str; 2] = ["mh", "mminush"];
/// The header of the written list; the reader takes the columns by name, in any order.
const HEAD: &str = "name,smiles,formula,mass,charge,adducts";

fn bad() -> Error {
    Error::new("error.tray.request")
}

fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::str).unwrap_or("")
}

/// The adducts a list may name: the catalogue without `[M]+` and `[M]-` (a charged structure is its own ion).
fn adducts() -> impl Iterator<Item = adduct_table::Row> {
    adduct_table::catalog().filter(|(_, _, _, add, sub, _, _)| !(add.is_empty() && sub.is_empty()))
}

fn adducts_json(o: &mut Out) {
    o.raw("{\"format\":\"tray/1\",\"adducts\":[");
    for (i, (name, id, _, _, _, z, common)) in adducts().enumerate() {
        o.raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(name)
            .raw(",\"id\":")
            .str(id)
            .raw(",\"pol\":")
            .int(i64::from(z.signum()))
            .raw(",\"common\":")
            .raw(if common { "true" } else { "false" })
            .raw(",\"on\":")
            .raw(if ON.contains(&id) { "true" } else { "false" })
            .raw("}");
    }
    o.raw("]}");
}

/// One entry for the tray: `{name, smiles, formula, charge, mass}` from the atoms, or `{error, label}` when an atom is no element.
fn entries_json(o: &mut Out, v: &Value) {
    o.raw("{\"format\":\"tray/1\",\"entries\":[");
    for (i, st) in v
        .get("structures")
        .map_or(&[][..], Value::arr)
        .iter()
        .enumerate()
    {
        o.raw(if i > 0 { "," } else { "" });
        let atoms: Vec<_> = st
            .get("atoms")
            .map_or(&[][..], Value::arr)
            .iter()
            .map(labels::atom_in)
            .collect();
        let shape = if atoms.is_empty() {
            Err(Error::new("error.ions.empty"))
        } else {
            labels::shape(&atoms)
        };
        match shape {
            Ok(s) => {
                o.raw("{\"name\":")
                    .str(text(st, "name").trim())
                    .raw(",\"smiles\":")
                    .str(text(st, "smiles").trim())
                    .raw(",\"formula\":")
                    .str(&labels::hill(&s.n))
                    .raw(",\"charge\":")
                    .int(s.q)
                    .raw(",\"mass\":")
                    .num(round(s.mass, 6))
                    .raw("}");
            }
            Err(e) => {
                let label = e
                    .params
                    .iter()
                    .find(|(k, _)| *k == "label")
                    .map_or("", |(_, v)| v.as_str());
                o.raw("{\"error\":")
                    .str(e.key)
                    .raw(",\"label\":")
                    .str(label)
                    .raw("}");
            }
        }
    }
    o.raw("]}");
}

/// A CSV field, quoted when it holds a comma, a quote or a line break (RFC 4180).
fn field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// A CSV line split at `,`; a field in double quotes may hold commas and `""` (the rule of the `screen.suspects` reader).
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

/// The suspect list of `entries` with the chosen `adducts` (catalogue names, `;` between them), one line per entry, `\n` line ends.
fn csv(v: &Value) -> Result<String> {
    let names: Vec<&str> = v
        .get("adducts")
        .map_or(&[][..], Value::arr)
        .iter()
        .filter_map(Value::str)
        .collect();
    if let Some(x) = names.iter().find(|n| !adducts().any(|a| a.0 == **n)) {
        return Err(Error::new("error.tray.adduct").with("adduct", x));
    }
    let rows = v.get("entries").map_or(&[][..], Value::arr);
    if rows.is_empty() {
        return Err(Error::new("error.tray.empty"));
    }
    let mut out = format!("{HEAD}\n");
    for (i, e) in rows.iter().enumerate() {
        let formula = text(e, "formula");
        let mass = e
            .get("mass")
            .and_then(Value::f64)
            .or_else(|| {
                elements::parse_formula(formula)
                    .ok()
                    .map(|f| elements::mass(&f))
            })
            .filter(|m| m.is_finite() && *m > 0.0)
            .ok_or_else(|| Error::new("error.tray.mass").with("n", &(i + 1).to_string()))?;
        let charge = e.get("charge").and_then(Value::f64).unwrap_or(0.0) as i64;
        let name = match text(e, "name").trim() {
            "" => format!("structure {}", i + 1),
            n => n.to_string(),
        };
        out.push_str(&format!(
            "{},{},{},{mass:.6},{charge},{}\n",
            field(&name),
            field(text(e, "smiles").trim()),
            field(formula),
            field(&names.join(";"))
        ));
    }
    Ok(out)
}

/// The entries of a suspect list (this tray's or another CSV with `name`, `smiles`, `formula` or `mass`): rows without a usable mass are
/// skipped and counted; `adducts` are the catalogue names the rows give, in order of first use.
fn read(o: &mut Out, csv: &str) -> Result<()> {
    let mut lines = csv.lines().filter(|l| !l.trim().is_empty());
    let head: Vec<String> = split(lines.next().unwrap_or("").trim_start_matches('\u{feff}'))
        .iter()
        .map(|h| h.to_ascii_lowercase())
        .collect();
    let col = |names: &[&str]| head.iter().position(|h| names.contains(&h.as_str()));
    let (cn, cs, cf, cm) = (
        col(&["name"]),
        col(&["smiles"]),
        col(&["formula", "neutral_formula"]),
        col(&["mass", "neutral_mass", "monoisotopic_mass"]),
    );
    let (cq, ca) = (col(&["charge"]), col(&["adducts", "adduct"]));
    if cf.is_none() && cm.is_none() {
        return Err(Error::new("error.tray.csv"));
    }
    let (mut seen, mut skipped, mut n) = (Vec::<&str>::new(), 0, 0);
    o.raw("{\"format\":\"tray/1\",\"entries\":[");
    for line in lines {
        let r = split(line);
        let get = |c: Option<usize>| c.and_then(|i| r.get(i)).map_or("", String::as_str);
        let formula = get(cf);
        let mass = get(cm)
            .parse::<f64>()
            .ok()
            .or_else(|| {
                elements::parse_formula(formula)
                    .ok()
                    .map(|f| elements::mass(&f))
            })
            .filter(|m| m.is_finite() && *m > 0.0);
        let Some(mass) = mass else {
            skipped += 1;
            continue;
        };
        for a in get(ca).split(';').map(str::trim) {
            if let Some(row) = adducts().find(|x| x.0 == a) {
                if !seen.contains(&row.0) {
                    seen.push(row.0);
                }
            }
        }
        o.raw(if n > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(get(cn))
            .raw(",\"smiles\":")
            .str(get(cs))
            .raw(",\"formula\":")
            .str(formula)
            .raw(",\"charge\":")
            .int(get(cq).parse::<i64>().unwrap_or(0))
            .raw(",\"mass\":")
            .num(round(mass, 6))
            .raw("}");
        n += 1;
    }
    o.raw("],\"adducts\":[");
    for (i, a) in seen.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(a);
    }
    o.raw("],\"skipped\":").int(skipped).raw("}");
    Ok(())
}

/// `tray/1` of a request: `{"op": "adducts"}` the catalogue adducts a list may name; `{"op": "entries", "structures": [{name, smiles, atoms}]}`
/// the formula and mass of drawn structures; `{"op": "csv", "entries": [{name, smiles, formula, mass, charge}], "adducts": [names]}` the
/// suspect list as CSV text; `{"op": "read", "text": CSV}` its entries back.
pub fn tray_text(request: &str) -> Result<String> {
    let v = json::parse(request.as_bytes()).map_err(|_| bad())?;
    let mut o = Out::default();
    match text(&v, "op") {
        "adducts" => adducts_json(&mut o),
        "entries" => entries_json(&mut o, &v),
        "csv" => return csv(&v),
        "read" => read(&mut o, text(&v, "text"))?,
        _ => return Err(bad()),
    }
    Ok(o.0)
}

#[cfg(test)]
#[path = "tray_tests.rs"]
mod tests;
