//! The members of a `.raw` folder as the reader needs them: found by their file name (the case of the names varies between MassLynx versions) and read whole when small.

use mzlab_core::error::{Error, Result};
use mzlab_core::readers::{FileSet, Limits};

/// The largest side file read whole (`_extern.inf`, an index, a table of channels).
const SMALL_MAX: u64 = 8 << 20;

/// The last component of a logical member name.
pub(super) fn base(n: &str) -> &str {
    n.rsplit(['/', '\\']).next().unwrap_or(n)
}

pub(super) fn read_error(detail: &str) -> Error {
    Error::new("err.file.raw").with("detail", detail)
}

/// Bytes of a data file that no acquisition writes (a record word, a size or a range that does not fit): damage of the copy, not a missing file.
pub(super) fn damage(detail: &str) -> Error {
    Error::new("err.file.folderDamaged").with("detail", detail)
}

/// The members of one folder.
pub(super) struct Folder {
    pub(super) fs: FileSet,
    /// `X.raw`, the folder the members are in.
    pub(super) name: String,
    pub(super) limits: Limits,
}

impl Folder {
    pub(super) fn new(fs: FileSet, limits: Limits) -> Folder {
        let name = fs
            .membri
            .iter()
            .find_map(|m| {
                let n = m.nome.replace('\\', "/");
                let dir = n.rsplit_once('/')?.0.to_string();
                Some(base(&dir).to_string())
            })
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "MassLynx.raw".to_string());
        Folder { fs, name, limits }
    }

    /// Index of the member called `file` (case ignored).
    pub(super) fn search(&self, file: &str) -> Option<usize> {
        self.fs
            .membri
            .iter()
            .position(|m| base(&m.nome).eq_ignore_ascii_case(file))
    }

    /// The whole member `file`, or `None` when the folder has none; a member over the limit is an error (no allocation).
    pub(super) fn read(&mut self, file: &str) -> Result<Option<Vec<u8>>> {
        let Some(i) = self.search(file) else {
            return Ok(None);
        };
        let m = &mut self.fs.membri[i];
        if m.dim > SMALL_MAX.min(self.limits.byte) {
            return Err(Error::memory_budget());
        }
        let n = usize::try_from(m.dim).map_err(|_| Error::memory_budget())?;
        let mut b = vec![0u8; n];
        m.dati.read_at(0, &mut b)?;
        Ok(Some(b))
    }

    /// `len` bytes of member `i` from `off`; the range must lie inside the member.
    pub(super) fn read_from(&mut self, i: usize, off: u64, len: u64) -> Result<Vec<u8>> {
        let m = self
            .fs
            .membri
            .get_mut(i)
            .ok_or_else(|| read_error("member"))?;
        let fine = off.checked_add(len).ok_or_else(|| damage("range"))?;
        if fine > m.dim {
            return Err(damage("range past the end of the file"));
        }
        if len > self.limits.byte {
            return Err(Error::memory_budget());
        }
        let mut b = vec![0u8; usize::try_from(len).map_err(|_| Error::memory_budget())?];
        m.dati.read_at(off, &mut b)?;
        Ok(b)
    }
}

/// A little-endian number at `p` (0 past the end: a short file is a damaged file, not a panic).
pub(super) fn u16_at(b: &[u8], p: usize) -> u16 {
    b.get(p..p + 2)
        .map_or(0, |s| u16::from_le_bytes([s[0], s[1]]))
}

pub(super) fn u32_at(b: &[u8], p: usize) -> u32 {
    b.get(p..p + 4)
        .map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub(super) fn f32_at(b: &[u8], p: usize) -> f32 {
    f32::from_bits(u32_at(b, p))
}

/// Text of a side file: Windows-1252 (the degree and micro signs) read as Latin-1, carriage returns dropped.
pub(super) fn text(b: &[u8]) -> String {
    b.iter()
        .map(|&c| c as char)
        .filter(|&c| c != '\r')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mzlab_core::readers::Member;

    #[test]
    fn windows_member_path_names_the_folder() {
        let fs = FileSet {
            membri: vec![Member {
                nome: "campione.raw\\_FUNC001.DAT".into(),
                dim: 0,
                dati: Box::new(Vec::<u8>::new()),
                sha1: None,
            }],
        };
        assert_eq!(Folder::new(fs, Limits::default()).name, "campione.raw");
    }
}
