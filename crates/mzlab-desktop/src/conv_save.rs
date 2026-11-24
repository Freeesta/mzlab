//! «Save the mzML files…» of the conversion queue: every ready mzML goes into one folder, each under the name the page asks for (or its own).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mzlab_core::Result;

use crate::conv_queue::{file_of, lock, Phase, Queue};
use crate::rpc::io_error;

/// A name the page gives to save a file under: a bare `.mzML` file name, never a path.
fn plain_mzml_name(n: &str) -> bool {
    n.len() > ".mzML".len() && n.ends_with(".mzML") && !n.contains(['/', '\\'])
}

impl Queue {
    /// Puts every ready mzML in the folder `dest` (a hard link where the system allows, else a copy); how many. `names` maps a page file to the name it is
    /// saved under (the names of `mzlab_core::samples::mzml_names` the page asked for with «Name files by sample name»); a file it does not list keeps its own.
    pub(crate) fn save(&self, dest: &Path, names: &HashMap<String, String>) -> Result<usize> {
        std::fs::create_dir_all(dest).map_err(|e| io_error(&e))?;
        let files: Vec<(PathBuf, Option<String>)> = lock(&self.inner)
            .jobs
            .iter()
            .filter(|j| j.phase == Phase::Done)
            .flat_map(|j| {
                j.files.iter().map(|f| {
                    let by = j
                        .names
                        .iter()
                        .find(|(_, id)| file_of(j, *id).as_ref() == Some(f))
                        .and_then(|(n, _)| names.get(n))
                        .filter(|n| plain_mzml_name(n));
                    (f.clone(), by.cloned())
                })
            })
            .collect();
        let mut n = 0;
        for (f, by) in files {
            let Some(own) = f.file_name() else { continue };
            let to = dest.join(by.as_deref().map_or(own, std::ffi::OsStr::new));
            let _ = std::fs::remove_file(&to);
            std::fs::hard_link(&f, &to)
                .or_else(|_| std::fs::copy(&f, &to).map(|_| ()))
                .map_err(|e| io_error(&e))?;
            n += 1;
        }
        Ok(n)
    }
}
