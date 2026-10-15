//! The inclusion list of a Thermo Exactive method: the table that follows the line `INCLUSION LIST` in the readable text of the method (mass, formula, species,
//! charge, polarity, start and end of the time window, collision energy, name). The columns are found by the name of their header, so a column the table lacks is
//! absent. A list is only a description of what the method asks the instrument to fragment: it is shown as the file says it, never turned into expected products.

use crate::metodo::{num, obj, text, V};

/// One entry of the list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entry {
    pub mz: f64,
    pub formula: String,
    pub species: String,
    pub charge: Option<f64>,
    pub polarity: Option<&'static str>,
    pub start_min: Option<f64>,
    pub end_min: Option<f64>,
    /// Normalised collision energy: one value, or the steps of a stepped energy (`20, 35, 60`).
    pub nce: Vec<f64>,
    pub name: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Col {
    Mass,
    Formula,
    Species,
    Charge,
    Polarity,
    Start,
    End,
    Nce,
    Name,
    Other,
}

fn column(cell: &str) -> Col {
    let c = cell.trim().to_ascii_lowercase();
    if c.starts_with("mass") || c.starts_with("m/z") {
        Col::Mass
    } else if c.starts_with("formula") {
        Col::Formula
    } else if c.starts_with("species") {
        Col::Species
    } else if c.starts_with("cs") || c.starts_with("charge") || c == "z" {
        Col::Charge
    } else if c.starts_with("polarity") {
        Col::Polarity
    } else if c.starts_with("start") {
        Col::Start
    } else if c.starts_with("end") {
        Col::End
    } else if c.contains("nce") || c.starts_with("ce") || c.starts_with("(n)ce") {
        Col::Nce
    } else if c.starts_with("comment") || c.starts_with("name") || c.starts_with("compound") {
        Col::Name
    } else {
        Col::Other
    }
}

fn number(cell: &str) -> Option<f64> {
    cell.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// The numbers of a cell written as a list (`20, 35, 60`, `20;35;60`, `30`); a comma not followed by a blank is a decimal comma. Anything else in the cell gives none.
pub fn number_list(cell: &str) -> Vec<f64> {
    let parts: Vec<&str> = cell
        .split(';')
        .flat_map(|p| p.split(", "))
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let list: Vec<f64> = parts.iter().filter_map(|p| number(p)).collect();
    if list.len() == parts.len() {
        list
    } else {
        Vec::new()
    }
}

fn cells(line: &str) -> Vec<&str> {
    let sep = if line.contains('|') { '|' } else { '\t' };
    let mut v: Vec<&str> = line.split(sep).map(str::trim).collect();
    if v.first() == Some(&"") && sep == '|' {
        v.remove(0);
    }
    if v.last() == Some(&"") {
        v.pop();
    }
    v
}

/// The entries of the inclusion list in `t` (`None`: no such table).
pub fn parse(t: &str) -> Option<Vec<Entry>> {
    let lines: Vec<&str> = t.lines().map(|l| l.trim_end_matches('\r')).collect();
    let at = lines
        .iter()
        .position(|l| l.to_ascii_uppercase().contains("INCLUSION LIST"))?;
    let (head, cols) = (at + 1..lines.len().min(at + 6)).find_map(|i| {
        let l = lines[i];
        if !(l.contains('|') || l.contains('\t')) {
            return None;
        }
        let cols: Vec<Col> = cells(l).into_iter().map(column).collect();
        cols.contains(&Col::Mass).then_some((i, cols))
    })?;
    let mut out = Vec::new();
    for l in lines.iter().skip(head + 1) {
        let c = cells(l);
        if !(l.contains('|') || l.contains('\t')) || c.len() < cols.len() {
            break;
        }
        let mut e = Entry::default();
        let mut mz = None;
        for (col, cell) in cols.iter().zip(&c) {
            match col {
                Col::Mass => mz = number(cell),
                Col::Formula => e.formula = cell.to_string(),
                Col::Species => e.species = cell.to_string(),
                Col::Charge => e.charge = number(cell),
                Col::Polarity => {
                    e.polarity = match cell.to_ascii_lowercase().as_str() {
                        "positive" | "pos" | "+" => Some("positive"),
                        "negative" | "neg" | "-" => Some("negative"),
                        _ => None,
                    }
                }
                Col::Start => e.start_min = number(cell),
                Col::End => e.end_min = number(cell),
                Col::Nce => e.nce = number_list(cell),
                Col::Name => e.name = cell.to_string(),
                Col::Other => {}
            }
        }
        let Some(mz) = mz else { break };
        e.mz = mz;
        out.push(e);
    }
    (!out.is_empty()).then_some(out)
}

/// The list as `inclusione` of an experiment.
pub fn section(list: &[Entry]) -> V {
    V::Arr(
        list.iter()
            .filter_map(|e| {
                obj(vec![
                    ("mz", num(Some(e.mz))),
                    ("formula", text(&e.formula)),
                    ("specie", text(&e.species)),
                    ("carica", num(e.charge)),
                    ("polarita", e.polarity.map(V::s)),
                    ("t_inizio_min", num(e.start_min)),
                    ("t_fine_min", num(e.end_min)),
                    (
                        "nce",
                        Some(V::Arr(e.nce.iter().map(|x| V::Num(*x)).collect())),
                    ),
                    ("nome", text(&e.name)),
                ])
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests;
