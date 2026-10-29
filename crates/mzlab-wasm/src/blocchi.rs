//! Block-wise file reader over the `read(offset, length)` function the worker gives (OPFS/Blob slices), used by every
//! engine route that parses a whole file (mzML, `.raw`, index build).
use js_sys::{Function, Uint8Array};
use std::io::{BufRead, Read, Seek, SeekFrom};
use wasm_bindgen::prelude::*;

/// Where `BlockReader` gets its blocks from: JS in the worker, a closure over memory in the tests.
pub(crate) trait Fetch {
    /// Replaces `into` with the `want` bytes (fewer at the end of the file) found at `pos`.
    fn fetch(&mut self, pos: u64, want: u64, into: &mut Vec<u8>) -> std::io::Result<()>;
}

/// `read(offset, length) -> Uint8Array`: a slice of the Blob read synchronously in the worker.
pub(crate) struct JsFetch(Function);

impl Fetch for JsFetch {
    fn fetch(&mut self, pos: u64, want: u64, into: &mut Vec<u8>) -> std::io::Result<()> {
        let block = self
            .0
            .call2(&JsValue::NULL, &(pos as f64).into(), &(want as f64).into())
            .map_err(|_| std::io::Error::other("block read failed"))?;
        let block: Uint8Array = block
            .dyn_into()
            .map_err(|_| std::io::Error::other("block is not a Uint8Array"))?;
        into.clear();
        into.resize(block.length() as usize, 0);
        block.copy_to(into);
        Ok(())
    }
}

/// Reads a file in blocks asked to JS (see `Fetch`).
/// It also seeks (the `.raw` reader jumps from packet to packet): the last block stays in memory, so scans that follow each
/// other cost one JS call per block, never one per read. It is a `BufRead` itself: a caller that wants buffering reads
/// straight from the block, with no second copy of the file.
pub(crate) struct BlockReader<F: Fetch = JsFetch> {
    src: F,
    pos: u64,
    size: u64,
    block: Vec<u8>,
    block_start: u64,
}

impl<F: Fetch> BlockReader<F> {
    fn with_fetch(src: F, size: u64) -> Self {
        BlockReader {
            src,
            pos: 0,
            size,
            block: Vec::new(),
            block_start: 0,
        }
    }

    fn fill(&mut self) -> std::io::Result<()> {
        let want = (BLOCK as u64).min(self.size - self.pos);
        self.src.fetch(self.pos, want, &mut self.block)?;
        self.block_start = self.pos;
        Ok(())
    }
}

pub(crate) const BLOCK: usize = 4 << 20; // 4 MiB: ~2.5x faster OPFS reads than 1 MiB in Chromium (docs/studi/wasm-potenza.md 3e); JS buffers grow to it

/// A `BlockReader` over the `read(offset, length)` function the worker gives.
pub(crate) fn js_blocks(read: Function, size: u64) -> BlockReader {
    BlockReader::with_fetch(JsFetch(read), size)
}

impl<F: Fetch> BufRead for BlockReader<F> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.pos >= self.size {
            return Ok(&[]);
        }
        let end = self.block_start + self.block.len() as u64;
        if self.pos < self.block_start || self.pos >= end {
            self.fill()?;
        }
        let off = ((self.pos - self.block_start) as usize).min(self.block.len());
        Ok(&self.block[off..])
    }

    fn consume(&mut self, amt: usize) {
        self.pos += amt as u64;
    }
}

impl<F: Fetch> Read for BlockReader<F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let avail = self.fill_buf()?;
        let n = buf.len().min(avail.len());
        buf[..n].copy_from_slice(&avail[..n]);
        self.consume(n);
        Ok(n)
    }
}

impl<F: Fetch> Seek for BlockReader<F> {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let p = match to {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => self.size.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = p.ok_or_else(|| std::io::Error::other("seek before the start"))?;
        Ok(self.pos)
    }
}

#[cfg(test)]
#[path = "blocchi_prove.rs"]
mod tests;
