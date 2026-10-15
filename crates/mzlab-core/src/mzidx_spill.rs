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
const CHUNK: usize = 128 << 10;
/// One peak: m/z (f64), intensity (f64), number of the scan (u32), little endian.
const REC: usize = 20;
const PER_CHUNK: usize = CHUNK / REC;
/// Partitions of the m/z axis (8 MB of write buffers).
const PARTS: usize = 64;

/// The peaks of pass 1 after the budget ran out, split by m/z (the bounds follow the peaks seen up to then, so the partitions are about as big as each other).
pub(crate) struct Parti {
    /// The first bucket of every partition (`PARTS + 1` values, the last is `NB`).
    first: Vec<usize>,
    of_bucket: Vec<u8>,
    bufs: Vec<Vec<u8>>,
    /// The slots of the scratch file that hold each partition, in writing order.
    slots: Vec<Vec<u32>>,
    records: Vec<u64>,
    next_slot: u32,
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
            bufs: (0..PARTS).map(|_| Vec::new()).collect(),
            slots: vec![Vec::new(); PARTS],
            records: vec![0; PARTS],
            next_slot: 0,
        }
    }

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

    pub(crate) fn push(&mut self, sp: &mut dyn Spill, mz: f64, it: f64, pos: u32) -> Result<()> {
        let p = usize::from(self.of_bucket[bucket(mz)]);
        let buf = &mut self.bufs[p];
        if buf.capacity() == 0 {
            buf.try_reserve_exact(CHUNK)
                .map_err(|_| Error::memory_budget())?;
        }
        buf.extend_from_slice(&mz.to_le_bytes());
        buf.extend_from_slice(&it.to_le_bytes());
        buf.extend_from_slice(&pos.to_le_bytes());
        self.records[p] += 1;
        if buf.len() >= PER_CHUNK * REC {
            self.flush(sp, p)?;
        }
        Ok(())
    }

    /// The end of pass 1: the partial slots go to the scratch file.
    pub(crate) fn fine(&mut self, sp: &mut dyn Spill) -> Result<()> {
        for p in 0..PARTS {
            self.flush(sp, p)?;
            self.bufs[p] = Vec::new();
        }
        Ok(())
    }
}

/// Calls `f(mz, intensity, scan)` for every peak of the partitions that may hold buckets `lo..hi` (one more on each side: a bucket taken after rounding
/// the m/z to 32 bits can be the next one), partition by partition, each in the order it was written. `f` keeps the peaks whose bucket it wants.
pub(crate) fn leggi_parti(
    parti: &Parti,
    sp: &mut dyn Spill,
    lo: usize,
    hi: usize,
    f: &mut dyn FnMut(f64, f64, u32) -> Result<()>,
) -> Result<()> {
    let (lo, hi) = (lo.saturating_sub(1), hi.saturating_add(1));
    let mut buf = Vec::new();
    buf.try_reserve_exact(CHUNK)
        .map_err(|_| Error::memory_budget())?;
    buf.resize(CHUNK, 0);
    for p in 0..PARTS {
        if parti.first[p] >= parti.first[p + 1] || parti.first[p] >= hi || parti.first[p + 1] <= lo
        {
            continue;
        }
        let mut left = parti.records[p];
        for &slot in &parti.slots[p] {
            let n = left.min(PER_CHUNK as u64) as usize;
            let bytes = &mut buf[..n * REC];
            sp.get(u64::from(slot) * CHUNK as u64, bytes)?;
            for r in bytes.chunks_exact(REC) {
                let (mz, rest) = r.split_at(8);
                let (it, pos) = rest.split_at(8);
                f(
                    f64::from_le_bytes(mz.try_into().unwrap_or([0; 8])),
                    f64::from_le_bytes(it.try_into().unwrap_or([0; 8])),
                    u32::from_le_bytes(pos.try_into().unwrap_or([0; 4])),
                )?;
            }
            left -= n as u64;
        }
    }
    Ok(())
}
