//! Files opened from outside the window (double click in the file manager, `mzlab file.mzML`, «Open with»): which arguments are files
//! the app can open, the queue the page empties when it is ready, and the rule of the notice «make mzLab the default app?».
//! Pure logic (no window): `main.rs` feeds it, `tests/opening.rs` proves it.

use crate::libraries::Config;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Extensions that name a folder (Agilent `.D`, Bruker `.d`): the file dialog cannot pick them, the system cannot associate them.
const FOLDER_ONLY: [&str; 1] = [".d"];
/// Too generic to claim from the system (netCDF is also climate and model data): the dialog offers it, the associations do not.
const NOT_ASSOCIATED: [&str; 1] = [".nc"];
/// The SCIEX companion of a `.wiff`: opened through its `.wiff`.
const WIFF_SCAN: &str = ".wiff.scan";

/// ONE source: the extensions of the readers' registry (`mzlab_lettori::estensioni()`, the same list the site's pickers get), without the folder-only ones.
/// Lower case with the dot; a companion such as `.wiff.scan` is kept whole.
pub fn file_extensions() -> Vec<String> {
    mzlab_lettori::estensioni()
        .into_iter()
        .filter(|e| !FOLDER_ONLY.contains(e))
        .map(str::to_string)
        .collect()
}

/// The extensions the dialog filter takes: the last component without dot (a filter cannot hold `mzml.gz`, so `gz` is offered), in both lower and UPPER case (the OS filters differ in case handling).
pub fn dialog_extensions() -> Vec<String> {
    let mut v = Vec::new();
    for e in file_extensions() {
        let b = e.rsplit('.').next().unwrap_or_default().to_string();
        v.push(b.to_ascii_uppercase());
        v.push(b);
    }
    v.sort();
    v.dedup();
    v
}

/// The extensions registered as file associations (no dot, lower case, without repeats): what `tauri.conf.json` must list.
/// The OS sees only the last extension, so the companion `.wiff.scan` is registered as `scan` (weakest claim, never the default: see `plan_from_args`,
/// which refuses a `.scan` that is not `.wiff.scan`); compressed `.mzml.gz` has no association of its own.
/// Folder formats (Waters `.raw` folder, Agilent `.D`, Bruker `.d`) cannot be associated by any OS: they stay drag and drop / «Open».
pub fn associated_extensions() -> Vec<String> {
    let mut v: Vec<String> = file_extensions()
        .into_iter()
        .filter(|e| !NOT_ASSOCIATED.contains(&e.as_str()))
        .filter(|e| e.matches('.').count() == 1 || e == WIFF_SCAN)
        .map(|e| e.rsplit('.').next().unwrap_or_default().to_string())
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

/// What an opening from outside becomes: the files to queue and the problems to tell the user (English: the native side has no translations).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub files: Vec<PathBuf>,
    pub problems: Vec<String>,
}

/// The file of `dir` whose name equals `name` ignoring case (the OS may hand over `X.WIFF.SCAN` while the folder holds `X.wiff`).
fn sibling(dir: &Path, name: &str) -> Option<PathBuf> {
    let want = name.to_lowercase();
    std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
        let p = e.path();
        (e.file_name().to_string_lossy().to_lowercase() == want && p.is_file()).then_some(p)
    })
}

/// What the system asked to open: `X.wiff.scan` is opened as the `.wiff` beside it (same folder, case-insensitive) together with itself, a `.wiff` takes its
/// `.wiff.scan` along, a `.wiff.scan` without its `.wiff` and a `.scan` of another maker are refused with a message and nothing is opened for them.
pub fn plan_from_paths(paths: impl IntoIterator<Item = PathBuf>) -> Plan {
    let mut plan = Plan::default();
    let add = |plan: &mut Plan, p: PathBuf| {
        if !plan.files.contains(&p) {
            plan.files.push(p);
        }
    };
    for p in paths {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let low = name.to_lowercase();
        let dir = p.parent().map(Path::to_path_buf).unwrap_or_default();
        if low.ends_with(".scan") && p.is_file() {
            if !low.ends_with(WIFF_SCAN) {
                plan.problems.push(format!("«{name}» is not a SCIEX scan companion (.wiff.scan) and cannot be opened as data."));
                continue;
            }
            let stem = &name[..name.len() - ".scan".len()];
            match sibling(&dir, stem) {
                Some(w) => {
                    add(&mut plan, w);
                    add(&mut plan, p);
                }
                None => plan.problems.push(format!("Cannot open «{name}»: the SCIEX data file «{stem}» is missing from the same folder.")),
            }
        } else if low.ends_with(".wiff") && p.is_file() {
            let scan = sibling(&dir, &format!("{name}.scan"));
            add(&mut plan, p);
            if let Some(s) = scan {
                add(&mut plan, s);
            }
        } else if is_openable(&p) {
            add(&mut plan, p);
        }
    }
    plan
}

/// `paths_from_args` followed by `plan_from_paths`: the command line and the second launch use it.
pub fn plan_from_args(args: &[String], cwd: Option<&Path>) -> Plan {
    let cand = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(|a| match cwd {
            Some(c) => c.join(a),
            None => PathBuf::from(a),
        });
    plan_from_paths(cand)
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
pub enum NoticeAnswer {
    #[serde(rename = "si")]
    Yes,
    #[serde(rename = "mai")]
    Never,
}

/// Counts one batch of files opened from outside and says whether the notice is due: at every other opening (the first, the third, …)
/// until the user answers for good. «No (not now)» changes nothing, so the next opening is skipped and the one after asks again.
pub fn record_opening(c: &mut Config) -> bool {
    let ask = c.risposta.is_none() && c.aperture.is_multiple_of(2);
    c.aperture = c.aperture.saturating_add(1);
    ask
}

/// The answer of the user: `Some` ends the notice, `None` is «not now».
pub fn answer_notice(c: &mut Config, r: Option<NoticeAnswer>) {
    if r.is_some() {
        c.risposta = r;
    }
}

/// The setting «ask again»: the next opening asks.
pub fn ask_again(c: &mut Config) {
    c.risposta = None;
    c.aperture = 0;
}
