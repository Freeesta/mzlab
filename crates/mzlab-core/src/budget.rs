//! Memory contract of the WASM modules (one instance may grow to 4 GiB; the working budget is chosen by `budget_auto`, 192 MB at the least): a phase estimates what it needs before it
//! allocates, then takes one of three ways: in memory (the estimate fits in half of the budget), in slabs (same bytes, less memory) or an orderly refusal
//! with the key `error.memoryBudget` (phase, estimate and budget in MB). Outputs do not come back as one `Vec`: they are written at offsets to a `WriteAt`
//! (an OPFS handle in the browser) and the caller only gets the length and the SHA-256.

use std::cell::Cell;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

const MB: usize = 1 << 20;
/// Smallest working budget of one WASM instance, MB (what a phone or a failed probe keeps); also the default when the page does not say.
pub const WASM_BUDGET_MB: usize = 192;
/// Budget of the native builds (desktop, tests): same code, other number.
pub const NATIVE_BUDGET_MB: usize = 1024;

/// How far a zlib array whose spectrum declares no length may inflate: 32 times its compressed bytes (real spectra compress 2 to 20 times), at least 1 MB, never past `TARGET_BYTES`.
pub fn undeclared_inflate(compressed: usize) -> usize {
    compressed.saturating_mul(32).clamp(1 << 20, TARGET_BYTES)
}

/// What a phase may hold at once. `reserve` hands out a `Guard` that gives the bytes back when dropped.
#[derive(Debug)]
pub struct MemBudget {
    cap: usize,
    used: Cell<usize>,
}

/// Bytes reserved from a budget, returned when the guard drops.
#[derive(Debug)]
pub struct Guard<'a> {
    budget: &'a MemBudget,
    bytes: usize,
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.budget.used.set(self.budget.used.get() - self.bytes);
    }
}

impl MemBudget {
    /// A budget of `mb` megabytes (fractions allowed: the tests use small ones).
    pub fn mb(mb: f64) -> MemBudget {
        MemBudget {
            cap: (mb.max(0.0) * MB as f64) as usize,
            used: Cell::new(0),
        }
    }

    /// The budget a JavaScript caller asks for (`budget_mb`): NaN, zero or negative gives the default of the WASM.
    pub fn from_request(mb: f64) -> MemBudget {
        MemBudget::mb(if mb.is_finite() && mb > 0.0 {
            mb
        } else {
            WASM_BUDGET_MB as f64
        })
    }

    pub fn cap(&self) -> usize {
        self.cap
    }

    pub fn used(&self) -> usize {
        self.used.get()
    }

    /// Bytes still free.
    pub fn left(&self) -> usize {
        self.cap.saturating_sub(self.used.get())
    }

    /// True if a phase whose estimate is `bytes` may take the way "in memory": it fits in half of what is free (the rest is code, stack, fragmentation
    /// and the buffers of the output).
    pub fn in_memory(&self, bytes: usize) -> bool {
        bytes <= self.left() / 2
    }

    /// Takes `bytes` for `phase`, or refuses in order.
    pub fn reserve(&self, phase: &'static str, bytes: usize) -> Result<Guard<'_>> {
        if bytes > self.left() {
            return Err(self.refuse(phase, bytes));
        }
        self.used.set(self.used.get() + bytes);
        Ok(Guard {
            budget: self,
            bytes,
        })
    }

    /// The error of a phase that does not fit even in its smallest slab; the excess is counted against what is free now.
    pub fn refuse(&self, phase: &'static str, estimate: usize) -> Error {
        refusal_free(phase, estimate, self.cap, self.left())
    }
}

/// `error.memoryBudget` with `{fase, stima_mb, budget_mb, libero_mb, eccesso_mb}` for a phase that has the whole budget free.
pub fn refusal(phase: &'static str, estimate: usize, cap: usize) -> Error {
    refusal_free(phase, estimate, cap, cap)
}

/// The same when part of the budget is already taken: `eccesso_mb` is what the phase asks above the free space (at least 1), so that "estimate 188,
/// budget 192" never looks as if it fitted when only 178 are free.
pub fn refusal_free(phase: &'static str, estimate: usize, cap: usize, free: usize) -> Error {
    Error::memory_budget()
        .with("fase", phase)
        .with("stima_mb", estimate.div_ceil(MB))
        .with("budget_mb", cap / MB)
        .with("libero_mb", free / MB)
        .with(
            "eccesso_mb",
            estimate.saturating_sub(free).div_ceil(MB).max(1),
        )
}

/// The memory of this target, bytes: what the decoded points of one spectrum may take at most.
pub const TARGET_BYTES: usize = if cfg!(target_family = "wasm") {
    WASM_BUDGET_MB
} else {
    NATIVE_BUDGET_MB
} * MB;

/// `error.memoryBudget` when `n` points of `per_point` bytes each exceed `budget` bytes: `n` is a count read from a file, checked before anything is decoded.
pub fn points_fit(n: usize, per_point: usize, budget: usize) -> Result<()> {
    if n.saturating_mul(per_point) > budget {
        return Err(Error::memory_budget());
    }
    Ok(())
}

/// An empty `Vec` with room for `n` items, or the memory error (never a trap).
pub fn try_vec<T>(n: usize) -> Result<Vec<T>> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| Error::memory_budget())?;
    Ok(v)
}

/// Where an output is written: bytes at an offset. The browser gives an OPFS handle (`write(k, offset, bytes)`), the tests a `Vec`.
pub trait WriteAt {
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()>;
}

impl WriteAt for Vec<u8> {
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()> {
        // an offset past usize (wasm32) or a sum that wraps is the same refusal as a failed grow, never a wrong write or a panic
        let off = usize::try_from(off).map_err(|_| Error::memory_budget())?;
        let end = off.checked_add(data.len()).ok_or_else(Error::memory_budget)?;
        if self.len() < end {
            self.try_reserve(end - self.len())
                .map_err(|_| Error::memory_budget())?;
            self.resize(end, 0);
        }
        self[off..end].copy_from_slice(data);
        Ok(())
    }
}

#[cfg(test)]
mod write_at_tests {
    use super::*;

    #[test]
    fn a_write_past_the_address_space_is_the_memory_error_and_not_a_panic() {
        let mut v: Vec<u8> = Vec::new();
        let e = v.write_at(u64::MAX - 1, &[1, 2, 3]).unwrap_err();
        assert_eq!(e.key, "error.memoryBudget");
        assert!(v.is_empty());
        v.write_at(2, &[7]).unwrap();
        assert_eq!(v, [0, 0, 7]);
    }
}

/// A sink that writes in order, counts the bytes and keeps the SHA-256 of everything written. A write that is not at the end is an error.
pub struct Hashing<W: WriteAt> {
    inner: W,
    pos: u64,
    hash: Sha256,
}

impl<W: WriteAt> Hashing<W> {
    pub fn new(inner: W) -> Hashing<W> {
        Hashing {
            inner,
            pos: 0,
            hash: Sha256::new(),
        }
    }

    /// The sink back, the bytes written and their SHA-256 (hex).
    pub fn finish(self) -> (W, u64, String) {
        let hex = self
            .hash
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        (self.inner, self.pos, hex)
    }
}

impl<W: WriteAt> WriteAt for Hashing<W> {
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()> {
        if off != self.pos {
            return Err(Error::new("error.io"));
        }
        self.inner.write_at(off, data)?;
        self.hash.update(data);
        self.pos += data.len() as u64;
        Ok(())
    }
}

/// SHA-256 (hex) of a byte string.
pub fn sha256_hex(b: &[u8]) -> String {
    sha256(b).iter().map(|x| format!("{x:02x}")).collect()
}

/// SHA-256 (hex) of the first `size` bytes of `r`, read in blocks of 1 MiB: a whole table or index is hashed without a copy of it in memory.
pub fn sha256_hex_at<R: crate::idxread::ReadAt + ?Sized>(r: &mut R, size: u64) -> Result<String> {
    let (mut h, mut buf, mut off) = (Sha256::new(), vec![0u8; MB.min(size as usize)], 0u64);
    while off < size {
        let n = (size - off).min(buf.len() as u64) as usize;
        r.read_at(off, &mut buf[..n])?;
        h.update(&buf[..n]);
        off += n as u64;
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// SHA-256 (hex) of everything `r` gives, in blocks of 1 MiB (a file hashed without reading it whole).
pub fn sha256_hex_read<R: std::io::Read>(mut r: R) -> std::io::Result<String> {
    let (mut h, mut buf) = (Sha256::new(), vec![0u8; MB]);
    loop {
        match r.read(&mut buf)? {
            0 => break,
            n => h.update(&buf[..n]),
        }
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// SHA-256 of a byte string, raw (the HMAC of the proxy ledger builds on it).
pub fn sha256(b: &[u8]) -> [u8; 32] {
    Sha256::digest(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader over bytes that keeps the largest block asked of it.
    struct Bytes(Vec<u8>, usize);
    impl crate::idxread::ReadAt for Bytes {
        fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
            self.1 = self.1.max(buf.len());
            buf.copy_from_slice(&self.0[off as usize..off as usize + buf.len()]);
            Ok(())
        }
    }

    #[test]
    fn the_hashes_in_blocks_are_the_hash_of_the_whole_and_read_no_more_than_a_block() {
        for n in [0, 1, MB - 1, MB, 2 * MB + 3] {
            let b: Vec<u8> = (0..n).map(|i| (i * 31 % 251) as u8).collect();
            let whole = sha256_hex(&b);
            let mut r = Bytes(b.clone(), 0);
            assert_eq!(sha256_hex_at(&mut r, n as u64).unwrap(), whole, "{n}");
            assert!(r.1 <= MB, "{n}: {}", r.1);
            assert_eq!(sha256_hex_read(&b[..]).unwrap(), whole, "{n}");
        }
    }

    #[test]
    fn reserve_gives_back_and_refuses_in_order() {
        let b = MemBudget::mb(1.0);
        {
            let _g = b.reserve("t", 600_000).unwrap();
            assert_eq!(b.used(), 600_000);
            let e = b.reserve("t", 600_000).unwrap_err();
            assert_eq!(e.key, "error.memoryBudget");
            assert!(e.params.iter().any(|(k, _)| *k == "fase"));
        }
        assert_eq!(b.used(), 0);
        assert!(b.in_memory(500_000) && !b.in_memory(600_000));
    }

    #[test]
    fn the_excess_is_counted_against_the_free_space() {
        let b = MemBudget::mb(192.0);
        let _g = b.reserve("t", 14 * MB).unwrap();
        let e = b.refuse("align", 188 * MB);
        let p = |k: &str| {
            e.params
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(p("budget_mb").as_deref(), Some("192"));
        assert_eq!(p("libero_mb").as_deref(), Some("178"));
        assert_eq!(p("eccesso_mb").as_deref(), Some("10"));
        // a size that would overflow a 32-bit usize when multiplied is still an orderly refusal
        let big = MemBudget::mb(192.0).reserve("t", usize::MAX).unwrap_err();
        assert_eq!(big.key, "error.memoryBudget");
    }

    #[test]
    fn request_defaults_and_hashing_sink() {
        assert_eq!(MemBudget::from_request(f64::NAN).cap(), WASM_BUDGET_MB * MB);
        let mut h = Hashing::new(Vec::new());
        h.write_at(0, b"ab").unwrap();
        h.write_at(2, b"c").unwrap();
        assert!(h.write_at(0, b"x").is_err());
        let (v, n, sha) = h.finish();
        assert_eq!((v.as_slice(), n), (&b"abc"[..], 3));
        assert_eq!(sha, sha256_hex(b"abc"));
    }
}
