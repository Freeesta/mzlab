//! Reader of `.mzlib` files over any `Source`: only the directories stay in memory; blocks and index pages are decoded on demand and kept
//! in a least-recently-used cache with a byte budget. Every length and offset read from the file is checked against the file size before use.

use crate::build::read_meta_record;
use crate::clean::dequant_mz;
use crate::codec::decompress;
use crate::error::{Error, Result};
use crate::format::*;
use crate::io::Source;
use crate::libinfo::LibInfo;
use crate::sha256::sha256;
use crate::spec::Meta;
use crate::util::{crc32c, le_u16, le_u32, le_u64, vec_with_capacity, zeroed};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

mod dirs;
mod entries;
mod verify;
use dirs::{read_chunked, read_index_dir, read_section_table, read_v2_parts};

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

/// Widens a small format constant at compile time (the assert fails the build if it ever grows past 16 bits).
#[allow(clippy::cast_possible_truncation)]
const fn small(x: usize) -> u32 {
    assert!(x < 1 << 16);
    x as u32
}

fn to_u32(x: usize) -> Result<u32> {
    u32::try_from(x).map_err(|_| Error::corrupt("size"))
}

/// `b[from..]`, or a corruption error when the offset read from the file is out of range.
fn tail<'a>(b: &'a [u8], from: usize, what: &'static str) -> Result<&'a [u8]> {
    b.get(from..).ok_or_else(|| Error::corrupt(what))
}

fn to_usize(x: u64) -> Result<usize> {
    usize::try_from(x).map_err(|_| Error::corrupt("size"))
}

/// Largest decoded size accepted for a block of each kind.
const MAX_TABLE_RAW: u32 = small(SPEC_REC_LEN) * RECORDS_PER_TABLE_CHUNK;
const MAX_PEAKS_RAW: u32 = 6 * SPECTRA_PER_PEAK_CHUNK * 500;
const MAX_META_RAW: u32 = RECORDS_PER_META_CHUNK * 10 * (small(crate::spec::MAX_FIELD_BYTES) + 4);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Table,
    Peaks,
    Meta,
    Frag,
    Nl,
}

type Block = (Rc<Vec<u8>>, u64);
/// One decoded chunk and its index.
type Held = Option<(u32, Rc<Vec<u8>>)>;

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

/// Per-spectrum numbers the Flash searches need for every index entry (8 bytes per spectrum), read once from the table on first use.
pub struct Light {
    /// Highest entropy-weighted intensity of each spectrum.
    pub scale: Vec<f32>,
    /// Natural logarithm of `scale` (0 where the scale is not positive): lets the Flash score take `ln` of an index intensity from a table.
    pub ln_scale: Vec<f64>,
    /// Precursor in 1e-5 Da units (`round(prec * 1e5)`), the value the neutral-loss index is built from.
    pub precq: Vec<u32>,
}

pub struct Library<S: Source> {
    src: S,
    pub header: Header,
    table: Chunked,
    peaks: Chunked,
    meta: Chunked,
    frag: IndexDir,
    nl: IndexDir,
    /// Record length of the spectra table (32 in v1 files, 36 in v2).
    rec_len: usize,
    info: LibInfo,
    /// The per-spectrum InChIKey hashes (v2): where they are, and the decoded array once asked for.
    inchi_at: Option<SectionEntry>,
    inchi: std::cell::OnceCell<Vec<u64>>,
    /// SHA-256 of the header and the section table, which carry the CRC of every directory: changes whenever the content does.
    fingerprint: [u8; 32],
    cache: RefCell<Cache>,
    /// Blocks and pages whose CRC was already checked: a block that was evicted from the cache is read again without a second pass over its bytes.
    verified: RefCell<std::collections::HashSet<(Kind, u32)>>,
    /// The last chunk used of each kind (table, peaks, meta): consecutive spectra of a scan sit in the same chunk, and this skips the hashed cache lookup.
    recent: RefCell<[Held; 3]>,
    light: std::cell::OnceCell<Light>,
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
        let reserved = head_reserved(version);
        if table_len + 4 > reserved.min(flen) {
            return Err(Error::corrupt("header"));
        }
        let head = read_vec(&src, 0, table_len + 4)?;
        let tl = to_usize(table_len)?;
        if !head
            .get(..tl)
            .is_some_and(|h| crc32c(h) == le_u32(&head, tl))
        {
            return Err(Error::corrupt("sections"));
        }
        let secs = read_section_table(&head, header.n_sections, flen, reserved)?;
        let n = header.n_spectra;
        let rec_len = spec_rec_len(version);
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
                if Some(e.first) != to_u32(i)?.checked_mul(per) {
                    return Err(Error::corrupt("directory"));
                }
            }
        }
        if table.entries.iter().any(|e| {
            e.method != METHOD_RAW || e.stored != e.raw || !(e.raw as usize).is_multiple_of(rec_len)
        }) {
            return Err(Error::corrupt("directory"));
        }
        let (info, inchi_at) = read_v2_parts(&src, &secs, n)?;
        Ok(Library {
            src,
            header,
            table,
            peaks,
            meta,
            frag,
            nl,
            rec_len,
            info,
            inchi_at,
            inchi: std::cell::OnceCell::new(),
            fingerprint: sha256(&[&head]),
            cache: RefCell::new(Cache {
                map: HashMap::new(),
                bytes: 0,
                budget: cache_bytes,
                tick: 0,
            }),
            verified: RefCell::new(std::collections::HashSet::new()),
            recent: RefCell::new([None, None, None]),
            light: std::cell::OnceCell::new(),
            block_reads: std::cell::Cell::new(0),
        })
    }

    /// The light table (built on first call, outside the block cache).
    pub fn light(&self) -> Result<&Light> {
        if let Some(l) = self.light.get() {
            return Ok(l);
        }
        let n = self.header.n_spectra as usize;
        let mut scale = vec_with_capacity::<f32>(n)?;
        let mut ln_scale = vec_with_capacity::<f64>(n)?;
        let mut precq = vec_with_capacity::<u32>(n)?;
        for (ci, e) in self.table.entries.iter().enumerate() {
            let raw = match self.cache.borrow_mut().get((Kind::Table, to_u32(ci)?)) {
                Some(r) => r,
                None => {
                    let stored = read_vec(&self.src, self.table.off + e.offset, e.stored as u64)?;
                    if crc32c(&stored) != e.crc {
                        return Err(Error::corrupt("crc"));
                    }
                    Rc::new(decompress(&stored, e.method, e.raw as usize)?)
                }
            };
            for r in raw.chunks_exact(self.rec_len) {
                let rec = SpecRec::from_bytes(r);
                scale.push(rec.scale);
                ln_scale.push(if rec.scale > 0.0 {
                    (rec.scale as f64).ln()
                } else {
                    0.0
                });
                precq.push(crate::clean::quant_mz(rec.prec));
            }
        }
        if scale.len() != n {
            return Err(Error::corrupt("table"));
        }
        let _ = self.light.set(Light {
            scale,
            ln_scale,
            precq,
        });
        Ok(self.light.get().expect("just set"))
    }

    /// What the library says about itself; empty for a v1 file.
    pub fn info(&self) -> &LibInfo {
        &self.info
    }

    /// Identifies the content of this file (see the field): equal fingerprints, equal libraries.
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    /// InChIKey hash of every spectrum in table order (0 = none); all zeros for a v1 file. Read and checked on first use.
    pub fn inchikeys(&self) -> Result<&[u64]> {
        if let Some(v) = self.inchi.get() {
            return Ok(v);
        }
        let n = self.header.n_spectra as usize;
        let mut v = vec_with_capacity::<u64>(n)?;
        match self.inchi_at {
            Some(e) => {
                let b = read_vec(&self.src, e.offset, e.dir_len as u64)?;
                if crc32c(&b) != e.crc {
                    return Err(Error::corrupt("inchikey"));
                }
                v.extend(b.chunks_exact(8).map(|c| le_u64(c, 0)));
            }
            None => v.resize(n, 0),
        }
        let _ = self.inchi.set(v);
        Ok(self.inchi.get().expect("just set"))
    }

    pub fn n_spectra(&self) -> u32 {
        self.header.n_spectra
    }

    pub fn n_peaks(&self) -> u64 {
        self.header.n_peaks
    }

    /// Changes the cache budget (the page splits `cache_budget` among the libraries that are open); the excess is dropped at once.
    pub fn set_cache_budget(&self, bytes: usize) {
        let mut c = self.cache.borrow_mut();
        c.budget = bytes;
        while c.bytes > c.budget && !c.map.is_empty() {
            let oldest = *c
                .map
                .iter()
                .min_by_key(|(_, e)| e.1)
                .map(|(k, _)| k)
                .expect("not empty");
            if let Some(e) = c.map.remove(&oldest) {
                c.bytes -= e.0.len();
            }
        }
    }

    pub fn cached_bytes(&self) -> usize {
        self.cache.borrow().bytes
    }

    fn chunk(&self, kind: Kind, idx: u32) -> Result<Rc<Vec<u8>>> {
        let slot = match kind {
            Kind::Table => 0,
            Kind::Peaks => 1,
            _ => 2,
        };
        if let Some(Some((i, v))) = self.recent.borrow().get(slot) {
            if *i == idx {
                return Ok(v.clone());
            }
        }
        let v = self.chunk_cached(kind, idx)?;
        if let Some(h) = self.recent.borrow_mut().get_mut(slot) {
            *h = Some((idx, v.clone()));
        }
        Ok(v)
    }

    /// Checks the CRC of a block the first time it is read; later reads of the same block (after an eviction) trust the first check.
    fn check_crc(&self, id: (Kind, u32), bytes: &[u8], crc: u32) -> Result<()> {
        if self.verified.borrow().contains(&id) {
            return Ok(());
        }
        if crc32c(bytes) != crc {
            return Err(Error::corrupt("crc"));
        }
        self.verified.borrow_mut().insert(id);
        Ok(())
    }

    fn chunk_cached(&self, kind: Kind, idx: u32) -> Result<Rc<Vec<u8>>> {
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
        self.check_crc((kind, idx), &stored, e.crc)?;
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
        let at = (i % RECORDS_PER_TABLE_CHUNK) as usize * self.rec_len;
        c.get(at..at + self.rec_len)
            .map(SpecRec::from_bytes)
            .ok_or_else(|| Error::corrupt("chunk"))
    }

    /// Calls `f(m/z in 1e-5 Da units, intensity as a fraction of 65535)` for each peak of spectrum `i`, in m/z order.
    fn each_peak_q(&self, i: u32, mut f: impl FnMut(u32, u16)) -> Result<()> {
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
        let start = to_usize(
            (r.first_peak as u64)
                .checked_sub(chunk_first)
                .ok_or_else(|| Error::corrupt("peaks"))?,
        )?;
        let n = r.n_peaks as usize;
        let t = to_usize(total)?;
        if start + n > t {
            return Err(Error::corrupt("peaks"));
        }
        let mut acc = 0u32;
        // Column `c` of the byte-plane layout (six planes of `t` bytes).
        let bad = || Error::corrupt("peaks");
        let col = |c: usize, j: usize| raw.get(c * t + j).copied().ok_or_else(bad);
        for j in start..start + n {
            let d = u32::from_le_bytes([col(0, j)?, col(1, j)?, col(2, j)?, col(3, j)?]);
            acc = acc.checked_add(d).ok_or_else(bad)?;
            f(acc, u16::from_le_bytes([col(4, j)?, col(5, j)?]));
        }
        Ok(())
    }

    /// Quantised peaks of spectrum `i`: m/z in 1e-5 Da units and intensities as fractions of 65535.
    pub fn peaks_q(&self, i: u32) -> Result<(Vec<u32>, Vec<u16>)> {
        let n = self.rec(i)?.n_peaks as usize;
        let (mut mz, mut it) = (Vec::with_capacity(n), Vec::with_capacity(n));
        self.each_peak_q(i, |m, v| {
            mz.push(m);
            it.push(v);
        })?;
        Ok((mz, it))
    }

    /// Peaks of spectrum `i` (m/z, intensity as a fraction of 1).
    pub fn peaks(&self, i: u32) -> Result<(Vec<f64>, Vec<f64>)> {
        let (mut mz, mut it) = (Vec::new(), Vec::new());
        self.peaks_into(i, &mut mz, &mut it)?;
        Ok((mz, it))
    }

    /// `peaks` into buffers the caller reuses (cleared first): no allocation per spectrum when scanning a window.
    pub fn peaks_into(&self, i: u32, mz: &mut Vec<f64>, it: &mut Vec<f64>) -> Result<()> {
        mz.clear();
        it.clear();
        self.each_peak_q(i, |m, v| {
            mz.push(dequant_mz(m));
            it.push(v as f64 / INT_MAX);
        })
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
                let l = to_usize(crate::util::get_varint(&raw, &mut pos)?)?;
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
        let ci = to_u32(c - 1)?;
        let chunk = self.chunk(Kind::Table, ci)?;
        let n_in = chunk.len() / self.rec_len;
        let (mut lo, mut hi) = (0usize, n_in);
        while lo < hi {
            let m = (lo + hi) / 2;
            let r = SpecRec::from_bytes(tail(&chunk, m * self.rec_len, "chunk")?);
            if less(pol_key(r.pol), r.prec) {
                lo = m + 1;
            } else {
                hi = m;
            }
        }
        Ok(ci * RECORDS_PER_TABLE_CHUNK + to_u32(lo)?)
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
}

#[allow(dead_code)]
fn _unused(d: &IndexDir) -> u64 {
    d.off
}
