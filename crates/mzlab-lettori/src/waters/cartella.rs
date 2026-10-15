//! The members of a `.raw` folder as the reader needs them: found by their file name (the case of the names varies between MassLynx versions) and read whole when small.

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{FileSet, Limiti};

/// The largest side file read whole (`_extern.inf`, an index, a table of channels).
const PICCOLO_MAX: u64 = 8 << 20;

/// The last component of a logical member name.
pub(super) fn base(n: &str) -> &str {
    n.rsplit(['/', '\\']).next().unwrap_or(n)
}

pub(super) fn errore(detail: &str) -> Error {
    Error::new("err.file.raw").with("detail", detail)
}

/// The members of one folder.
pub(super) struct Cartella {
    pub(super) fs: FileSet,
    /// `X.raw`, the folder the members are in.
    pub(super) nome: String,
    pub(super) limiti: Limiti,
}

impl Cartella {
    pub(super) fn new(fs: FileSet, limiti: Limiti) -> Cartella {
        let nome = fs
            .membri
            .iter()
            .find_map(|m| {
                let n = m.nome.replace('\\', "/");
                let dir = n.rsplit_once('/')?.0.to_string();
                Some(base(&dir).to_string())
            })
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "MassLynx.raw".to_string());
        Cartella { fs, nome, limiti }
    }

    /// Index of the member called `file` (case ignored).
    pub(super) fn cerca(&self, file: &str) -> Option<usize> {
        self.fs
            .membri
            .iter()
            .position(|m| base(&m.nome).eq_ignore_ascii_case(file))
    }

    /// The whole member `file`, or `None` when the folder has none; a member over the limit is an error (no allocation).
    pub(super) fn leggi(&mut self, file: &str) -> Result<Option<Vec<u8>>> {
        let Some(i) = self.cerca(file) else {
            return Ok(None);
        };
        let m = &mut self.fs.membri[i];
        if m.dim > PICCOLO_MAX.min(self.limiti.byte) {
            return Err(Error::memory_budget());
        }
        let n = usize::try_from(m.dim).map_err(|_| Error::memory_budget())?;
        let mut b = vec![0u8; n];
        m.dati.read_at(0, &mut b)?;
        Ok(Some(b))
    }

    /// `len` bytes of member `i` from `off`; the range must lie inside the member.
    pub(super) fn leggi_a(&mut self, i: usize, off: u64, len: u64) -> Result<Vec<u8>> {
        let m = self.fs.membri.get_mut(i).ok_or_else(|| errore("member"))?;
        let fine = off.checked_add(len).ok_or_else(|| errore("range"))?;
        if fine > m.dim {
            return Err(errore("range past the end of the file"));
        }
        if len > self.limiti.byte {
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
pub(super) fn testo(b: &[u8]) -> String {
    b.iter()
        .map(|&c| c as char)
        .filter(|&c| c != '\r')
        .collect()
}
