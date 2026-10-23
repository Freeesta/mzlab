//! SHA-256 and size of a file read in pieces, for the files a flow run makes (`FileRunner::keep`); used by `flow.rs`.

use std::fs::File;
use std::path::{Path, PathBuf};

use mzlab_core::budget::{Hashing, WriteAt};
use mzlab_core::error::Result;
use mzlab_core::flow::Artifact;
use mzlab_core::step::DataType;

use crate::flow::{io, FileRunner};

/// A sink that keeps nothing: `Hashing` over it gives the size and SHA-256 of a stream.
struct Discard;

impl WriteAt for Discard {
    fn write_at(&mut self, _: u64, _: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// Size and SHA-256 (hex) of a file, read in pieces so that a large file is never in memory.
pub(crate) fn file_sha256(path: &Path) -> Result<(u64, String)> {
    use std::io::Read;
    let mut f = File::open(path).map_err(io)?;
    let mut sink = Hashing::new(Discard);
    let mut buf = vec![0u8; 1 << 20];
    let mut at = 0u64;
    loop {
        let n = f.read(&mut buf).map_err(io)?;
        if n == 0 {
            break;
        }
        sink.write_at(at, &buf[..n])?;
        at += n as u64;
    }
    let (_, bytes, sha) = sink.finish();
    Ok((bytes, sha))
}

impl FileRunner {
    /// Keeps a file the run made: its size and SHA-256 are read from it.
    pub(crate) fn keep(&self, path: PathBuf, ty: DataType) -> Result<Artifact> {
        let (bytes, sha256) = file_sha256(&path)?;
        Ok(self.keep_known(path, ty, bytes, sha256))
    }

    /// `keep` of a file whose size and SHA-256 the writer took while it wrote it: the file is not read again.
    pub(crate) fn keep_known(
        &self,
        path: PathBuf,
        ty: DataType,
        bytes: u64,
        sha256: String,
    ) -> Artifact {
        let label = path.file_name().map(|n| n.to_string_lossy().into_owned());
        self.made
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(sha256.clone(), path);
        Artifact {
            sha256,
            bytes,
            ty,
            label,
        }
    }
}
