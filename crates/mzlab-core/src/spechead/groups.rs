//! The `referenceableParamGroup`s of a file for the shared header reader (`spechead`).

use super::Term;

/// The `referenceableParamGroup`s of a file, by id: a list, since a file has a handful (a hash map costs the `.wasm` modules a few KB).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Groups(Vec<(String, Vec<Term>)>);

impl Groups {
    pub fn new() -> Groups {
        Groups::default()
    }
    /// Group `id`; of a repeated id, the last one.
    pub fn get(&self, id: &str) -> Option<&Vec<Term>> {
        self.0.iter().rev().find(|(k, _)| k == id).map(|(_, g)| g)
    }
    /// Adds group `id` (a repeated id hides the earlier one).
    pub fn insert(&mut self, id: String, g: Vec<Term>) {
        self.0.push((id, g));
    }
}
