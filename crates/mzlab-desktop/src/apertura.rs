//! Files opened from outside the window (double click in the file manager, `mzlab file.mzML`, «Open with»): which arguments are files
//! the app can open, the queue the page empties when it is ready, and the rule of the notice «make mzLab the default app?».
//! Pure logic (no window): `main.rs` feeds it, `tests/apertura.rs` proves it.

use crate::librerie::Config;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Extensions that name a folder (Agilent `.D`, Bruker `.d`): the file dialog cannot pick them, the system cannot associate them.
const FOLDER_ONLY: [&str; 1] = [".d"];
/// Too generic to claim from the system (netCDF is also climate and model data): the dialog offers it, the associations do not.
const NOT_ASSOCIATED: [&str; 1] = [".nc"];

/// ONE source: the extensions of the readers' registry (`mzlab_lettori::estensioni()`, the same list the site's pickers get), without the folder-only ones.
/// Lower case with the dot; a companion such as `.wiff.scan` is kept whole.
pub fn file_extensions() -> Vec<String> {
    mzlab_lettori::estensioni()
        .into_iter()
        .filter(|e| !FOLDER_ONLY.contains(e))
        .map(str::to_string)
        .collect()
}

/// The extensions the dialog filter takes: no dot, in both lower and UPPER case (the OS filters differ in case handling).
pub fn dialog_extensions() -> Vec<String> {
    let mut v = Vec::new();
    for e in file_extensions() {
        let b = e.trim_start_matches('.').to_string();
        v.push(b.to_ascii_uppercase());
        v.push(b);
    }
    v.sort();
    v.dedup();
    v
}

/// The extensions registered as file associations (no dot, lower case, without repeats; companions such as `.wiff.scan` are not associated): what `tauri.conf.json` must list.
pub fn associated_extensions() -> Vec<String> {
    let mut v: Vec<String> = file_extensions()
        .into_iter()
        .filter(|e| !NOT_ASSOCIATED.contains(&e.as_str()) && e.matches('.').count() == 1)
        .map(|e| e.trim_start_matches('.').to_string())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Does the path name a file (not a folder) whose name ends with an extension of the readers' registry?
pub fn is_openable(p: &Path) -> bool {
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    file_extensions().iter().any(|e| name.ends_with(e.as_str())) && p.is_file()
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
