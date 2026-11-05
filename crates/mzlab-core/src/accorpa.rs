//! Write coalescing for outputs whose every call is costly (the browser's OPFS callbacks: a JS call, a sync write and a read-back each time).
//! `Accorpa` wraps an `mzidx::Output` or an `mzidx_spill::Spill` and joins contiguous writes into blocks of up to `BLOCCO` bytes; a write that
//! does not follow the pending block, a read of the scratch area and `fine()` send the pending block first, so the bytes and their order
//! on the destination stay the same. Measurements in `docs/agenti/misure-velocita.md`.

use crate::error::Result;
use crate::mzidx::Output;
use crate::mzidx_spill::Spill;

/// Bytes joined before a block is sent (OPFS: 1 MiB writes cost about half of 256 KiB ones, a quarter of 64 KiB ones, per byte).
pub const BLOCCO: usize = 1 << 20;
/// Every put of the scratch area (`mzidx_spill`) starts a slot of `CHUNK` bytes, is shorter than one and is the only put of its slot: a put that starts
/// at most a slot after the end of the pending block can only start the next slot, so the gap is the unwritten tail of the pending one, filled with zeros.
pub const BUCO: u64 = crate::mzidx_spill::CHUNK as u64;

/// An output or scratch area whose contiguous writes are sent in blocks. Call `fine()` when the build succeeded: what is still pending is written then.
pub struct Accorpa<O> {
    dest: O,
    at: u64,
    buf: Vec<u8>,
    cap: usize,
}

impl<O> Accorpa<O> {
    pub fn new(dest: O) -> Self {
        Self::with_block(dest, BLOCCO)
    }
    pub fn with_block(dest: O, cap: usize) -> Self {
        Accorpa {
            dest,
            at: 0,
            buf: Vec::new(),
            cap: cap.max(1),
        }
    }
    /// The destination, once everything is written; pending bytes are dropped when `fine()` is not called first.
    pub fn into_inner(self) -> O {
        self.dest
    }
    /// Appends `data` at `offset` to the pending block, sending it with `send` first when `data` does not follow it; sends the block when it is full.
    /// `hole`: the gap, in bytes, that may still join (zeros).
    fn add(
        &mut self,
        offset: u64,
        data: &[u8],
        hole: u64,
        send: fn(&mut O, u64, &[u8]) -> Result<()>,
    ) -> Result<()> {
        if !self.buf.is_empty() {
            let end = self.at + self.buf.len() as u64;
            match offset.checked_sub(end) {
                Some(g) if g <= hole => self.buf.resize(self.buf.len() + g as usize, 0),
                _ => self.send(send)?,
            }
        }
        if self.buf.is_empty() {
            if data.len() >= self.cap {
                return send(&mut self.dest, offset, data);
            }
            self.at = offset;
        }
        self.buf.extend_from_slice(data);
        if self.buf.len() >= self.cap {
            self.send(send)?;
        }
        Ok(())
    }
    fn send(&mut self, send: fn(&mut O, u64, &[u8]) -> Result<()>) -> Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let r = send(&mut self.dest, self.at, &self.buf);
        self.buf.clear();
        r
    }
}

fn out<O: Output>(o: &mut O, offset: u64, data: &[u8]) -> Result<()> {
    o.write_at(offset, data)
}

fn put<S: Spill>(s: &mut S, offset: u64, data: &[u8]) -> Result<()> {
    s.put(offset, data)
}

impl<O: Output> Accorpa<O> {
    /// Writes what is pending.
    pub fn fine(&mut self) -> Result<()> {
        self.send(out::<O>)
    }
}

impl<O: Output> Output for Accorpa<O> {
    fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        self.add(offset, data, 0, out::<O>)
    }
}

/// As a scratch area: puts are joined like the writes of an output (across the slack at the end of a slot, `BUCO`); a get sends the pending block first (it may hold the bytes asked for).
impl<S: Spill> Spill for Accorpa<S> {
    fn put(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        self.add(offset, data, BUCO, put::<S>)
    }
    fn get(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.send(put::<S>)?;
        self.dest.get(offset, buf)
    }
}

#[cfg(test)]
#[path = "accorpa_tests.rs"]
mod tests;
