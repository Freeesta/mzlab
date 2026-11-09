//! The levels of the records (catalogue < imported < user) and the archive that the engine holds between calls: the catalogue is loaded once, the records of the
//! user replace the catalogue ones with the same `id` (in the same place), the new ones follow.

use std::sync::Mutex;

use super::Molecule;

fn rank(m: &Molecule) -> usize {
    super::LEVELS
        .iter()
        .position(|l| *l == m.origine.tipo)
        .unwrap_or(2)
}

/// The catalogue first, in its order, then the others; for an `id` that is in more places the highest level wins (the later of two equal ones), keeping the
/// place of the first.
pub fn merge(catalog: &[Molecule], altri: &[Molecule]) -> Vec<Molecule> {
    let mut out: Vec<Molecule> = Vec::with_capacity(catalog.len() + altri.len());
    let mut at = std::collections::HashMap::new();
    for m in catalog.iter().chain(altri) {
        match at.get(&m.id) {
            Some(&k) if rank(m) >= rank(&out[k]) => out[k] = m.clone(),
            Some(_) => {}
            None => {
                at.insert(m.id.clone(), out.len());
                out.push(m.clone());
            }
        }
    }
    out
}

/// What `carica` leaves in the engine.
#[derive(Default)]
pub struct Archive {
    pub catalog: Vec<Molecule>,
    pub altri: Vec<Molecule>,
    /// `unisci(catalogo, altri)`.
    pub tutte: Vec<Molecule>,
}

pub static ARCHIVE: Mutex<Archive> = Mutex::new(Archive {
    catalog: Vec::new(),
    altri: Vec::new(),
    tutte: Vec::new(),
});

/// Runs `f` on the archive (a lock left by a panic is not an error).
pub fn con<R>(f: impl FnOnce(&mut Archive) -> R) -> R {
    f(&mut ARCHIVE.lock().unwrap_or_else(|e| e.into_inner()))
}

impl Archive {
    pub fn update(&mut self) {
        self.tutte = merge(&self.catalog, &self.altri);
    }
}
