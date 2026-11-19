//! The parts of `samples/1` that `samples/1` does not have: free typed columns (`Column`, `Cell`), the document that carries them
//! (`Table`) and what the file says about itself (`FileData`). Used by `json3`, `csv`, `model` and `validate3`.

use super::Sample;

/// The value of a free column.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Text(String),
    Number(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Text,
    Number,
    Category,
}

impl ColumnType {
    pub fn name(self) -> &'static str {
        match self {
            ColumnType::Text => "text",
            ColumnType::Number => "number",
            ColumnType::Category => "category",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "text" => ColumnType::Text,
            "number" => ColumnType::Number,
            "category" => ColumnType::Category,
            _ => return None,
        })
    }
}

/// A free column: name, type, the allowed values of a category and whether every file must have a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub kind: ColumnType,
    pub values: Option<Vec<String>>,
    pub required: bool,
}

impl Column {
    pub fn new(name: &str, kind: ColumnType) -> Self {
        Column {
            name: name.to_string(),
            kind,
            values: None,
            required: false,
        }
    }

    /// Whether `c` is a value this column accepts (a number for a number, a listed text for a category when the list is given).
    pub fn accepts(&self, c: &Cell) -> bool {
        match (self.kind, c) {
            (ColumnType::Number, Cell::Number(_)) | (ColumnType::Text, Cell::Text(_)) => true,
            (ColumnType::Category, Cell::Text(t)) => {
                self.values.as_ref().is_none_or(|v| v.contains(t))
            }
            _ => false,
        }
    }
}

/// What a file says about itself (`from_file`): the user never edits it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileData {
    pub instrument: String,
    pub method: String,
    pub mz: Option<(f64, f64)>,
    pub rt: Option<(f64, f64)>,
    pub scans: Option<u32>,
}

/// A `samples/1` document: the files and the declared free columns.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    pub samples: Vec<Sample>,
    pub columns: Vec<Column>,
}

impl Table {
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }
}

impl Sample {
    /// The free value of `name`, if the file has one.
    pub fn extra_of(&self, name: &str) -> Option<&Cell> {
        self.extra.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    /// Sets (or replaces) a free value, keeping `extra` sorted by name.
    pub fn set_extra(&mut self, name: &str, v: Cell) {
        match self.extra.binary_search_by(|(k, _)| k.as_str().cmp(name)) {
            Ok(i) => self.extra[i].1 = v,
            Err(i) => self.extra.insert(i, (name.to_string(), v)),
        }
    }
}

/// A free column read without a declared type: a number when every value is one written the way a number is written here.
pub(super) fn infer(values: &[Option<&str>]) -> ColumnType {
    let filled: Vec<&str> = values
        .iter()
        .flatten()
        .copied()
        .filter(|v| !v.is_empty())
        .collect();
    let canon = |v: &&str| {
        v.parse::<f64>()
            .is_ok_and(|x| x.is_finite() && format!("{x}") == **v)
    };
    if !filled.is_empty() && filled.iter().all(canon) {
        ColumnType::Number
    } else {
        ColumnType::Text
    }
}
