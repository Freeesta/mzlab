//! One polarity table: the CSV that `mzlab-cli features` writes (columns `id`, `mz` and `rt`; the others are not read). A wrong row refuses the table
//! with its line (the header is line 1) and the reason.

use mzlab_core::error::{Error, Result};

pub struct Row {
    pub id: String,
    pub mz: f64,
    pub rt: f64,
}

/// The cells of a line cut at `d` (a double-quoted cell may hold `d`), trimmed.
fn split(line: &str, d: char) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c == d && !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out.iter().map(|c| c.trim().to_string()).collect()
}

fn number(t: &str) -> Option<f64> {
    t.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

fn bad(table: &str, line: usize, detail: &str) -> Error {
    Error::new("err.pair.table")
        .with("table", table)
        .with("row", line.to_string())
        .with("detail", detail)
}

pub fn read(name: &str, text: &str) -> Result<Vec<Row>> {
    let text = text.trim_start_matches('\u{feff}');
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_end_matches('\r')))
        .filter(|(_, l)| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
    let Some((hl, header)) = lines.next() else {
        return Err(bad(name, 1, "empty"));
    };
    // the delimiter that cuts the header into most cells (`,` on a tie)
    let d = *[';', '\t', ',']
        .iter()
        .max_by_key(|d| (header.matches(**d).count(), **d == ','))
        .unwrap_or(&',');
    let names: Vec<String> = split(header, d).iter().map(|h| h.to_lowercase()).collect();
    let col = |k: &[&str]| names.iter().position(|n| k.contains(&n.as_str()));
    let need = |k: &[&str]| col(k).ok_or_else(|| bad(name, hl, &format!("no_column_{}", k[0])));
    let mz_at = need(&["mz", "m/z", "row m/z"])?;
    let rt_at = need(&["rt", "row retention time", "retention time"])?;
    let id_at = col(&["id", "name", "row id"]);
    let mut rows = Vec::new();
    for (line, l) in lines {
        let cells = split(l, d);
        let get = |i: usize| cells.get(i).map_or("", |c| c.as_str());
        let mz = number(get(mz_at))
            .filter(|v| *v > 0.0)
            .ok_or_else(|| bad(name, line, "bad_mz"))?;
        let rt = number(get(rt_at))
            .filter(|v| *v >= 0.0)
            .ok_or_else(|| bad(name, line, "bad_rt"))?;
        let id = id_at.map_or("", get);
        let id = if id.is_empty() {
            line.to_string()
        } else {
            id.to_string()
        };
        rows.push(Row { id, mz, rt });
    }
    Ok(rows)
}
