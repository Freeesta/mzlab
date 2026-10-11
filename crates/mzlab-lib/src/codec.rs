//! Zstandard blocks (pure Rust: `ruzstd`, in the WASM graph too). A block that does not shrink is stored raw.

use crate::error::{Error, Result};
use crate::format::{METHOD_RAW, METHOD_ZSTD};
use crate::util::vec_with_capacity;
use ruzstd::decoding::StreamingDecoder;
use ruzstd::encoding::{compress_to_vec, CompressionLevel};
use std::io::Read;

pub fn compress(raw: &[u8]) -> (u8, Vec<u8>) {
    if raw.len() >= 64 {
        let z = compress_to_vec(raw, CompressionLevel::Fastest);
        if z.len() < raw.len() {
            return (METHOD_ZSTD, z);
        }
    }
    (METHOD_RAW, raw.to_vec())
}

/// Decodes exactly `raw_len` bytes; anything else (short, long, malformed) is `lib.err.corrupt`.
pub fn decompress(stored: &[u8], method: u8, raw_len: usize) -> Result<Vec<u8>> {
    match method {
        METHOD_RAW => {
            if stored.len() != raw_len {
                return Err(Error::corrupt("block"));
            }
            let mut v = vec_with_capacity::<u8>(raw_len)?;
            v.extend_from_slice(stored);
            Ok(v)
        }
        METHOD_ZSTD => {
            let mut dec = StreamingDecoder::new(stored).map_err(|_| Error::corrupt("block"))?;
            let mut out = vec_with_capacity::<u8>(raw_len)?;
            out.resize(raw_len, 0);
            dec.read_exact(&mut out)
                .map_err(|_| Error::corrupt("block"))?;
            let mut extra = [0u8; 1];
            if dec.read(&mut extra).map_err(|_| Error::corrupt("block"))? != 0 {
                return Err(Error::corrupt("block"));
            }
            Ok(out)
        }
        _ => Err(Error::corrupt("method")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_wrong_lengths() {
        let raw: Vec<u8> = (0..5000u32).map(|i| (i % 17) as u8).collect();
        let (m, z) = compress(&raw);
        assert_eq!(m, METHOD_ZSTD);
        assert!(z.len() < raw.len() / 2);
        assert_eq!(decompress(&z, m, raw.len()).unwrap(), raw);
        assert!(decompress(&z, m, raw.len() - 1).is_err());
        assert!(decompress(&z, m, raw.len() + 1).is_err());
        assert!(decompress(&z[..z.len() / 2], m, raw.len()).is_err());
        let (m, s) = compress(&[1, 2, 3]);
        assert_eq!(
            (m, decompress(&s, m, 3).unwrap()),
            (METHOD_RAW, vec![1, 2, 3])
        );
    }
}
