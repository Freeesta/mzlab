//! The lists of the user, `liste/1` (`docs/schemi/liste-1.json`): the transformations (a change of formula and how many times it may repeat), the adducts (a
//! change of formula and a charge). A list is read from a CSV with an error for every wrong row
//! (`csv.rs`), written as canonical JSON (`json.rs`: keys in alphabetical order, items sorted by name and change) whose SHA-256 enters `prov/1`, and read back
//! to the same bytes. The transformation list of mzFinder (`name;change[;comment]` per line) is the header-less form of the same CSV and goes through here
//! (`transformation_rows`), so that its results do not change.

mod csv;
mod json;

pub use csv::from_csv;
pub use json::{errors_json, from_json, to_json};

use mzlab_core::budget::sha256_hex;

use crate::legacy::transf::{Delta, Row};

pub const SCHEMA: &str = "liste/1";
/// Most times a transformation may repeat.
pub const MAX_REPEAT: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Transformations,
    Adducts,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Transformations => "transformations",
            Kind::Adducts => "adducts",
        }
    }

    pub fn from_name(s: &str) -> Option<Kind> {
        [Kind::Transformations, Kind::Adducts]
            .into_iter()
            .find(|k| k.name() == s)
    }
}

/// A change of formula that a product may have, repeated up to `max_repeat` times (`None` = the rule default).
#[derive(Debug, Clone, PartialEq)]
pub struct Transformation {
    pub name: String,
    pub delta: Delta,
    pub max_repeat: Option<u32>,
    pub comment: String,
}

/// An adduct: the ion is the molecule plus `delta`, with `charge`.
#[derive(Debug, Clone, PartialEq)]
pub struct Adduct {
    pub name: String,
    pub delta: Delta,
    pub charge: i8,
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Liste {
    Transformations(Vec<Transformation>),
    Adducts(Vec<Adduct>),
}

/// A wrong row of a CSV: the 1-based line, an `error.liste.*` key and the text that explains it.
#[derive(Debug, Clone, PartialEq)]
pub struct RowError {
    pub line: usize,
    pub key: &'static str,
    pub detail: String,
}

impl Liste {
    pub fn kind(&self) -> Kind {
        match self {
            Liste::Transformations(_) => Kind::Transformations,
            Liste::Adducts(_) => Kind::Adducts,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Liste::Transformations(v) => v.len(),
            Liste::Adducts(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// SHA-256 of the canonical JSON: what the `.mzflow` and `prov/1` record.
    pub fn digest(&self) -> String {
        sha256_hex(to_json(self).as_bytes())
    }
}

/// The transformation list in the text of mzFinder (`name;change[;comment]` per line, `#` comments): the header-less form of the CSV, with the errors of the
/// Python as plain text.
pub fn from_legacy_text(text: &str) -> Result<Liste, String> {
    let rows = crate::legacy::transf::parse(text)?;
    Ok(Liste::Transformations(
        rows.into_iter()
            .map(|r| Transformation {
                name: r.name,
                delta: r.delta,
                max_repeat: r.max_repeat,
                comment: r.comment,
            })
            .collect(),
    ))
}

impl Liste {
    /// The transformations as the rows `legacy::transf::merge` adds to the rules, in the order of the list (empty for another kind of list).
    pub fn transformation_rows(&self) -> Vec<Row> {
        match self {
            Liste::Transformations(v) => v
                .iter()
                .map(|t| Row {
                    name: t.name.clone(),
                    delta: t.delta.clone(),
                    comment: t.comment.clone(),
                    max_repeat: t.max_repeat,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}
