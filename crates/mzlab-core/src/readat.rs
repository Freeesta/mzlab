//! The `ReadAt` of a file on disk and of bytes in memory, and the two causes of a failed read: the source file did not give its bytes (`err.file.read`)
//! or the index is damaged (`error.badIndex`, `index_bytes`). The site's readers (`mzlab-wasm`) and the desktop map give the same `err.file.read`.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;

impl ReadAt for Vec<u8> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        let a = off as usize;
        let src = a
            .checked_add(buf.len())
            .and_then(|b| self.get(a..b))
            .ok_or_else(|| read_error(off, buf.len()))?;
        buf.copy_from_slice(src);
        Ok(())
    }
}

impl ReadAt for std::fs::File {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        use std::io::{Read, Seek, SeekFrom};
        self.seek(SeekFrom::Start(off))
            .and_then(|_| self.read_exact(buf))
            .map_err(|_| read_error(off, buf.len()))
    }
}

/// `err.file.read`: the bytes `off..off + len` of a file did not come (cut short, moved or closed); the index reads turn it into `error.badIndex` (`index_bytes`).
pub fn read_error(off: u64, len: usize) -> Error {
    Error::new("err.file.read")
        .with("offset", off)
        .with("length", len)
}

/// A read of the index itself: whatever the reader says, a missing or short slice means the index is damaged (`error.badIndex`), so the page builds it again.
pub(crate) fn index_bytes<R: ReadAt + ?Sized>(r: &mut R, off: u64, buf: &mut [u8]) -> Result<()> {
    r.read_at(off, buf)
        .map_err(|_| Error::new("error.badIndex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_source_read_names_its_bytes_and_an_index_read_stays_bad_index() {
        let mut v = vec![0u8; 10];
        let e = v.read_at(8, &mut [0u8; 4]).unwrap_err();
        assert_eq!(e.key, "err.file.read");
        assert_eq!(
            e.params,
            vec![("offset", "8".to_string()), ("length", "4".to_string())]
        );
        assert_eq!(
            index_bytes(&mut v, 8, &mut [0u8; 4]).unwrap_err().key,
            "error.badIndex"
        );
        let path = std::env::temp_dir().join(format!("mzlab-readat-{}", std::process::id()));
        std::fs::write(&path, [1u8; 6]).unwrap();
        let mut f = std::fs::File::open(&path).unwrap();
        assert_eq!(
            f.read_at(4, &mut [0u8; 4]).unwrap_err().key,
            "err.file.read"
        );
        let mut ok = [0u8; 2];
        f.read_at(4, &mut ok).unwrap();
        assert_eq!(ok, [1, 1]);
        std::fs::remove_file(&path).unwrap();
    }
}
