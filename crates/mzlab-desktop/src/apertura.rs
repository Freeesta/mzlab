//! Files opened from outside the window (double click in the file manager, `mzlab file.mzML`, «Open with»): which arguments are files
//! the app can open, the queue the page empties when it is ready, and the rule of the notice «make mzLab the default app?».
//! Pure logic (no window): `main.rs` feeds it, `tests/apertura.rs` proves it.

use crate::librerie::Config;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Single files the app is associated with: mzML and Thermo `.raw`. Folders (Agilent `.D`, Bruker `.d`, Waters `.raw`) cannot be associated.
const EXT: [&str; 2] = ["mzml", "raw"];

/// Does the path name a file (not a folder) with an extension of `EXT`?
pub fn is_openable(p: &Path) -> bool {
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    ext.is_some_and(|e| EXT.contains(&e.as_str())) && p.is_file()
}

/// The files among the command line arguments (program name already removed); relative paths are taken from `cwd`. Options and anything else are ignored.
pub fn paths_from_args(args: &[String], cwd: Option<&Path>) -> Vec<PathBuf> {
    args.iter()
        .filter(|a| !a.starts_with('-'))
        .map(|a| match cwd {
            Some(c) => c.join(a),
            None => PathBuf::from(a),
        })
        .filter(|p| is_openable(p))
        .collect()
}

/// Files that arrived before the page could take them (cold start, or the window still loading): the page empties it with `take_opened`.
#[derive(Default)]
pub struct Pending(Mutex<Vec<PathBuf>>);

impl Pending {
    pub fn push(&self, paths: impl IntoIterator<Item = PathBuf>) {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        for p in paths {
            if !g.contains(&p) {
                g.push(p);
            }
        }
    }

    pub fn take(&self) -> Vec<PathBuf> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

/// How the user answered the notice for good: «Yes» (did the setting of the system) or «Don't ask again».
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Risposta {
    Si,
    Mai,
}

/// Counts one batch of files opened from outside and says whether the notice is due: at every other opening (the first, the third, …)
/// until the user answers for good. «No (not now)» changes nothing, so the next opening is skipped and the one after asks again.
pub fn registra_apertura(c: &mut Config) -> bool {
    let ask = c.risposta.is_none() && c.aperture.is_multiple_of(2);
    c.aperture = c.aperture.saturating_add(1);
    ask
}

/// The answer of the user: `Some` ends the notice, `None` is «not now».
pub fn rispondi(c: &mut Config, r: Option<Risposta>) {
    if r.is_some() {
        c.risposta = r;
    }
}

/// The setting «ask again»: the next opening asks.
pub fn rifai(c: &mut Config) {
    c.risposta = None;
    c.aperture = 0;
}
