//! The files that leave the table, written here so the site and the desktop give the same bytes: the CSV "full" for mzmine (and for readers that follow
//! its columns), the FBMN quantification table and, in `export_ms`, the MGF and the SIRIUS `.ms`. Numbers only: no name, no identification.
//!
//! The rows are those of the view (every row without one) and the areas are the normalised ones when there is a view. A cell with no peak has no area
//! and no height; the state of every cell is written with its exact word, so a reader that counts only `DETECTED` (as Attributio does) never counts a
//! filled or targeted cell.

use crate::aligned::{Aligned, CELL_ABSENT, CELL_DETECTED, CELL_FILLED, CELL_TARGETED};
use crate::norm::measured;
use crate::samples::Sample;
use crate::view::View;

/// What every writer reads.
pub struct Ctx<'a> {
    pub table: &'a Aligned,
    pub samples: &'a [Sample],
    pub view: Option<&'a View>,
    pub table_sha256: &'a str,
    /// Polarity of the table: 1 or -1.
    pub polarity: i8,
}

impl Ctx<'_> {
    /// Rows to write, ascending.
    pub fn rows(&self) -> Vec<usize> {
        self.view.map_or_else(|| (0..self.table.len()).collect(), |v| v.rows.iter().map(|&r| r as usize).collect())
    }

    /// Area of a cell: normalised when there is a view, else as measured; None where the file has no peak.
    pub fn area(&self, row: usize, file: usize) -> Option<f64> {
        match self.view {
            Some(v) => v.area_at(self.table, row, file),
            None => measured(self.table, row, file),
        }
    }

    /// Lines `# ...` that open the formats that allow comments.
    pub fn header(&self) -> Vec<String> {
        crate::report::header_lines(self.table, self.samples, self.view, self.table_sha256)
    }
}

/// `v` rounded to `d` decimals, written without trailing zeros (the way the page writes `+x.toFixed(d)`).
pub(crate) fn fix(v: f64, d: usize) -> String {
    let s = format!("{v:.d$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// `v` with 8 significant digits (the way the page writes `toPrecision(8)`).
pub(crate) fn sig8(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return "0".into();
    }
    format!("{:.7e}", v).parse::<f64>().map_or_else(|_| "0".into(), |x| format!("{x}"))
}

fn quote(s: &str) -> String {
    if s.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// File name without its extension, the name of the sample in the columns.
fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(a, _)| a)
}

fn state_word(state: u8, mzmine: bool) -> &'static str {
    match state {
        CELL_DETECTED => "DETECTED",
        CELL_TARGETED if mzmine => "ESTIMATED",
        CELL_TARGETED => "TARGETED",
        CELL_FILLED if mzmine => "ESTIMATED",
        CELL_FILLED => "FILLED",
        _ => "UNKNOWN",
    }
}

fn write_line(out: &mut String, cells: Vec<String>) {
    out.push_str(&cells.iter().map(|c| quote(c)).collect::<Vec<_>>().join(","));
    out.push('\n');
}

/// The CSV "full" of mzmine: `id, mz, rt, rt_range:min, rt_range:max, charge` and, for every file, `datafile:<F>:area|height|feature_state`.
/// `mzmine_states` writes `ESTIMATED` for the filled and targeted cells, for readers that know only mzmine's words; without it they keep their own.
pub fn csv_full(c: &Ctx, mzmine_states: bool) -> String {
    let t = c.table;
    let mut head: Vec<String> = ["id", "mz", "rt", "rt_range:min", "rt_range:max", "charge"].map(String::from).to_vec();
    for s in c.samples {
        for col in ["area", "height", "feature_state"] {
            head.push(format!("datafile:{}:{col}", stem(&s.file)));
        }
    }
    let mut out = String::new();
    write_line(&mut out, head);
    for r in c.rows() {
        let q = (!t.meta.is_empty()).then(|| t.meta.get(r));
        let edge = |v: Option<f32>| v.filter(|x| x.is_finite()).map_or(String::new(), |x| fix(f64::from(x), 4));
        let mut row = vec![
            t.id(r).to_string(),
            fix(t.mz[r], 5),
            fix(t.rt[r], 4),
            edge(q.map(|q| q.rt_lo)),
            edge(q.map(|q| q.rt_hi)),
            q.map_or(String::new(), |q| q.charge.to_string()),
        ];
        for k in 0..t.n_files {
            let cell = r * t.n_files + k;
            let a = c.area(r, k);
            row.push(a.map_or(String::new(), sig8));
            row.push(a.map_or(String::new(), |_| sig8(f64::from(t.height[cell]))));
            row.push(state_word(if a.is_some() { t.state[cell] } else { CELL_ABSENT }, mzmine_states).into());
        }
        write_line(&mut out, row);
    }
    out
}

/// The quantification table that FBMN reads: `row ID, row m/z, row retention time`, the ion-identity columns (-1 = none; `best ion` and
/// `neutral M mass` come from the identity of the row when it has a hypothesis) and `<sample>.mzML Peak area`.
pub fn quant(c: &Ctx) -> String {
    let t = c.table;
    let names = crate::rowident::adduct_names();
    let mut head: Vec<String> =
        ["row ID", "row m/z", "row retention time", "correlation group ID", "annotation network number", "best ion", "neutral M mass"].map(String::from).to_vec();
    head.extend(c.samples.iter().map(|s| format!("{}.mzML Peak area", stem(&s.file))));
    let mut out = String::new();
    write_line(&mut out, head);
    for r in c.rows() {
        let named = !t.ident.is_empty() && t.ident.src[r] >> 1 != 0;
        let ion = if named { names.get(usize::from(t.ident.ion[r]).wrapping_sub(1)).copied().unwrap_or("") } else { "" };
        let mut row = vec![
            t.id(r).to_string(),
            fix(t.mz[r], 5),
            fix(t.rt[r], 4),
            "-1".into(),
            "-1".into(),
            ion.into(),
            if named { fix(t.ident.neutral[r], 5) } else { String::new() },
        ];
        row.extend((0..t.n_files).map(|k| c.area(r, k).map_or("0".into(), sig8)));
        write_line(&mut out, row);
    }
    out
}

/// Header and rows of the sheet that the page writes as xlsx and as CSV with `;`: `id, mz, rt, rt_range:min, rt_range:max, charge` and, for every file,
/// `<F> area` and `<F> state`. Cells are None where there is nothing to write; the rows are those of the view and the areas the ones of `Ctx::area`.
fn sheet(c: &Ctx) -> (Vec<String>, Vec<Vec<Option<Cell>>>) {
    let t = c.table;
    let mut head: Vec<String> = ["id", "mz", "rt", "rt_range:min", "rt_range:max", "charge"].map(String::from).to_vec();
    for s in c.samples {
        head.push(format!("{} area", stem(&s.file)));
        head.push(format!("{} state", stem(&s.file)));
    }
    let rows = c
        .rows()
        .into_iter()
        .map(|r| {
            let q = (!t.meta.is_empty()).then(|| t.meta.get(r));
            let edge = |v: Option<f32>| v.filter(|x| x.is_finite()).map(|x| Cell::Num(f64::from(x)));
            let mut row = vec![Some(Cell::Num(f64::from(t.id(r)))), Some(Cell::Num(t.mz[r])), Some(Cell::Num(t.rt[r])), edge(q.map(|q| q.rt_lo)), edge(q.map(|q| q.rt_hi)), q.map(|q| Cell::Num(f64::from(q.charge)))];
            for k in 0..t.n_files {
                let a = c.area(r, k);
                row.push(a.map(Cell::Num));
                row.push(Some(Cell::Text(state_word(if a.is_some() { t.state[r * t.n_files + k] } else { CELL_ABSENT }, false).into())));
            }
            row
        })
        .collect();
    (head, rows)
}

/// One cell of the sheet.
enum Cell {
    Num(f64),
    Text(String),
}

/// The rows of the sheet of features for the xlsx that the page zips: `{"hojas":[{"nombre":"Features","filas":[[header],[row]...]}]}`, a cell with nothing is `null`.
pub fn xlsx_rows(c: &Ctx) -> String {
    use mzlab_core::json::Out;
    let (head, rows) = sheet(c);
    let mut o = Out::default();
    o.raw("{\"hojas\":[{\"nombre\":\"Features\",\"filas\":[[");
    for (i, h) in head.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(h);
    }
    o.raw("]");
    for row in rows {
        o.raw(",[");
        for (i, cell) in row.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            match cell {
                Some(Cell::Num(v)) => o.opt(Some(*v).filter(|v| v.is_finite())),
                Some(Cell::Text(s)) => o.str(s),
                None => o.raw("null"),
            };
        }
        o.raw("]");
    }
    o.raw("]}]}");
    o.0
}

/// The same sheet as text for a spreadsheet: UTF-8, `;` between cells, a cell with `;`, quote or line break between quotes, numbers as `fix`/`sig8` would.
pub fn csv_semicolon(c: &Ctx) -> String {
    let (head, rows) = sheet(c);
    let q = |s: &str| if s.contains([';', '"', '\r', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    let mut out = head.iter().map(|h| q(h)).collect::<Vec<_>>().join(";");
    out.push('\n');
    for row in rows {
        let cells: Vec<String> = row
            .iter()
            .map(|cell| match cell {
                Some(Cell::Num(v)) => sig8(*v),
                Some(Cell::Text(s)) => q(s),
                None => String::new(),
            })
            .collect();
        out.push_str(&cells.join(";"));
        out.push('\n');
    }
    out
}
