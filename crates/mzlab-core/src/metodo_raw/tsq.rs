//! The readable method of a Thermo triple quadrupole (TSQ Quantum, Vantage, Endura, Quantiva, Altis, Quantis): the `Experiment Type` / `Label (unit): value`
//! lines before the transition table, and the table itself (`Compound Name | Start Time (min) | … | Precursor (m/z) | Product (m/z) | Collision Energy (V) | …`).
//! The columns differ between models (`RF Lens`, `RF Lens (V)`, `Min Dwell Time (ms)`), so each is found by the name of its header; a column the table lacks is absent.
//! Only the layout seen in the files read so far is decoded: a text with a transition table that does not parse gives `None` (the caller warns).

use crate::metodo::{num, obj, text, V};

/// One row of the transition table.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    pub name: String,
    pub start_min: Option<f64>,
    pub end_min: Option<f64>,
    /// `"positive"` or `"negative"` as the table spells it.
    pub polarity: Option<&'static str>,
    pub q1: f64,
    pub q3: f64,
    pub ce_ev: Option<f64>,
    /// RF lens (or tube lens) voltage.
    pub rf_lens_v: Option<f64>,
    pub dwell_ms: Option<f64>,
}

/// A numeric setting before the table: `id`, label as written, value, unit.
pub type Setting = (String, String, f64, String);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    /// `Experiment Type: SRM`.
    pub kind: Option<String>,
    pub settings: Vec<Setting>,
    pub rows: Vec<Row>,
    /// The text ends inside a row: the file keeps only the first bytes of the method summary, so the rows after the cut are not there.
    pub cut: bool,
}

fn number(cell: &str) -> Option<f64> {
    let t: String = cell.trim().chars().filter(|c| *c != ',').collect();
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// The column of a header cell (`Precursor (m/z)` → `Col::Q1`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Col {
    Name,
    Start,
    End,
    Polarity,
    Q1,
    Q3,
    Ce,
    Rf,
    Dwell,
    Other,
}

fn column(cell: &str) -> Col {
    let c = cell.trim().to_ascii_lowercase();
    if c.starts_with("compound") {
        Col::Name
    } else if c.starts_with("start time") {
        Col::Start
    } else if c.starts_with("end time") {
        Col::End
    } else if c.starts_with("polarity") {
        Col::Polarity
    } else if c.starts_with("precursor") {
        Col::Q1
    } else if c.starts_with("product") {
        Col::Q3
    } else if c.starts_with("collision energy") {
        Col::Ce
    } else if c.starts_with("rf lens") || c.starts_with("tube lens") {
        Col::Rf
    } else if c.contains("dwell") {
        Col::Dwell
    } else {
        Col::Other
    }
}

fn slug(label: &str) -> String {
    let mut out = String::new();
    for c in label.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_string()
}

/// `Label (unit): value` of a line before the table, when the value is a number.
fn setting(line: &str) -> Option<Setting> {
    let (label, value) = line.split_once(':')?;
    let value = number(value)?;
    let label = label.trim();
    let (name, unit) = match label.rfind('(').filter(|_| label.ends_with(')')) {
        Some(p) => (label.get(..p)?.trim(), label.get(p + 1..label.len() - 1)?),
        None => (label, ""),
    };
    (!name.is_empty() && name.chars().next()?.is_alphabetic())
        .then(|| (slug(name), name.to_string(), value, unit.trim().to_string()))
}

/// The table and the settings of the readable method `t` (`None`: no transition table). The first line that looks like a header may be the torn end of
/// another text (the extraction glues blocks), so the headers are tried in turn until one is followed by rows.
pub fn parse(t: &str) -> Option<Table> {
    let lines: Vec<&str> = t.lines().map(|l| l.trim_end_matches('\r')).collect();
    (0..lines.len())
        .filter(|&i| {
            let l = lines[i].to_ascii_lowercase();
            l.contains('|') && l.contains("precursor") && l.contains("product")
        })
        .find_map(|head| table_at(&lines, head))
}

fn table_at(lines: &[&str], head: usize) -> Option<Table> {
    let cols: Vec<Col> = lines.get(head)?.split('|').map(column).collect();
    if !cols.contains(&Col::Q1) || !cols.contains(&Col::Q3) {
        return None;
    }
    let mut table = Table::default();
    for l in lines.get(..head)? {
        if let Some(rest) = l.trim().strip_prefix("Experiment Type:") {
            table.kind = text(rest).and_then(|v| match v {
                V::Str(s) => Some(s),
                _ => None,
            });
        } else if let Some(s) = setting(l) {
            table.settings.push(s);
        }
    }
    for (k, l) in lines.iter().enumerate().skip(head + 1) {
        let cells: Vec<&str> = l.split('|').collect();
        // a row has one cell per column and a closing bar: the first line that does not is the end of the table
        if !l.contains('|') || cells.len() < cols.len() {
            table.cut = l.contains('|') && k + 1 == lines.len();
            break;
        }
        let mut row = Row::default();
        let (mut q1, mut q3) = (None, None);
        for (c, cell) in cols.iter().zip(&cells) {
            match c {
                Col::Name => row.name = cell.trim().to_string(),
                Col::Start => row.start_min = number(cell),
                Col::End => row.end_min = number(cell),
                Col::Polarity => {
                    row.polarity = match cell.trim().to_ascii_lowercase().as_str() {
                        "positive" => Some("positive"),
                        "negative" => Some("negative"),
                        _ => None,
                    }
                }
                Col::Q1 => q1 = number(cell),
                Col::Q3 => q3 = number(cell),
                Col::Ce => row.ce_ev = number(cell),
                Col::Rf => row.rf_lens_v = number(cell),
                Col::Dwell => row.dwell_ms = number(cell),
                Col::Other => {}
            }
        }
        let (Some(q1), Some(q3)) = (q1, q3) else {
            break;
        };
        row.q1 = q1;
        row.q3 = q3;
        table.rows.push(row);
    }
    (!table.rows.is_empty()).then_some(table)
}

/// True when the text says it holds SRM transitions (`Experiment Type: SRM` or a table header with a precursor column): used to warn when `parse` finds none.
pub fn announces_transitions(t: &str) -> bool {
    t.lines().any(|l| {
        let l = l.to_ascii_lowercase();
        l.trim().starts_with("experiment type: srm")
            || l.trim().starts_with("experiment type: crm")
            || (l.contains('|') && l.contains("precursor") && l.contains("product"))
    })
}

/// The experiment of a TSQ method for `ms.periodi[0].esperimenti`: the type, the settings before the table and the transitions.
pub fn experiment(t: &Table, index: usize) -> Option<V> {
    let params: Vec<V> = t
        .settings
        .iter()
        .filter_map(|(id, label, value, unit)| {
            obj(vec![
                ("id", text(id)),
                ("etichetta", text(label)),
                ("valore", num(Some(*value))),
                ("unita", text(unit)),
            ])
        })
        .collect();
    let transitions: Vec<V> = t
        .rows
        .iter()
        .filter_map(|r| {
            obj(vec![
                ("q1", num(Some(r.q1))),
                ("q3", num(Some(r.q3))),
                ("dwell_ms", num(r.dwell_ms)),
                ("ce_eV", num(r.ce_ev)),
                ("rf_lens_V", num(r.rf_lens_v)),
                ("polarita", r.polarity.map(V::s)),
                ("t_inizio_min", num(r.start_min)),
                ("t_fine_min", num(r.end_min)),
                ("nome", text(&r.name)),
            ])
        })
        .collect();
    obj(vec![
        ("indice", num(Some(index as f64))),
        ("tipo", t.kind.as_deref().and_then(text)),
        ("parametri", Some(V::Arr(params))),
        ("transizioni", Some(V::Arr(transitions))),
    ])
}

#[cfg(test)]
mod tests;
