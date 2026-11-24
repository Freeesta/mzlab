//! «Save the mzML files…» of the conversion queue: every ready mzML goes into one folder, each under the name the page asks for (or its own); a file that is already
//! there is never replaced, and every choice made on the way is reported.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use mzlab_core::Result;

use crate::conv_queue::{file_of, lock, Phase, Queue};
use crate::rpc::io_error;

/// What a save did: how many files are in the folder now, the names left alone because another file is already there, and the files whose asked name was not
/// a bare `.mzML` file name (saved under their own).
#[derive(Debug, Default, serde::Serialize)]
pub struct Saved {
    pub saved: usize,
    pub kept: Vec<String>,
    pub own: Vec<String>,
}

/// A name the page gives to save a file under: a bare `.mzML` file name, never a path.
fn plain_mzml_name(n: &str) -> bool {
    n.len() > ".mzML".len() && n.ends_with(".mzML") && !n.contains(['/', '\\'])
}

/// The two files hold the same bytes (read side by side, so a large mzML is never held whole).
fn same_bytes(a: &Path, b: &Path) -> bool {
    let (Ok(x), Ok(y)) = (std::fs::File::open(a), std::fs::File::open(b)) else {
        return false;
    };
    if x.metadata().map(|m| m.len()).ok() != y.metadata().map(|m| m.len()).ok() {
        return false;
    }
    let (mut x, mut y) = (std::io::BufReader::new(x), std::io::BufReader::new(y));
    let (mut p, mut q) = (vec![0u8; 1 << 16], vec![0u8; 1 << 16]);
    loop {
        let (n, m) = match (x.read(&mut p), y.read(&mut q)) {
            (Ok(n), Ok(m)) => (n, m),
            _ => return false,
        };
        if n != m || p[..n] != q[..m] {
            return false;
        }
        if n == 0 {
            return true;
        }
    }
}

impl Queue {
    /// Puts every ready mzML in the folder `dest` (a hard link where the system allows, else a copy). `names` maps a page file to the name it is saved under (the
    /// names of `mzlab_core::samples::mzml_names` the page asked for with «Name files by sample name»); a file it does not list keeps its own. A name that
    /// is not a bare `.mzML` file name is not used (the file keeps its own, said in `own`); a file already there with other bytes is left as it is (said in
    /// `kept`), as `mzlab-cli file convert` does: nothing is replaced unless the user removes it.
    pub(crate) fn save(&self, dest: &Path, names: &HashMap<String, String>) -> Result<Saved> {
        std::fs::create_dir_all(dest).map_err(|e| io_error(&e))?;
        let files: Vec<(PathBuf, Option<String>, Option<String>)> = lock(&self.inner)
            .jobs
            .iter()
            .filter(|j| j.phase == Phase::Done)
            .flat_map(|j| {
                j.files.iter().map(|f| {
                    let page = j
                        .names
                        .iter()
                        .find(|(_, id)| file_of(j, *id).as_ref() == Some(f))
                        .and_then(|(n, _)| names.get(n));
                    let by = page.filter(|n| plain_mzml_name(n)).cloned();
                    let refused = page.filter(|n| !plain_mzml_name(n)).cloned();
                    (f.clone(), by, refused)
                })
            })
            .collect();
        let mut out = Saved::default();
        for (f, by, refused) in files {
            let Some(own) = f.file_name().map(|n| n.to_string_lossy().into_owned()) else {
                continue;
            };
            let name = by.unwrap_or_else(|| own.clone());
            if refused.is_some() {
                out.own.push(own);
            }
            let to = dest.join(&name);
            if to.exists() {
                // the same bytes (a second save of the same file) is a file saved; other bytes are never replaced
                if same_bytes(&f, &to) {
                    out.saved += 1;
                } else {
                    out.kept.push(name);
                }
                continue;
            }
            std::fs::hard_link(&f, &to)
                .or_else(|_| std::fs::copy(&f, &to).map(|_| ()))
                .map_err(|e| io_error(&e))?;
            out.saved += 1;
        }
        Ok(out)
    }
}
