//! The targets table of `quant.targets`: a CSV (`,`, `;` or tab, header first, `#` and empty lines skipped) or JSON (an array of objects, or
//! `{"targets": [...]}`) with `id`, `mz`, `rt_min`, `rt_max` and optional `polarity` (+, -, empty = every scan) and `role` (`target`, `standard`,
//! `parent`: the last two are where `offset = auto` measures the m/z offset). A wrong row refuses the table with its source line (the line of
//! the CSV file, header included, or the 1-based item of the JSON list) and the reason.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

use crate::lists::csv::{delimiter, split};

/// What a row is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Target,
    /// A standard or the parent: integrated like a target, and the ion `offset = auto` measures the offset on.
    Reference,
}

/// One target: m/z and RT window in minutes; `polarity` none = the scans of every polarity.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub id: String,
    pub mz: f64,
    pub rt_min: f64,
    pub rt_max: f64,
    pub polarity: Option<i8>,
    pub role: Role,
    /// Where the row is in the table: its line in the CSV file (1 = the first line), or its 1-based item in the JSON list.
    pub line: usize,
}

fn bad(line: usize, detail: &str) -> Error {
    Error::new("err.quant.targets.table")
        .with("row", line.to_string())
        .with("detail", detail.to_string())
}

fn polarity(t: &str, line: usize) -> Result<Option<i8>> {
    match t.trim().to_lowercase().as_str() {
        "" => Ok(None),
        "+" | "pos" | "positive" | "1" | "+1" => Ok(Some(1)),
        "-" | "neg" | "negative" | "-1" => Ok(Some(-1)),
        _ => Err(bad(line, "bad_polarity")),
    }
}

fn role(t: &str, line: usize) -> Result<Role> {
    match t.trim().to_lowercase().as_str() {
        "" | "target" => Ok(Role::Target),
        "standard" | "std" | "parent" | "reference" => Ok(Role::Reference),
        _ => Err(bad(line, "bad_role")),
    }
}

/// One row from its cells (text), checked: a name without control characters (a line break would split a row of the table), a positive
/// m/z, 0 <= rt_min < rt_max.
fn row(cells: [&str; 6], line: usize) -> Result<Target> {
    let [id, mz, lo, hi, pol, r] = cells;
    let id = id.trim();
    if id.is_empty() {
        return Err(bad(line, "no_id"));
    }
    if id.chars().any(char::is_control) {
        return Err(bad(line, "bad_id"));
    }
    let num = |t: &str| t.trim().parse::<f64>().ok().filter(|v| v.is_finite());
    let mz = num(mz)
        .filter(|v| *v > 0.0)
        .ok_or_else(|| bad(line, "bad_mz"))?;
    let (lo, hi) = match (num(lo), num(hi)) {
        (Some(a), Some(b)) if a >= 0.0 && b > a => (a, b),
        (Some(a), Some(b)) if a >= 0.0 && b <= a => return Err(bad(line, "rt_order")),
        _ => return Err(bad(line, "bad_rt")),
    };
    Ok(Target {
        id: id.to_string(),
        mz,
        rt_min: lo,
        rt_max: hi,
        polarity: polarity(pol, line)?,
        role: role(r, line)?,
        line,
    })
}

fn csv(text: &str) -> Result<Vec<Target>> {
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_end_matches('\r')))
        .filter(|(_, l)| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
    let Some((hl, header)) = lines.next() else {
        return Err(Error::new("err.quant.targets.empty"));
    };
    let d = delimiter(header);
    let names: Vec<String> = split(header, d)
        .iter()
        .map(|h| h.trim().to_lowercase())
        .collect();
    let col = |k: &[&str]| names.iter().position(|n| k.contains(&n.as_str()));
    let need = |k: &[&str]| col(k).ok_or_else(|| bad(hl, &format!("no_column_{}", k[0])));
    let at = [
        Some(need(&["id", "name"])?),
        Some(need(&["mz", "m/z"])?),
        Some(need(&["rt_min"])?),
        Some(need(&["rt_max"])?),
        col(&["polarity"]),
        col(&["role"]),
    ];
    let mut out = Vec::new();
    for (line, l) in lines {
        let cells = split(l, d);
        let get = |k: usize| at[k].and_then(|i| cells.get(i)).map_or("", |s| s.as_str());
        out.push(row([get(0), get(1), get(2), get(3), get(4), get(5)], line)?);
    }
    Ok(out)
}

fn cell(v: Option<&Value>) -> String {
    match v {
        Some(Value::Num(n)) => n.to_string(),
        Some(Value::Str(s)) => s.clone(),
        _ => String::new(),
    }
}

fn json(text: &str) -> Result<Vec<Target>> {
    let v = mzlab_core::json::parse(text.as_bytes()).map_err(|_| bad(0, "bad_json"))?;
    let list = match &v {
        Value::Arr(a) => a.as_slice(),
        Value::Obj(_) => v.get("targets").map(Value::arr).unwrap_or(&[]),
        _ => return Err(bad(0, "bad_json")),
    };
    let mut out = Vec::new();
    for (k, o) in list.iter().enumerate() {
        let id = cell(o.get("id").or_else(|| o.get("name")));
        let c = |key: &str| cell(o.get(key));
        out.push(row(
            [
                &id,
                &c("mz"),
                &c("rt_min"),
                &c("rt_max"),
                &c("polarity"),
                &c("role"),
            ],
            k + 1,
        )?);
    }
    Ok(out)
}

/// The targets of `bytes` (CSV or JSON, told by the first character), in their order; an id written twice is refused.
pub fn parse(bytes: &[u8]) -> Result<Vec<Target>> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start_matches('\u{feff}');
    let first = text.trim_start().chars().next();
    let rows = match first {
        None => return Err(Error::new("err.quant.targets.empty")),
        Some('[') | Some('{') => json(text)?,
        _ => csv(text)?,
    };
    if rows.is_empty() {
        return Err(Error::new("err.quant.targets.empty"));
    }
    let mut seen = std::collections::HashSet::new();
    for r in &rows {
        if !seen.insert(r.id.as_str()) {
            return Err(bad(r.line, &format!("duplicate_id {}", r.id)));
        }
    }
    Ok(rows)
}
