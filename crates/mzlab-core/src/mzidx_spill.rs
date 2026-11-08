//! Scratch file of the `.mzidx` builder (`docs/agenti/mzidx.md`, «Spillover»): when the peaks of pass 1 do not fit in the budget they are not dropped
//! (the stream would be decoded again for every round) but written to a scratch file, split by m/z into `PARTS` partitions. A round of the builder then
//! reads only the partitions that hold its buckets, in the order the peaks were written (the order of the scans). One decoding of the stream in all.
//! The scratch file is where the caller says (a second file of OPFS in the browser, a temporary file on the desktop): it is not part of the index.

use std::io::{Read, Seek, SeekFrom, Write};

use crate::error::{Error, Result};
use crate::mzidx::{bucket, NB};

/// A scratch area with random access: written at offsets, read back at offsets. Files and `Cursor<Vec<u8>>` implement it; the browser gives callbacks.
pub trait Spill {
    fn put(&mut self, offset: u64, data: &[u8]) -> Result<()>;
    fn get(&mut self, offset: u64, buf: &mut [u8]) -> Result<()>;
}

fn io_error(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

impl<T: Read + Write + Seek> Spill for T {
    fn put(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        self.seek(SeekFrom::Start(offset)).map_err(io_error)?;
        self.write_all(data).map_err(io_error)
    }
    fn get(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.seek(SeekFrom::Start(offset)).map_err(io_error)?;
        self.read_exact(buf).map_err(io_error)
    }
}

/// Half of a scratch file shared by two builders (`quale` 0 or 1): the slots of the two alternate, so each sees its own file and neither knows about the other.
pub struct Meta<'a, 'b> {
    pub sp: &'a std::cell::RefCell<&'b mut dyn Spill>,
    pub quale: u64,
}

impl Meta<'_, '_> {
    fn dove(&self, offset: u64) -> u64 {
        let c = CHUNK as u64;
        (offset / c * 2 + self.quale) * c + offset % c
    }
}

impl Spill for Meta<'_, '_> {
    fn put(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        self.sp.borrow_mut().put(self.dove(offset), data)
    }
    fn get(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.sp.borrow_mut().get(self.dove(offset), buf)
    }
}

/// Bytes of one slot of the scratch file.
pub(crate) const CHUNK: usize = 64 << 10;
/// One peak, little endian: m/z, intensity, number of the scan (u32). Two kinds, kept in separate partitions: 64-bit values (20 B) and the 32-bit
/// values of an MS1 section of high resolution (12 B, already rounded as the section writes them). A scan only has one kind, so the order of the
/// scans is kept within each kind.
const REC: [usize; 2] = [20, 12];
const PER_CHUNK: [usize; 2] = [CHUNK / REC[0], CHUNK / REC[1]];
/// Partitions of the m/z axis (8 MB of write buffers).
const PARTS: usize = 64;

/// The peaks of pass 1 after the budget ran out, split by m/z (the bounds follow the peaks seen up to then, so the partitions are about as big as each other).
pub(crate) struct Parti {
    /// The first bucket of every partition (`PARTS + 1` values, the last is `NB`).
    first: Vec<usize>,
    of_bucket: Vec<u8>,
    /// Index `kind * PARTS + partition`.
    bufs: Vec<Vec<u8>>,
    /// The slots of the scratch file that hold each partition, in writing order.
    slots: Vec<Vec<u32>>,
    records: Vec<u64>,
    next_slot: u32,
    /// Whether 64-bit peaks of MS1 scans were written: an MS1 section of 32 bits may take one into the bucket above its own (`leggi_parti`).
    ms1_larghe: bool,
}

/// The first bucket of every partition for the peaks counted so far.
fn confini(counts: &[u64]) -> Vec<usize> {
    let total: u64 = counts.iter().sum();
    let mut first = vec![0usize; PARTS + 1];
    let (mut run, mut i) = (0u64, 1usize);
    for (b, &c) in counts.iter().enumerate().take(NB) {
        run += c;
        while i < PARTS && run * PARTS as u64 >= total * i as u64 {
            first[i] = b + 1;
            i += 1;
        }
    }
    while i < PARTS {
        first[i] = NB;
        i += 1;
    }
    first[PARTS] = NB;
    first
}

impl Parti {
    /// `counts`: the peaks seen so far in every 1-unit bucket of m/z.
    pub(crate) fn new(counts: &[u64]) -> Parti {
        let first = confini(counts);
        let mut of_bucket = vec![0u8; NB];
        for (p, w) in first.windows(2).enumerate() {
            if let Some(cells) = of_bucket.get_mut(w[0]..w[1]) {
                cells.fill(p as u8);
            }
        }
        Parti {
            first,
            of_bucket,
            bufs: (0..2 * PARTS).map(|_| Vec::new()).collect(),
            slots: vec![Vec::new(); 2 * PARTS],
            records: vec![0; 2 * PARTS],
            next_slot: 0,
            ms1_larghe: false,
        }
    }

    /// `p`: `kind * PARTS + partition`.
    fn flush(&mut self, sp: &mut dyn Spill, p: usize) -> Result<()> {
        let buf = &mut self.bufs[p];
        if buf.is_empty() {
            return Ok(());
        }
        sp.put(u64::from(self.next_slot) * CHUNK as u64, buf)?;
        self.slots[p].push(self.next_slot);
        self.next_slot += 1;
        buf.clear();
        Ok(())
    }

    fn buffer(&mut self, kind: usize, p: usize) -> Result<&mut Vec<u8>> {
        let buf = &mut self.bufs[kind * PARTS + p];
        if buf.capacity() == 0 {
            buf.try_reserve_exact(CHUNK)
                .map_err(|_| Error::memory_budget())?;
        }
        Ok(buf)
    }
}

/// One record (`m/z`, intensity, scan, little-endian) made on the stack and appended in one copy: pass 1 appends one per spilled peak.
#[allow(clippy::indexing_slicing)] // `R == 2 * W + 4`, the two record widths of `REC`
fn record<const R: usize, const W: usize>(mz: [u8; W], it: [u8; W], pos: u32) -> [u8; R] {
    let mut r = [0; R];
    r[..W].copy_from_slice(&mz);
    r[W..2 * W].copy_from_slice(&it);
    r[2 * W..].copy_from_slice(&pos.to_le_bytes());
    r
}

fn f64_at(r: &[u8], at: usize) -> f64 {
    r.get(at..at + 8)
        .and_then(|b| b.try_into().ok())
        .map_or(0.0, f64::from_le_bytes)
}

fn f32_at(r: &[u8], at: usize) -> f32 {
    r.get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map_or(0.0, f32::from_le_bytes)
}

fn u32_at(r: &[u8], at: usize) -> u32 {
    r.get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map_or(0, u32::from_le_bytes)
}

impl Parti {
    /// A peak kept in 64 bits; `ms1`: of an MS1 scan.
    pub(crate) fn push(
        &mut self,
        sp: &mut dyn Spill,
        mz: f64,
        it: f64,
        pos: u32,
        ms1: bool,
    ) -> Result<()> {
        self.ms1_larghe |= ms1;
        let p = usize::from(self.of_bucket[bucket(mz)]);
        let buf = self.buffer(0, p)?;
        buf.extend_from_slice(&record::<20, 8>(mz.to_le_bytes(), it.to_le_bytes(), pos));
        let full = buf.len() >= PER_CHUNK[0] * REC[0];
        self.records[p] += 1;
        if full {
            self.flush(sp, p)?;
        }
        Ok(())
    }

    /// A peak of an MS1 section kept in 32 bits: the partition is the one of the rounded m/z.
    pub(crate) fn push32(&mut self, sp: &mut dyn Spill, mz: f32, it: f32, pos: u32) -> Result<()> {
        let p = usize::from(self.of_bucket[bucket(f64::from(mz))]);
        let buf = self.buffer(1, p)?;
        buf.extend_from_slice(&record::<12, 4>(mz.to_le_bytes(), it.to_le_bytes(), pos));
        let full = buf.len() >= PER_CHUNK[1] * REC[1];
        self.records[PARTS + p] += 1;
        if full {
            self.flush(sp, PARTS + p)?;
        }
        Ok(())
    }

    /// The end of pass 1: the partial slots go to the scratch file.
    pub(crate) fn fine(&mut self, sp: &mut dyn Spill) -> Result<()> {
        for p in 0..2 * PARTS {
            self.flush(sp, p)?;
            self.bufs[p] = Vec::new();
        }
        Ok(())
    }
}

/// With a scratch file (`parti`): `hi` lowered to the last partition boundary in `lo + 1..=hi` (none: `hi`), so that every partition is read by one round when the rounds can hold one;
/// `used` lowered by the `cost` of the buckets left out. At the end of the axis `hi` stays.
pub(crate) fn allinea(
    parti: Option<&Parti>,
    lo: usize,
    hi: usize,
    used: usize,
    cost: &dyn Fn(usize) -> usize,
) -> (usize, usize) {
    let Some(first) = parti.map(|p| &p.first) else {
        return (hi, used);
    };
    match first
        .partition_point(|&f| f <= hi)
        .checked_sub(1)
        .and_then(|i| first.get(i).copied())
    {
        Some(b) if hi < NB && b > lo => (b, used - (b..hi).map(cost).sum::<usize>()),
        _ => (hi, used),
    }
}

/// Calls `f(mz, intensity, scan)` for every peak of the partitions that may hold buckets `lo..hi`, partition by partition, each in the order it was
/// written; `f` keeps the peaks whose bucket it wants. The 32-bit peaks are split by the bucket their section takes; a 64-bit peak of an MS1 scan
/// rounded to 32 bits by its section can only go up one bucket (`f32` holds every integer m/z and rounding keeps the order), so bucket `lo - 1` is read too.
pub(crate) fn leggi_parti(
    parti: &Parti,
    sp: &mut dyn Spill,
    lo: usize,
    hi: usize,
    f: &mut dyn FnMut(f64, f64, u32) -> Result<()>,
) -> Result<()> {
    let lo0 = lo.saturating_sub(usize::from(parti.ms1_larghe));
    let mut buf = Vec::new();
    buf.try_reserve_exact(CHUNK)
        .map_err(|_| Error::memory_budget())?;
    buf.resize(CHUNK, 0);
    for p in 0..PARTS {
        if parti.first[p] >= parti.first[p + 1] || parti.first[p] >= hi {
            continue;
        }
        for kind in 0..2 {
            if parti.first[p + 1] <= [lo0, lo][kind] {
                continue;
            }
            let q = kind * PARTS + p;
            let mut left = parti.records[q];
            for &slot in &parti.slots[q] {
                let n = left.min(PER_CHUNK[kind] as u64) as usize;
                let bytes = &mut buf[..n * REC[kind]];
                sp.get(u64::from(slot) * CHUNK as u64, bytes)?;
                for r in bytes.chunks_exact(REC[kind]) {
                    if kind == 0 {
                        f(f64_at(r, 0), f64_at(r, 8), u32_at(r, 16))?;
                    } else {
                        f(
                            f64::from(f32_at(r, 0)),
                            f64::from(f32_at(r, 4)),
                            u32_at(r, 8),
                        )?;
                    }
                }
                left -= n as u64;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "mzidx_spill_prove.rs"]
mod prove;
