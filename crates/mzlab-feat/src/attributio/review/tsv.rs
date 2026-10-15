// Ported from Attributio src/attributio/review/sources.py (the way `pandas.read_csv(sep="\t")` types a column, which the hit tables rely on).
//
// A small table of tab-separated text: a column whose non-empty cells are all numbers is numeric (integers when none is missing), any other
// column keeps its text, and the usual "not available" spellings are missing cells.

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Na,
    Int(i64),
    Float(f64),
    Str(String),
}

impl Cell {
    pub fn is_na(&self) -> bool {
        matches!(self, Cell::Na)
    }

    /// `pd.to_numeric(errors="coerce")` of the cell: a number, or `None`.
    pub fn num(&self) -> Option<f64> {
        match self {
            Cell::Int(i) => Some(*i as f64),
            Cell::Float(f) => Some(*f),
            Cell::Str(s) => s.trim().parse::<f64>().ok().filter(|v| v.is_finite()),
            Cell::Na => None,
        }
    }

    /// The text if the cell is text.
    pub fn text(&self) -> Option<&str> {
        match self {
            Cell::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Python's `str(cell)` of the pandas value: a missing cell is `nan`, a float has its `.0`.
    pub fn py_str(&self) -> String {
        match self {
            Cell::Na => "nan".to_string(),
            Cell::Int(i) => i.to_string(),
            Cell::Float(f) if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e16 => {
                format!("{f:.1}")
            }
            Cell::Float(f) => format!("{f}"),
            Cell::Str(s) => s.clone(),
        }
    }
}

const NA_TOKENS: [&str; 19] = [
    "", "#N/A", "#N/A N/A", "#NA", "-1.#IND", "-1.#QNAN", "-NaN", "-nan", "1.#IND", "1.#QNAN",
    "<NA>", "N/A", "NA", "NULL", "NaN", "None", "n/a", "nan", "null",
];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tsv {
    pub header: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}

fn split_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (mut cur, mut quoted, mut chars) = (String::new(), false, line.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' if cur.is_empty() && !quoted => quoted = true,
            '"' if quoted => quoted = false,
            '\t' if !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

impl Tsv {
    pub fn parse(text: &str) -> Tsv {
        let mut lines = text
            .trim_start_matches('\u{feff}')
            .lines()
            .filter(|l| !l.is_empty());
        let Some(h) = lines.next() else {
            return Tsv::default();
        };
        let header = split_line(h);
        let raw: Vec<Vec<String>> = lines.map(split_line).collect();
        let mut rows: Vec<Vec<Cell>> = raw.iter().map(|_| Vec::new()).collect();
        for c in 0..header.len() {
            let col: Vec<Option<&str>> = raw
                .iter()
                .map(|r| {
                    r.get(c)
                        .map(|s| s.as_str())
                        .filter(|s| !NA_TOKENS.contains(s))
                })
                .collect();
            let numeric = col
                .iter()
                .flatten()
                .all(|s| s.trim().parse::<f64>().is_ok_and(|v| v.is_finite()));
            let all_int = col
                .iter()
                .all(|s| s.is_some_and(|s| s.trim().parse::<i64>().is_ok()));
            for (r, v) in rows.iter_mut().zip(&col) {
                r.push(match v {
                    None => Cell::Na,
                    Some(s) if !numeric => Cell::Str((*s).to_string()),
                    Some(s) if all_int => Cell::Int(s.trim().parse().unwrap_or(0)),
                    Some(s) => Cell::Float(s.trim().parse().unwrap_or(f64::NAN)),
                });
            }
        }
        Tsv { header, rows }
    }

    pub fn col(&self, name: &str) -> Option<usize> {
        self.header.iter().position(|h| h == name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.col(name).is_some()
    }

    /// The cell of a row, `Na` when the column does not exist.
    pub fn get<'a>(&'a self, row: usize, name: &str) -> &'a Cell {
        const NA: Cell = Cell::Na;
        self.col(name)
            .and_then(|c| self.rows[row].get(c))
            .unwrap_or(&NA)
    }
}
