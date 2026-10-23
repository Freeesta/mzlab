//! Hashes taken while the bytes pass, so that a file is not read again to be hashed: `HashWrite` (SHA-256 of what is written) and `hash_read` (SHA-256 and
//! SHA-1 of a stream in one pass). Used by the converter of `mzlab-cli` for the output it makes and the input it reads.

use std::io::{Read, Result, Write};

use sha1::Sha1;
use sha2::{Digest as _, Sha256};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// A writer that counts the bytes and hashes them (SHA-256) on their way to `inner`.
pub struct HashWrite<W: Write> {
    inner: W,
    bytes: u64,
    sha: Sha256,
}

impl<W: Write> HashWrite<W> {
    pub fn new(inner: W) -> Self {
        HashWrite {
            inner,
            bytes: 0,
            sha: Sha256::new(),
        }
    }

    /// The writer back, the bytes that went through and their SHA-256 (hex).
    pub fn finish(self) -> (W, u64, String) {
        (self.inner, self.bytes, hex(&self.sha.finalize()))
    }
}

impl<W: Write> Write for HashWrite<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let n = self.inner.write(buf)?;
        self.sha.update(buf.get(..n).unwrap_or_default());
        self.bytes += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

/// Size, SHA-256 and SHA-1 (hex) of everything `r` gives, read once in pieces of 1 MiB.
pub fn hash_read<R: Read>(r: &mut R) -> Result<(u64, String, String)> {
    let (mut sha256, mut sha1) = (Sha256::new(), Sha1::new());
    let (mut buf, mut bytes) = (vec![0u8; 1 << 20], 0u64);
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let piece = buf.get(..n).unwrap_or_default();
        sha256.update(piece);
        sha1.update(piece);
        bytes += n as u64;
    }
    Ok((bytes, hex(&sha256.finalize()), hex(&sha1.finalize())))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::cast_possible_truncation)]
mod tests {
    use super::*;

    fn sample(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 31 + i / 7) as u8).collect()
    }

    #[test]
    fn the_hash_of_what_is_written_is_the_hash_of_the_bytes_whatever_the_pieces() {
        let data = sample(3_000_001);
        let mut w = HashWrite::new(Vec::new());
        for piece in data.chunks(8191) {
            w.write_all(piece).unwrap();
        }
        w.flush().unwrap();
        let (out, bytes, sha) = w.finish();
        assert!(out == data);
        assert_eq!(bytes, data.len() as u64);
        assert_eq!(sha, hex(&Sha256::digest(&data)));
    }

    #[test]
    fn one_read_gives_both_hashes_and_the_size() {
        for n in [0, 1, (1 << 20) - 1, 1 << 20, (1 << 20) + 1, 2_500_000] {
            let data = sample(n);
            let (bytes, sha256, sha1) = hash_read(&mut data.as_slice()).unwrap();
            assert_eq!(bytes, n as u64);
            assert_eq!(sha256, hex(&Sha256::digest(&data)), "n={n}");
            assert_eq!(sha1, hex(&Sha1::digest(&data)), "n={n}");
        }
    }
}
