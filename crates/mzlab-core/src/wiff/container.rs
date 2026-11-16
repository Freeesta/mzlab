//! Opening the container of a `.wiff`: a pair is read by sector (its size is mostly detector traces and the streams of samples nobody asked for); a copy cut short is refused whole (`err.file.wiffTruncated`).

use super::Ole;
use crate::error::{Error, Result};

/// The container of a `.wiff`; one cut short (sectors its header or its FAT use lie past the end) is refused whole: its streams would read empty or short,
/// so the samples, the index or the method would be missing unsaid.
pub fn container(wiff: Vec<u8>, name: &str) -> Result<Ole> {
    container_within(wiff, name, &crate::readers::Limits::held())
}

/// `container` bounded by `limits`: a file past the memory budget is told as such, not as a file that is not a `.wiff`.
pub fn container_within(wiff: Vec<u8>, name: &str, limits: &crate::readers::Limits) -> Result<Ole> {
    let size = wiff.len() as u64;
    let cut = Ole::header_cut(&wiff);
    finish(Ole::open(wiff, name, limits), cut, name, size)
}

/// `container_within` over a file read by sector (`Ole::open_at`): a `.wiff` bigger than the memory budget opens, and only the streams asked for are read.
pub fn container_at(
    mut src: Box<dyn crate::idxread::ReadAt>,
    len: u64,
    name: &str,
    limits: &crate::readers::Limits,
) -> Result<Ole> {
    let mut head = vec![0u8; usize::try_from(len.min(512)).unwrap_or(512)];
    src.read_at(0, &mut head)?;
    let cut = Ole::header_cut(&head);
    finish(Ole::open_at(src, len, name, limits), cut, name, len)
}

#[inline(never)]
fn finish(ole: Result<Ole>, cut: Option<u64>, name: &str, size: u64) -> Result<Ole> {
    let ole = ole.map_err(|e| match cut {
        _ if e.key == Error::memory_budget().key || e.key == "error.fixedLimit" => e,
        Some(needed) => truncated(name, size, needed),
        None => Error::new("err.file.notWiff").with("name", name),
    })?;
    match ole.cut_short() {
        Some(needed) => Err(truncated(name, size, needed)),
        None => Ok(ole),
    }
}

fn truncated(name: &str, size: u64, needed: u64) -> Error {
    Error::new("err.file.wiffTruncated")
        .with("name", name)
        .with("size", size)
        .with("needed", needed)
}

/// The container of member `m` of a pair, read by sector: whatever its size, only the streams asked for are read. A SHA-1 the member already carries (taken
/// during the copy) saves reading the whole file for it.
pub(super) fn open_member(
    m: &mut crate::readers::Member,
    name: &str,
    limits: &crate::readers::Limits,
) -> Result<Ole> {
    let known = m
        .sha1
        .take()
        .filter(|h| !h.is_empty() && !super::reader::is_mark(h));
    let (len, src) = (
        m.dim,
        std::mem::replace(&mut m.dati, Box::new(Vec::<u8>::new())),
    );
    Ok(container_at(src, len, name, limits)?.with_sha1(known))
}
