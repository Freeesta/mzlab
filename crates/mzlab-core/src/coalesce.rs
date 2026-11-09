//! Write coalescing for outputs whose every call is costly (the browser's OPFS callbacks: a JS call, a sync write and a read-back each time).
//! `Accorpa` wraps an `mzidx::Output` or an `mzidx_spill::Spill` and joins contiguous writes into blocks of up to `BLOCK` bytes, one pending block per
//! stream of writes (the columns of the sections of a `.mzidx` are written in turn, each continuing where it stopped). A write sends first the pending blocks
//! it overlaps, so pending blocks never overlap and the bytes on the destination stay those of direct writes; a read of the scratch area and `fine()` send them all.

use crate::error::Result;
use crate::mzidx::Output;
use crate::mzidx_spill::Spill;

/// Bytes joined before a block is sent (OPFS: 1 MiB writes cost about half of 256 KiB ones, a quarter of 64 KiB ones, per byte).
pub const BLOCK: usize = 1 << 20;
/// Pending blocks of an output: a `.mzidx` interleaves the writes of up to about 20 columns, and 8 streams join them into blocks of about 1 MiB (557 writes become 178 on 422 MB of a `.raw`).
pub const FLUSSI: usize = 8;
/// Every put of the scratch area (`mzidx_spill`) starts a slot of `CHUNK` bytes, is shorter than one and is the only put of its slot: a put that starts
/// at most a slot after the end of the pending block can only start the next slot, so the gap is the unwritten tail of the pending one, filled with zeros.
pub const BUCO: u64 = crate::mzidx_spill::CHUNK as u64;

struct Block {
    at: u64,
    buf: Vec<u8>,
}

impl Block {
    fn end(&self) -> u64 {
        self.at + self.buf.len() as u64
    }
}

/// An output or scratch area whose contiguous writes are sent in blocks. Call `fine()` when the build succeeded: what is still pending is written then.
pub struct Accorpa<O> {
    dest: O,
    /// The pending blocks, the least recently written first.
    blocks: Vec<Block>,
    cap: usize,
    streams: usize,
}

impl<O> Accorpa<O> {
    /// One pending block (the scratch area, whose puts follow each other).
    pub fn new(dest: O) -> Self {
        Self::with_block(dest, BLOCK)
    }
    /// `FLUSSI` pending blocks (the `.mzidx`).
    pub fn flussi(dest: O) -> Self {
        Self::with_streams(dest, BLOCK, FLUSSI)
    }
    pub fn with_block(dest: O, cap: usize) -> Self {
        Self::with_streams(dest, cap, 1)
    }
    pub fn with_streams(dest: O, cap: usize, streams: usize) -> Self {
        Accorpa {
            dest,
            blocks: Vec::new(),
            cap: cap.max(1),
            streams: streams.max(1),
        }
    }
    /// The destination, once everything is written; pending bytes are dropped when `fine()` is not called first.
    pub fn into_inner(self) -> O {
        self.dest
    }
    /// Appends `data` at `offset` to the pending block it continues (across a gap of at most `hole` bytes, filled with zeros) or starts a block,
    /// sending first the blocks the write overlaps and, when there are `streams` already, the least recently written one; sends a block when it is full.
    fn add(
        &mut self,
        offset: u64,
        data: &[u8],
        hole: u64,
        send: fn(&mut O, u64, &[u8]) -> Result<()>,
    ) -> Result<()> {
        let to = offset + data.len() as u64;
        let goes_on = |b: &Block| offset.checked_sub(b.end()).is_some_and(|g| g <= hole);
        let from = self
            .blocks
            .iter()
            .find(|b| goes_on(b))
            .map_or(offset, Block::end);
        let mut i = 0;
        while i < self.blocks.len() {
            let b = &self.blocks[i];
            if !goes_on(b) && b.at < to && b.end() > from {
                let b = self.blocks.remove(i);
                send(&mut self.dest, b.at, &b.buf)?;
            } else {
                i += 1;
            }
        }
        let mut b = match self.blocks.iter().position(goes_on) {
            Some(i) => self.blocks.remove(i),
            None => {
                if data.len() >= self.cap {
                    return send(&mut self.dest, offset, data);
                }
                if self.blocks.len() >= self.streams {
                    let old = self.blocks.remove(0);
                    send(&mut self.dest, old.at, &old.buf)?;
                }
                Block {
                    at: offset,
                    buf: Vec::new(),
                }
            }
        };
        let gap = (offset - b.end()) as usize;
        b.buf.resize(b.buf.len() + gap, 0);
        b.buf.extend_from_slice(data);
        if b.buf.len() >= self.cap {
            return send(&mut self.dest, b.at, &b.buf);
        }
        self.blocks.push(b);
        Ok(())
    }
    fn send(&mut self, send: fn(&mut O, u64, &[u8]) -> Result<()>) -> Result<()> {
        for b in std::mem::take(&mut self.blocks) {
            send(&mut self.dest, b.at, &b.buf)?;
        }
        Ok(())
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
#[path = "coalesce_tests.rs"]
mod tests;
