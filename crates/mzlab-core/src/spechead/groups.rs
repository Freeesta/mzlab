//! The `referenceableParamGroup`s of a file for the shared header reader (`spechead`).

use super::Term;

/// The `referenceableParamGroup`s of a file, by id: a list, since a file has a handful (a hash map costs the `.wasm` modules a few KB).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Groups(Vec<(String, Vec<Term>)>);

impl Groups {
    pub fn new() -> Groups {
        Groups::default()
    }
    pub fn get(&self, id: &str) -> Option<&Vec<Term>> {
        self.0.iter().find(|(k, _)| k == id).map(|(_, g)| g)
    }
    /// Adds group `id`; a repeated id replaces the earlier one.
    pub fn insert(&mut self, id: String, g: Vec<Term>) {
        match self.0.iter_mut().find(|(k, _)| *k == id) {
            Some(e) => e.1 = g,
            None => self.0.push((id, g)),
        }
    }
}
