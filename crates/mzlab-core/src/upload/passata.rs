//! `Passata`: the three fingerprints of a file taken in one pass over its bytes (used by `upload::fingerprint3` and the uploads that read in pieces).

use sha1::{Digest as _, Sha1};
use sha2::Sha256;

use super::{hex, probe_hex, PROBE};

/// The three fingerprints of one file taken in one pass, fed piece by piece in file order (`aggiorna`): SHA-256 (identity), SHA-1 of every byte (the
/// `sourceFile` of the mzML) and the `probe` (SHA-1 of the size, the first and the last 64 KB).
pub struct Passata {
    size: u64,
    seen: u64,
    sha256: Sha256,
    /// `None` when the caller does not want the SHA-1 of every byte (an mzML upload: only the SHA-256 and the `probe` are used).
    sha1: Option<Sha1>,
    head: Vec<u8>,
    tail: Vec<u8>,
}

impl Passata {
    pub fn new(size: u64) -> Passata {
        Passata {
            size,
            seen: 0,
            sha256: Sha256::new(),
            sha1: Some(Sha1::new()),
            head: Vec::with_capacity(size.min(PROBE) as usize),
            tail: Vec::with_capacity(size.min(PROBE) as usize),
        }
    }
    /// Only the SHA-256 and the `probe`: `fine` gives an empty SHA-1.
    pub fn senza_sha1(size: u64) -> Passata {
        Passata {
            sha1: None,
            ..Passata::new(size)
        }
    }
    /// `senza_sha1` or `new` by a flag (the page asks for it per upload).
    pub fn scelta(size: u64, senza: Option<bool>) -> Passata {
        if senza == Some(true) {
            Passata::senza_sha1(size)
        } else {
            Passata::new(size)
        }
    }
    pub fn aggiorna(&mut self, b: &[u8]) {
        self.sha256.update(b);
        if let Some(h) = self.sha1.as_mut() {
            h.update(b);
        }
        let n = (PROBE as usize)
            .saturating_sub(self.head.len())
            .min(b.len());
        self.head.extend_from_slice(&b[..n]);
        let keep = PROBE as usize;
        if b.len() >= keep {
            // the piece alone fills the tail
            self.tail.clear();
            self.tail.extend_from_slice(&b[b.len() - keep..]);
        } else {
            let old = self.tail.len();
            self.tail.drain(..(old + b.len()).saturating_sub(keep));
            self.tail.extend_from_slice(b);
        }
        self.seen += b.len() as u64;
    }
    /// `(sha256, sha1, probe)` as lower-case hex; `None` when the pieces given are not exactly `size` bytes.
    pub fn fine(self) -> Option<(String, String, String)> {
        if self.seen != self.size {
            return None;
        }
        Some((
            hex(&self.sha256.finalize()),
            self.sha1.map(|h| hex(&h.finalize())).unwrap_or_default(),
            probe_hex(self.size, &self.head, &self.tail),
        ))
    }
}
