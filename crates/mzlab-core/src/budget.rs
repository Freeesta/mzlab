//! Memory contract of the WASM modules (one instance has a hard 256 MB and a working budget of 192 MB): a phase estimates what it needs before it
//! allocates, then takes one of three ways: in memory (the estimate fits in half of the budget), in slabs (same bytes, less memory) or an orderly refusal
//! with the key `error.memoryBudget` (phase, estimate and budget in MB). Outputs do not come back as one `Vec`: they are written at offsets to a `WriteAt`
//! (an OPFS handle in the browser) and the caller only gets the length and the SHA-256.

use std::cell::Cell;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

const MB: usize = 1 << 20;
/// Working budget of one WASM instance, MB (75 % of the 256 MB of the linker).
pub const WASM_BUDGET_MB: usize = 192;
/// Budget of the native builds (desktop, tests): same code, other number.
pub const NATIVE_BUDGET_MB: usize = 1024;

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

    /// The error of a phase that does not fit even in its smallest slab.
    pub fn refuse(&self, phase: &'static str, estimate: usize) -> Error {
        refusal(phase, estimate, self.cap)
    }
}

/// `error.memoryBudget` with `{fase, stima_mb, budget_mb}`.
pub fn refusal(phase: &'static str, estimate: usize, cap: usize) -> Error {
    Error::memory_budget()
        .with("fase", phase)
        .with("stima_mb", estimate.div_ceil(MB))
        .with("budget_mb", cap / MB)
}

/// An empty `Vec` with room for `n` items, or the memory error (never a trap).
pub fn try_vec<T>(n: usize) -> Result<Vec<T>> {
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| Error::memory_budget())?;
    Ok(v)
}

/// Where an output is written: bytes at an offset. The browser gives an OPFS handle (`write(k, offset, bytes)`), the tests a `Vec`.
pub trait WriteAt {
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()>;
}

impl WriteAt for Vec<u8> {
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()> {
        let (off, end) = (off as usize, off as usize + data.len());
        if self.len() < end {
            self.try_reserve(end - self.len())
                .map_err(|_| Error::memory_budget())?;
            self.resize(end, 0);
        }
        self[off..end].copy_from_slice(data);
        Ok(())
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
        let hex = self.hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
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
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
