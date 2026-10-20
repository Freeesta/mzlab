//! The levels of the records (catalogue < imported < user) and the archive that the engine holds between calls: the catalogue is loaded once, the records of the
//! user replace the catalogue ones with the same `id` (in the same place), the new ones follow.

use std::sync::Mutex;

use super::Molecola;

fn rank(m: &Molecola) -> usize {
    super::LIVELLI
        .iter()
        .position(|l| *l == m.origine.tipo)
        .unwrap_or(2)
}

/// The catalogue first, in its order, then the others; for an `id` that is in more places the highest level wins (the later of two equal ones), keeping the
/// place of the first.
pub fn unisci(catalogo: &[Molecola], altri: &[Molecola]) -> Vec<Molecola> {
    let mut out: Vec<Molecola> = Vec::with_capacity(catalogo.len() + altri.len());
    let mut at = std::collections::HashMap::new();
    for m in catalogo.iter().chain(altri) {
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
pub struct Archivio {
    pub catalogo: Vec<Molecola>,
    pub altri: Vec<Molecola>,
    /// `unisci(catalogo, altri)`.
    pub tutte: Vec<Molecola>,
}

pub static ARCHIVIO: Mutex<Archivio> = Mutex::new(Archivio {
    catalogo: Vec::new(),
    altri: Vec::new(),
    tutte: Vec::new(),
});

/// Runs `f` on the archive (a lock left by a panic is not an error).
pub fn con<R>(f: impl FnOnce(&mut Archivio) -> R) -> R {
    f(&mut ARCHIVIO.lock().unwrap_or_else(|e| e.into_inner()))
}

impl Archivio {
    pub fn aggiorna(&mut self) {
        self.tutte = unisci(&self.catalogo, &self.altri);
    }
}
