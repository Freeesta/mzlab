//! SHA-256 and size of a file read in pieces, for the files a flow run makes (`FileRunner::keep`); used by `flow.rs`.

use std::fs::File;
use std::path::Path;

use mzlab_core::budget::{Hashing, WriteAt};
use mzlab_core::error::Result;

use crate::flow::io;

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
