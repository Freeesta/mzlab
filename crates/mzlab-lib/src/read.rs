//! Reader of `.mzlib` files over any `Source`: only the directories stay in memory; blocks and index pages are decoded on demand and kept
//! in a least-recently-used cache with a byte budget. Every length and offset read from the file is checked against the file size before use.

use crate::build::read_meta_record;
use crate::clean::dequant_mz;
use crate::codec::decompress;
use crate::error::{Error, Result};
use crate::format::*;
use crate::io::Source;
use crate::spec::Meta;
use crate::util::{crc32c, le_u16, le_u32, le_u64, vec_with_capacity, zeroed};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Cache budget in bytes: desktop and ordinary browsers / iPad and phones. The one place where the numbers live.
pub const CACHE_BYTES: usize = 64 << 20;
pub const CACHE_BYTES_SMALL: usize = 32 << 20;

pub fn cache_budget(small_device: bool) -> usize {
    if small_device {
        CACHE_BYTES_SMALL
    } else {
        CACHE_BYTES
    }
}

/// Largest decoded size accepted for a block of each kind.
const MAX_TABLE_RAW: u32 = SPEC_REC_LEN as u32 * RECORDS_PER_TABLE_CHUNK;
const MAX_PEAKS_RAW: u32 = 6 * 4096 * 500;
const MAX_META_RAW: u32 = RECORDS_PER_META_CHUNK * 10 * (crate::spec::MAX_FIELD_BYTES as u32 + 4);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Table,
    Peaks,
    Meta,
    Frag,
    Nl,
}

type Block = (Rc<Vec<u8>>, u64);

struct Cache {
    map: HashMap<(Kind, u32), Block>,
    bytes: usize,
    budget: usize,
    tick: u64,
}

impl Cache {
    fn get(&mut self, k: (Kind, u32)) -> Option<Rc<Vec<u8>>> {
        self.tick += 1;
        let t = self.tick;
        self.map.get_mut(&k).map(|e| {
            e.1 = t;
            e.0.clone()
        })
    }

    fn put(&mut self, k: (Kind, u32), v: Rc<Vec<u8>>) {
        self.tick += 1;
        self.bytes += v.len();
        if let Some(old) = self.map.insert(k, (v, self.tick)) {
            self.bytes -= old.0.len();
        }
        while self.bytes > self.budget && self.map.len() > 1 {
            let oldest = *self
                .map
                .iter()
                .min_by_key(|(_, e)| e.1)
                .map(|(k, _)| k)
                .expect("not empty");
            if oldest == k {
                break;
            }
            if let Some(e) = self.map.remove(&oldest) {
                self.bytes -= e.0.len();
            }
        }
    }
}

struct Chunked {
    off: u64,
    entries: Vec<ChunkEntry>,
}

struct IndexDir {
    off: u64,
    data_off: u64,
    n_entries: u64,
    pages: Vec<(u32, u32, u32)>, // first m/z, count, crc
}

pub struct Library<S: Source> {
    src: S,
    pub header: Header,
    table: Chunked,
    peaks: Chunked,
    meta: Chunked,
    frag: IndexDir,
    nl: IndexDir,
    cache: RefCell<Cache>,
    /// Decoded-chunk reads served from the file (not the cache): lets tests and the bench see the cache work.
    pub block_reads: std::cell::Cell<u64>,
}

fn section(secs: &[SectionEntry], kind: u32) -> Result<&SectionEntry> {
    secs.iter()
        .find(|s| s.kind == kind)
        .ok_or_else(|| Error::corrupt("section"))
}

fn read_vec<S: Source + ?Sized>(src: &S, off: u64, len: u64) -> Result<Vec<u8>> {
    let end = off
        .checked_add(len)
        .ok_or_else(|| Error::corrupt("section"))?;
    if end > src.len() {
        return Err(Error::corrupt("truncated"));
    }
    let mut v = zeroed(usize::try_from(len).map_err(|_| Error::memory_budget())?)?;
    src.read_at(off, &mut v)?;
    Ok(v)
}

impl<S: Source> Library<S> {
    pub fn open(src: S, cache_bytes: usize) -> Result<Self> {
        let flen = src.len();
        if flen < HEADER_LEN as u64 {
            return Err(Error::corrupt("truncated"));
        }
        let mut h = [0u8; HEADER_LEN];
        src.read_at(0, &mut h)?;
        if h[0..4] != MAGIC {
            return Err(Error::new("lib.err.archive"));
        }
        let version = le_u16(&h, 4);
        let min_reader = le_u16(&h, 6);
        if min_reader > VERSION {
            return Err(Error::version(version, min_reader));
        }
        if crc32c(&h[..32]) != le_u32(&h, 32) {
            return Err(Error::corrupt("header"));
        }
        let header = Header {
            version,
            min_reader,
            n_spectra: le_u32(&h, 8),
            n_sections: le_u32(&h, 12),
            n_peaks: le_u64(&h, 16),
            flags: le_u32(&h, 24),
        };
        if header.n_sections == 0
            || header.n_sections > MAX_SECTIONS
            || header.n_spectra == 0
            || header.n_spectra > MAX_SPECTRA
        {
            return Err(Error::corrupt("header"));
        }
        let table_len = HEADER_LEN as u64 + header.n_sections as u64 * SECTION_ENTRY_LEN as u64;
        if table_len + 4 > HEAD_RESERVED.min(flen) {
            return Err(Error::corrupt("header"));
        }
        let head = read_vec(&src, 0, table_len + 4)?;
        if crc32c(&head[..table_len as usize]) != le_u32(&head, table_len as usize) {
            return Err(Error::corrupt("sections"));
        }
        let mut secs = Vec::new();
        for i in 0..header.n_sections as usize {
            let e = SectionEntry::from_bytes(&head[HEADER_LEN + i * SECTION_ENTRY_LEN..]);
            if e.offset.checked_add(e.length).is_none_or(|end| end > flen)
                || e.offset < HEAD_RESERVED
                || e.dir_len as u64 > e.length
            {
                return Err(Error::corrupt("section"));
            }
            secs.push(e);
        }
        let n = header.n_spectra;
        let table = read_chunked(
            &src,
            section(&secs, SEC_SPECTRA)?,
            n.div_ceil(RECORDS_PER_TABLE_CHUNK),
            MAX_TABLE_RAW,
        )?;
        let peaks = read_chunked(
            &src,
            section(&secs, SEC_PEAKS)?,
            n.div_ceil(SPECTRA_PER_PEAK_CHUNK),
            MAX_PEAKS_RAW,
        )?;
        let meta = read_chunked(
            &src,
            section(&secs, SEC_META)?,
            n.div_ceil(RECORDS_PER_META_CHUNK),
            MAX_META_RAW,
        )?;
        let frag = read_index_dir(&src, section(&secs, SEC_FRAG)?)?;
        let nl = read_index_dir(&src, section(&secs, SEC_NL)?)?;
        for (c, per) in [
            (&table, RECORDS_PER_TABLE_CHUNK),
            (&peaks, SPECTRA_PER_PEAK_CHUNK),
            (&meta, RECORDS_PER_META_CHUNK),
        ] {
            for (i, e) in c.entries.iter().enumerate() {
                if e.first != i as u32 * per {
                    return Err(Error::corrupt("directory"));
                }
            }
        }
        if table
            .entries
            .iter()
            .any(|e| e.method != METHOD_RAW || e.stored != e.raw)
        {
            return Err(Error::corrupt("directory"));
        }
        Ok(Library {
            src,
            header,
            table,
            peaks,
            meta,
            frag,
            nl,
            cache: RefCell::new(Cache {
                map: HashMap::new(),
                bytes: 0,
                budget: cache_bytes,
                tick: 0,
            }),
            block_reads: std::cell::Cell::new(0),
        })
    }

    pub fn n_spectra(&self) -> u32 {
        self.header.n_spectra
    }

    pub fn n_peaks(&self) -> u64 {
        self.header.n_peaks
    }

    pub fn cached_bytes(&self) -> usize {
        self.cache.borrow().bytes
    }

    fn chunk(&self, kind: Kind, idx: u32) -> Result<Rc<Vec<u8>>> {
        if let Some(v) = self.cache.borrow_mut().get((kind, idx)) {
            return Ok(v);
        }
        let (c, max) = match kind {
            Kind::Table => (&self.table, MAX_TABLE_RAW),
            Kind::Peaks => (&self.peaks, MAX_PEAKS_RAW),
            Kind::Meta => (&self.meta, MAX_META_RAW),
            _ => unreachable!("index pages have their own reader"),
        };
        let e = c
            .entries
            .get(idx as usize)
            .ok_or_else(|| Error::corrupt("chunk"))?;
        debug_assert!(e.raw <= max);
        let stored = read_vec(&self.src, c.off + e.offset, e.stored as u64)?;
        if crc32c(&stored) != e.crc {
            return Err(Error::corrupt("crc"));
        }
        let raw = Rc::new(decompress(&stored, e.method, e.raw as usize)?);
        self.block_reads.set(self.block_reads.get() + 1);
        self.cache.borrow_mut().put((kind, idx), raw.clone());
        Ok(raw)
    }

    pub fn rec(&self, i: u32) -> Result<SpecRec> {
        if i >= self.header.n_spectra {
            return Err(Error::corrupt("index"));
        }
        let c = self.chunk(Kind::Table, i / RECORDS_PER_TABLE_CHUNK)?;
        let at = (i % RECORDS_PER_TABLE_CHUNK) as usize * SPEC_REC_LEN;
        c.get(at..at + SPEC_REC_LEN)
            .map(SpecRec::from_bytes)
            .ok_or_else(|| Error::corrupt("chunk"))
    }

    /// Quantised peaks of spectrum `i`: m/z in 1e-5 Da units and intensities as fractions of 65535.
    pub fn peaks_q(&self, i: u32) -> Result<(Vec<u32>, Vec<u16>)> {
        let r = self.rec(i)?;
        let ci = i / SPECTRA_PER_PEAK_CHUNK;
        let chunk_first = self.rec(ci * SPECTRA_PER_PEAK_CHUNK)?.first_peak as u64;
        let last = ((ci + 1) * SPECTRA_PER_PEAK_CHUNK).min(self.header.n_spectra) - 1;
        let lr = self.rec(last)?;
        let total = lr.first_peak as u64 + lr.n_peaks as u64 - chunk_first;
        let raw = self.chunk(Kind::Peaks, ci)?;
        if raw.len() as u64 != 6 * total {
            return Err(Error::corrupt("peaks"));
        }
        let start = (r.first_peak as u64)
            .checked_sub(chunk_first)
            .ok_or_else(|| Error::corrupt("peaks"))? as usize;
        let n = r.n_peaks as usize;
        let t = total as usize;
        if start + n > t {
            return Err(Error::corrupt("peaks"));
        }
        let mut mz = Vec::with_capacity(n);
        let mut it = Vec::with_capacity(n);
        let mut acc = 0u32;
        for j in start..start + n {
            let d = u32::from_le_bytes([raw[j], raw[t + j], raw[2 * t + j], raw[3 * t + j]]);
            acc = acc.checked_add(d).ok_or_else(|| Error::corrupt("peaks"))?;
            mz.push(acc);
            it.push(u16::from_le_bytes([raw[4 * t + j], raw[5 * t + j]]));
        }
        Ok((mz, it))
    }

    /// Peaks of spectrum `i` as `f64`: m/z in Da, intensity as a fraction of the highest (≤ 1).
    pub fn peaks(&self, i: u32) -> Result<(Vec<f64>, Vec<f64>)> {
        let (mq, iq) = self.peaks_q(i)?;
        Ok((
            mq.into_iter().map(dequant_mz).collect(),
            iq.into_iter().map(|v| v as f64 / INT_MAX).collect(),
        ))
    }

    pub fn meta(&self, i: u32) -> Result<Meta> {
        if i >= self.header.n_spectra {
            return Err(Error::corrupt("index"));
        }
        let raw = self.chunk(Kind::Meta, i / RECORDS_PER_META_CHUNK)?;
        let mut pos = 0usize;
        let skip = (i % RECORDS_PER_META_CHUNK) as usize;
        for _ in 0..skip {
            for _ in 0..crate::spec::META_FIELDS {
                let l = crate::util::get_varint(&raw, &mut pos)? as usize;
                pos = pos
                    .checked_add(l)
                    .filter(|p| *p <= raw.len())
                    .ok_or_else(|| Error::corrupt("meta"))?;
            }
        }
        read_meta_record(&raw, &mut pos)
    }

    /// First index whose key (polarity key, precursor) is not less than the given one.
    fn lower_bound(&self, pk: u8, prec: f64) -> Result<u32> {
        let less = |e_pk: u8, e_prec: f64| e_pk < pk || (e_pk == pk && e_prec < prec);
        let dir = &self.table.entries;
        // last chunk whose first key is less than the target
        let c = dir.partition_point(|e| less(e.first_pol_key, e.first_prec));
        if c == 0 {
            return Ok(0);
        }
        let ci = (c - 1) as u32;
        let chunk = self.chunk(Kind::Table, ci)?;
        let n_in = chunk.len() / SPEC_REC_LEN;
        let (mut lo, mut hi) = (0usize, n_in);
        while lo < hi {
            let m = (lo + hi) / 2;
            let r = SpecRec::from_bytes(&chunk[m * SPEC_REC_LEN..]);
            if less(pol_key(r.pol), r.prec) {
                lo = m + 1;
            } else {
                hi = m;
            }
        }
        Ok(ci * RECORDS_PER_TABLE_CHUNK + lo as u32)
    }

    /// Spectra of polarity key `pk` (0 negative, 1 unknown, 2 positive) with a precursor in `[lo, hi]`: a contiguous index range.
    pub fn range_for_pol(&self, pk: u8, lo: f64, hi: f64) -> Result<std::ops::Range<u32>> {
        let a = self.lower_bound(pk, lo)?;
        let b = if hi.is_infinite() {
            self.lower_bound(pk + 1, f64::NEG_INFINITY)?
        } else {
            self.upper_bound(pk, hi)?
        };
        Ok(a..b.max(a))
    }

    fn upper_bound(&self, pk: u8, prec: f64) -> Result<u32> {
        // first key greater than (pk, prec): lower bound of the next representable precursor
        self.lower_bound(
            pk,
            f64::from_bits(if prec >= 0.0 {
                prec.to_bits() + 1
            } else {
                prec.to_bits() - 1
            }),
        )
    }

    /// Index ranges to look at for a query of polarity `pol` (+1, -1, 0 = any); spectra of unknown polarity are always kept.
    pub fn ranges(&self, pol: i8, lo: f64, hi: f64) -> Result<Vec<std::ops::Range<u32>>> {
        let keys: &[u8] = match pol {
            p if p > 0 => &[1, 2],
            p if p < 0 => &[0, 1],
            _ => &[0, 1, 2],
        };
        let mut out = Vec::new();
        for &k in keys {
            let r = self.range_for_pol(k, lo, hi)?;
            if !r.is_empty() {
                out.push(r);
            }
        }
        Ok(out)
    }

    // ------------------------------------------------ Flash indexes

    fn index_dir(&self, nl: bool) -> &IndexDir {
        if nl {
            &self.nl
        } else {
            &self.frag
        }
    }

    pub fn index_entries(&self, nl: bool) -> u64 {
        self.index_dir(nl).n_entries
    }

    fn page(&self, nl: bool, p: u32) -> Result<Rc<Vec<u8>>> {
        let kind = if nl { Kind::Nl } else { Kind::Frag };
        if let Some(v) = self.cache.borrow_mut().get((kind, p)) {
            return Ok(v);
        }
        let d = self.index_dir(nl);
        let (_, count, crc) = *d
            .pages
            .get(p as usize)
            .ok_or_else(|| Error::corrupt("page"))?;
        let bytes = read_vec(
            &self.src,
            d.data_off + p as u64 * PAGE_BYTES as u64,
            count as u64 * ENTRY_BYTES as u64,
        )?;
        if crc32c(&bytes) != crc {
            return Err(Error::corrupt("crc"));
        }
        let v = Rc::new(bytes);
        self.block_reads.set(self.block_reads.get() + 1);
        self.cache.borrow_mut().put((kind, p), v.clone());
        Ok(v)
    }

    /// Calls `f(mz_q, spectrum, intensity_q)` for every entry of an index with `lo <= m/z <= hi` (quantised units), in m/z order.
    pub fn for_each_entry(
        &self,
        nl: bool,
        lo: u32,
        hi: u32,
        mut f: impl FnMut(u32, u32, u16),
    ) -> Result<()> {
        let d = self.index_dir(nl);
        if d.pages.is_empty() || lo > hi {
            return Ok(());
        }
        let first = d.pages.partition_point(|p| p.0 < lo).saturating_sub(1);
        for p in first..d.pages.len() {
            if d.pages[p].0 > hi {
                break;
            }
            let b = self.page(nl, p as u32)?;
            let c = d.pages[p].1 as usize;
            for j in 0..c {
                let mz = le_u32(&b, j * 4);
                if mz < lo {
                    continue;
                }
                if mz > hi {
                    break;
                }
                f(mz, le_u32(&b, c * 4 + j * 4), le_u16(&b, c * 8 + j * 2));
            }
        }
        Ok(())
    }

    /// Reads and checks everything (every block and page against its CRC, the peak counts against the table): `Ok` only for a sound file.
    pub fn verify(&self) -> Result<()> {
        let n = self.header.n_spectra;
        let mut peaks = 0u64;
        let mut prev: Option<(u8, f64)> = None;
        for i in 0..n {
            let r = self.rec(i)?;
            let k = (pol_key(r.pol), r.prec);
            if let Some(p) = prev {
                if p.0 > k.0 || (p.0 == k.0 && p.1 > k.1) || r.first_peak as u64 != peaks {
                    return Err(Error::corrupt("table"));
                }
            }
            prev = Some(k);
            peaks += r.n_peaks as u64;
            let (mz, _) = self.peaks_q(i)?;
            if mz.windows(2).any(|w| w[0] >= w[1]) {
                return Err(Error::corrupt("peaks"));
            }
            self.meta(i)?;
        }
        if peaks != self.header.n_peaks {
            return Err(Error::corrupt("peaks"));
        }
        for nl in [false, true] {
            let d = self.index_dir(nl);
            let mut total = 0u64;
            let mut last = 0u32;
            for p in 0..d.pages.len() as u32 {
                let b = self.page(nl, p)?;
                let c = d.pages[p as usize].1 as usize;
                for j in 0..c {
                    let (mz, sp) = (le_u32(&b, j * 4), le_u32(&b, c * 4 + j * 4));
                    if mz < last || sp >= n {
                        return Err(Error::corrupt("index"));
                    }
                    last = mz;
                }
                total += c as u64;
            }
            if total != d.n_entries {
                return Err(Error::corrupt("index"));
            }
        }
        Ok(())
    }
}

fn read_chunked<S: Source>(
    src: &S,
    s: &SectionEntry,
    expect_chunks: u32,
    max_raw: u32,
) -> Result<Chunked> {
    let d = read_vec(src, s.offset, s.dir_len as u64)?;
    if crc32c(&d) != s.crc || d.len() < 8 {
        return Err(Error::corrupt("directory"));
    }
    let n = le_u32(&d, 0);
    if n != expect_chunks || d.len() as u64 != chunk_dir_len(n) {
        return Err(Error::corrupt("directory"));
    }
    let mut entries = vec_with_capacity(n as usize)?;
    for i in 0..n as usize {
        let e = ChunkEntry::from_bytes(&d[8 + i * CHUNK_ENTRY_LEN..]);
        let end = e.offset.checked_add(e.stored as u64);
        if e.raw > max_raw
            || end.is_none_or(|x| x > s.length)
            || e.offset < s.dir_len as u64
            || e.stored > e.raw.saturating_add(1024)
            || e.first_pol_key > 2
        {
            return Err(Error::corrupt("directory"));
        }
        entries.push(e);
    }
    Ok(Chunked {
        off: s.offset,
        entries,
    })
}

fn read_index_dir<S: Source>(src: &S, s: &SectionEntry) -> Result<IndexDir> {
    let d = read_vec(src, s.offset, s.dir_len as u64)?;
    if crc32c(&d) != s.crc || d.len() < 16 {
        return Err(Error::corrupt("directory"));
    }
    let n_entries = le_u64(&d, 0);
    let n_pages = le_u32(&d, 8);
    let epp = le_u32(&d, 12);
    if epp != ENTRIES_PER_PAGE
        || d.len() as u64 != page_dir_len(n_pages)
        || n_entries.div_ceil(epp as u64) != n_pages as u64
    {
        return Err(Error::corrupt("directory"));
    }
    let mut pages = vec_with_capacity(n_pages as usize)?;
    let mut sum = 0u64;
    for i in 0..n_pages as usize {
        let at = 16 + i * PAGE_DIR_ENTRY_LEN;
        let p = (le_u32(&d, at), le_u32(&d, at + 4), le_u32(&d, at + 8));
        if p.1 == 0 || p.1 > epp || (i + 1 < n_pages as usize && p.1 != epp) {
            return Err(Error::corrupt("directory"));
        }
        sum += p.1 as u64;
        pages.push(p);
    }
    if sum != n_entries {
        return Err(Error::corrupt("directory"));
    }
    let data_off = s.offset + s.dir_len as u64;
    let need = if n_pages == 0 {
        0
    } else {
        (n_pages as u64 - 1) * PAGE_BYTES as u64
            + pages[n_pages as usize - 1].1 as u64 * ENTRY_BYTES as u64
    };
    if data_off + need > s.offset + s.length {
        return Err(Error::corrupt("section"));
    }
    Ok(IndexDir {
        off: s.offset,
        data_off,
        n_entries,
        pages,
    })
}

#[allow(dead_code)]
fn _unused(d: &IndexDir) -> u64 {
    d.off
}
