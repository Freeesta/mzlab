//! `Passata`: the three fingerprints of a file taken in one pass over its bytes (used by `upload::fingerprint3` and the uploads that read in pieces).

use sha1::{Digest as _, Sha1};
use sha2::Sha256;

use super::{hex, probe_hex, PROBE};

/// The three fingerprints of one file taken in one pass, fed piece by piece in file order (`aggiorna`): SHA-256 (identity), SHA-1 of every byte (the
/// `sourceFile` of the mzML) and the `probe` (SHA-1 of the size, the first and the last 64 KB).
pub struct Passata {
    size: u64,
    seen: u64,
    /// `None` for the SHA-1 half of a member hashed by two workers (`solo_sha1`).
    sha256: Option<Sha256>,
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
            sha256: Some(Sha256::new()),
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
    /// Only the SHA-1 and the `probe`: `fine` gives an empty SHA-256. With `senza_sha1` on a second core, the two halves of `new` (the site's two hash workers).
    pub fn solo_sha1(size: u64) -> Passata {
        Passata {
            sha256: None,
            ..Passata::new(size)
        }
    }
    /// `senza_sha1`, `solo_sha1` (the SHA-1 half of a member that two workers hash) or `new` by the page's flags.
    pub fn scelta(size: u64, senza: Option<bool>, solo: Option<bool>) -> Passata {
        if solo == Some(true) {
            Passata::solo_sha1(size)
        } else if senza == Some(true) {
            Passata::senza_sha1(size)
        } else {
            Passata::new(size)
        }
    }
    pub fn aggiorna(&mut self, b: &[u8]) {
        if let Some(h) = self.sha256.as_mut() {
            h.update(b);
        }
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
            self.sha256.map(|h| hex(&h.finalize())).unwrap_or_default(),
            self.sha1.map(|h| hex(&h.finalize())).unwrap_or_default(),
            probe_hex(self.size, &self.head, &self.tail),
        ))
    }
}

/// Native `upload::fingerprint3`: the SHA-1 is taken on a second thread from the same pieces (`solo_sha1`), as the site's second hash worker does; one read of the file.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn due_fili(
    r: &mut dyn crate::idxread::ReadAt,
    size: u64,
    step: u64,
) -> crate::Result<(String, String, String)> {
    use std::sync::{mpsc, Arc};
    let mut x = Passata::senza_sha1(size);
    std::thread::scope(|s| {
        let (tx, rx) = mpsc::sync_channel::<Arc<Vec<u8>>>(4);
        let half = s.spawn(move || {
            let mut y = Passata::solo_sha1(size);
            for piece in rx {
                y.aggiorna(&piece);
            }
            y.fine()
        });
        let mut at = 0;
        let mut read = Ok(());
        while at < size {
            let n = (size - at).min(step) as usize;
            let mut buf = vec![0u8; n];
            read = r.read_at(at, &mut buf);
            if read.is_err() {
                break;
            }
            x.aggiorna(&buf);
            if tx.send(Arc::new(buf)).is_err() {
                break;
            }
            at += n as u64;
        }
        drop(tx);
        let y = half.join().ok().flatten();
        read?;
        match (x.fine(), y) {
            (Some((sha256, _, probe)), Some((_, sha1, _))) => Ok((sha256, sha1, probe)),
            _ => Err(crate::Error::new("err.file.unknown")),
        }
    })
}
