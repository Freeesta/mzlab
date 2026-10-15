// Ported from Attributio annotate.py, curation.py.
//! What the cascade asks of the curation of names (the contaminant map, the cleaning of library names, the canonical name of an identity). The cascade reads
//! it through `Curation`: `TableCuration` (`attributio::curation`) is the rule with the tables of a campaign, `BasicCuration` the rule without (no
//! contaminant map, no category override): a name is its own identity.

use crate::attributio::curation::{classify_noncec, clean_library_name, unnamed};
use crate::attributio::names;

/// `(label, category)` of a curated contaminant.
pub type Cec = (String, String);

pub trait Curation {
    /// The contaminant of the first name that matches the map; the names are tried in the order given (most reliable source first).
    fn match_cec(&self, names: &[Option<&str>]) -> Option<Cec>;
    /// Whether a contaminant label is in the headline subset.
    fn is_headline(&self, label: &str) -> bool;
    /// Adduct suffix, collision energy and CCS removed from a raw library name.
    fn clean_library_name(&self, name: &str) -> String;
    /// `(identity, category)` of one annotated row, the rule every count uses.
    fn curate_identity(
        &self,
        cec: Option<&Cec>,
        disp_name: Option<&str>,
        formula: Option<&str>,
    ) -> (String, String);
    /// The canonical name of an identity (`formula` groups the PEG series).
    fn canonical_name(&self, identity: &str, formula: Option<&str>) -> String;
    /// The corrected formula of an identity, by its lowercase key.
    fn formula_override(&self, key: &str) -> Option<String>;
}

/// Curation without tables.
#[derive(Clone, Copy, Debug, Default)]
pub struct BasicCuration;

impl Curation for BasicCuration {
    fn match_cec(&self, _names: &[Option<&str>]) -> Option<Cec> {
        None
    }
    fn is_headline(&self, _label: &str) -> bool {
        false
    }
    fn clean_library_name(&self, name: &str) -> String {
        clean_library_name(name)
    }
    fn curate_identity(
        &self,
        cec: Option<&Cec>,
        disp_name: Option<&str>,
        formula: Option<&str>,
    ) -> (String, String) {
        if let Some((label, cat)) = cec.filter(|c| !c.0.trim().is_empty()) {
            return (label.clone(), cat.clone());
        }
        match disp_name.filter(|n| !n.trim().is_empty()) {
            Some(n) => (n.to_string(), classify_noncec(Some(n)).to_string()),
            None => unnamed(formula),
        }
    }
    fn canonical_name(&self, identity: &str, formula: Option<&str>) -> String {
        names::canonical_name(identity, formula)
    }
    fn formula_override(&self, _key: &str) -> Option<String> {
        None
    }
}
