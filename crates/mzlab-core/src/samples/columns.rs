//! The parts of `campioni/3` that `campioni/2` does not have: free typed columns (`Colonna`, `Cell`), the document that carries them
//! (`Tabella`) and what the file says about itself (`FileData`). Used by `json3`, `csv`, `modello` and `valida3`.

use super::Sample;

/// The value of a free column.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Testo(String),
    Numero(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Testo,
    Numero,
    Categoria,
}

impl ColumnType {
    pub fn name(self) -> &'static str {
        match self {
            ColumnType::Testo => "testo",
            ColumnType::Numero => "numero",
            ColumnType::Categoria => "categoria",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "testo" => ColumnType::Testo,
            "numero" => ColumnType::Numero,
            "categoria" => ColumnType::Categoria,
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
            (ColumnType::Numero, Cell::Numero(_)) | (ColumnType::Testo, Cell::Testo(_)) => true,
            (ColumnType::Categoria, Cell::Testo(t)) => {
                self.values.as_ref().is_none_or(|v| v.contains(t))
            }
            _ => false,
        }
    }
}

/// What a file says about itself (`dal_file`): the user never edits it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileData {
    pub instrument: String,
    pub method: String,
    pub mz: Option<(f64, f64)>,
    pub rt: Option<(f64, f64)>,
    pub scans: Option<u32>,
}

/// A `campioni/3` document: the files and the declared free columns.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    pub campioni: Vec<Sample>,
    pub colonne: Vec<Column>,
}

impl Table {
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.colonne.iter().find(|c| c.name == name)
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
        ColumnType::Numero
    } else {
        ColumnType::Testo
    }
}
