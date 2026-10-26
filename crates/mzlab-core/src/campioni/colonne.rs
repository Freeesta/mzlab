//! The parts of `campioni/3` that `campioni/2` does not have: free typed columns (`Colonna`, `Cell`), the document that carries them
//! (`Tabella`) and what the file says about itself (`DatiFile`). Used by `json3`, `csv`, `modello` and `valida3`.

use super::Campione;

/// The value of a free column.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Testo(String),
    Numero(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoColonna {
    Testo,
    Numero,
    Categoria,
}

impl TipoColonna {
    pub fn name(self) -> &'static str {
        match self {
            TipoColonna::Testo => "testo",
            TipoColonna::Numero => "numero",
            TipoColonna::Categoria => "categoria",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "testo" => TipoColonna::Testo,
            "numero" => TipoColonna::Numero,
            "categoria" => TipoColonna::Categoria,
            _ => return None,
        })
    }
}

/// A free column: name, type, the allowed values of a category and whether every file must have a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Colonna {
    pub name: String,
    pub kind: TipoColonna,
    pub values: Option<Vec<String>>,
    pub required: bool,
}

impl Colonna {
    pub fn new(name: &str, kind: TipoColonna) -> Self {
        Colonna {
            name: name.to_string(),
            kind,
            values: None,
            required: false,
        }
    }

    /// Whether `c` is a value this column accepts (a number for a number, a listed text for a category when the list is given).
    pub fn accepts(&self, c: &Cell) -> bool {
        match (self.kind, c) {
            (TipoColonna::Numero, Cell::Numero(_)) | (TipoColonna::Testo, Cell::Testo(_)) => true,
            (TipoColonna::Categoria, Cell::Testo(t)) => {
                self.values.as_ref().is_none_or(|v| v.contains(t))
            }
            _ => false,
        }
    }
}

/// What a file says about itself (`dal_file`): the user never edits it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DatiFile {
    pub instrument: String,
    pub method: String,
    pub mz: Option<(f64, f64)>,
    pub rt: Option<(f64, f64)>,
    pub scans: Option<u32>,
}

/// A `campioni/3` document: the files and the declared free columns.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tabella {
    pub campioni: Vec<Campione>,
    pub colonne: Vec<Colonna>,
}

impl Tabella {
    pub fn column(&self, name: &str) -> Option<&Colonna> {
        self.colonne.iter().find(|c| c.name == name)
    }
}

impl Campione {
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
pub(super) fn infer(values: &[Option<&str>]) -> TipoColonna {
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
        TipoColonna::Numero
    } else {
        TipoColonna::Testo
    }
}
