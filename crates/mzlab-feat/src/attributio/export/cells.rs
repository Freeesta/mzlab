//! Sheets of cells that the exports of Attributio share: a name, a header and typed rows, rendered as the rows of a spreadsheet (`{"hojas":[...]}`, the model of
//! `mzlab_feat::export::xlsx_rows`, zipped into an XLSX by the page) or as a CSV with `;`.

use crate::attributio::frame::Cell;
use mzlab_core::json::Out;

/// One sheet: header and rows (a row has as many cells as the header).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub head: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}

impl Sheet {
    pub fn new(name: &str, head: &[&str]) -> Sheet {
        Sheet {
            name: name.to_string(),
            head: head.iter().map(|h| h.to_string()).collect(),
            rows: Vec::new(),
        }
    }

    /// The same sheet with the header given as text.
    pub fn with_head(name: &str, head: Vec<String>) -> Sheet {
        Sheet {
            name: name.to_string(),
            head,
            rows: Vec::new(),
        }
    }

    /// Text with `;` between cells, UTF-8, «\n» line ends; a cell with `;`, a quote or a line break goes between quotes; missing cells are empty.
    pub fn csv(&self) -> String {
        let q = |s: &str| {
            if s.contains([';', '"', '\r', '\n']) {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };
        let mut out = self.head.iter().map(|h| q(h)).collect::<Vec<_>>().join(";");
        out.push('\n');
        for row in &self.rows {
            let cells: Vec<String> = row.iter().map(|c| q(&cell_text(c))).collect();
            out.push_str(&cells.join(";"));
            out.push('\n');
        }
        out
    }

    /// `{"name":...,"head":[...],"rows":[[...]]}`: the form the golden data and the tests compare.
    pub fn json(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"head\":[");
        for (i, h) in self.head.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(h);
        }
        o.raw("],\"name\":").str(&self.name).raw(",\"rows\":[");
        for (i, r) in self.rows.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            row_json(&mut o, r);
        }
        o.raw("]}");
        o.0
    }
}

/// The text of a cell for a CSV: numbers as Python shows a float (`4.0`, `1e-05`), `true`/`false`, nothing for a missing cell.
pub fn cell_text(c: &Cell) -> String {
    match c {
        Cell::Null => String::new(),
        Cell::Bool(b) => b.to_string(),
        Cell::Int(i) => i.to_string(),
        Cell::Num(v) => super::pyfmt::py_float(*v),
        Cell::Str(s) => s.clone(),
    }
}

fn cell_json(o: &mut Out, c: &Cell) {
    match c {
        Cell::Null => o.raw("null"),
        Cell::Bool(b) => o.raw(if *b { "true" } else { "false" }),
        Cell::Int(i) => o.int(*i),
        Cell::Num(v) => o.opt(Some(*v).filter(|v| v.is_finite())),
        Cell::Str(s) => o.str(s),
    };
}

fn row_json(o: &mut Out, row: &[Cell]) {
    o.raw("[");
    for (i, c) in row.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        cell_json(o, c);
    }
    o.raw("]");
}

/// `{"hojas":[{"nombre":...,"filas":[[header],[row]...]}...]}` one row at a time through `emit`.
pub fn xlsx_rows_to(sheets: &[Sheet], emit: &mut dyn FnMut(&str)) {
    let mut o = Out::default();
    o.raw("{\"hojas\":[");
    for (k, s) in sheets.iter().enumerate() {
        o.raw(if k > 0 {
            ",{\"nombre\":"
        } else {
            "{\"nombre\":"
        })
        .str(&s.name)
        .raw(",\"filas\":[[");
        for (i, h) in s.head.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(h);
        }
        o.raw("]");
        emit(&std::mem::take(&mut o.0));
        for r in &s.rows {
            o.raw(",");
            row_json(&mut o, r);
            emit(&std::mem::take(&mut o.0));
        }
        o.raw("]}");
    }
    o.raw("]}");
    emit(&o.0);
}

/// `xlsx_rows_to` as one text.
pub fn xlsx_rows(sheets: &[Sheet]) -> String {
    let mut out = String::new();
    xlsx_rows_to(sheets, &mut |t| out.push_str(t));
    out
}
