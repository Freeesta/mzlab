//! The stamp of a dataset for the conversion queue (`conv_coda.rs`): size and date over every file that makes it.

use std::path::{Path, PathBuf};

fn file_stamp(m: &std::fs::Metadata) -> (u64, u64) {
    let t = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64);
    (m.len(), t)
}

fn add_members(dir: &Path, acc: &mut (u64, u64)) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let Ok(t) = e.file_type() else { continue };
        if t.is_dir() {
            add_members(&e.path(), acc);
        } else if let Ok(m) = e.metadata() {
            let (len, ms) = file_stamp(&m);
            *acc = (acc.0.wrapping_add(len), acc.1.max(ms));
        }
    }
}

/// Total size and latest date over every file of the dataset: the `.wiff.scan` beside a `.wiff`, the files inside a `.D` folder
/// (the date of the folder itself does not change when a file inside is rewritten).
pub(crate) fn stamp(p: &Path) -> Option<(u64, u64)> {
    let m = std::fs::metadata(p).ok()?;
    let mut acc = file_stamp(&m);
    if m.is_dir() {
        add_members(p, &mut acc);
    } else if p
        .extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("wiff"))
    {
        let mut scan = p.as_os_str().to_owned();
        scan.push(".scan");
        if let Ok(m) = std::fs::metadata(PathBuf::from(scan)) {
            let (len, ms) = file_stamp(&m);
            acc = (acc.0.wrapping_add(len), acc.1.max(ms));
        }
    }
    Some(acc)
}
