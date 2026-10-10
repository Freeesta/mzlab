//! Builder of `.mzlib` files with bounded memory: cleaned spectra are spilled to scratch space as they come; at the end they are read back in
//! (polarity, precursor) order and streamed into the peak and metadata blocks, while the Flash index entries are routed to m/z buckets in the
//! scratch space and sorted one bucket at a time. In memory: one small key per spectrum, the open blocks and one bucket.

use crate::clean::{clean, dequant_mz, quant_mz, weighted, CleanOpts, MAX_MZ};
use crate::codec::compress;
use crate::error::{Error, Result};
use crate::format::*;
use crate::io::{ExtentStore, Scratch, Sink};
use crate::spec::{licence, Meta, Spectrum, META_FIELDS};
use crate::util::{crc32c, get_varint, le_f64, le_u16, le_u32, put_varint};

const S_REC: usize = 0;
const S_META: usize = 1;
const S_TABLE: usize = 2;
const BUCKETS: usize = 128;
/// Width of an index bucket in 1e-5 Da units (16 Da); the last bucket takes everything above.
const BUCKET_WIDTH: u32 = 16 * 100_000;
const S_FRAG: usize = 3;
const S_NL: usize = S_FRAG + BUCKETS;
const N_STREAMS: usize = S_NL + BUCKETS;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BuildOpts {
    pub clean: CleanOpts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildInfo {
    pub n_spectra: u32,
    pub n_peaks: u64,
    pub bytes: u64,
    /// Spectra with no peak left after the cleaning.
    pub emptied: u64,
    pub pos: u32,
    pub neg: u32,
}

struct Key {
    prec: f64,
    pol_key: u8,
    orig: u32,
    off: u64,
    len: u32,
}

pub struct Builder<T: Scratch> {
    store: ExtentStore<T>,
    keys: Vec<Key>,
    opts: BuildOpts,
    next_orig: u32,
    emptied: u64,
}

impl<T: Scratch> Builder<T> {
    pub fn new(scratch: T, opts: BuildOpts) -> Self {
        Builder {
            store: ExtentStore::new(scratch, N_STREAMS),
            keys: Vec::new(),
            opts,
            next_orig: 0,
            emptied: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Cleans and stores a spectrum. `Ok(false)`: nothing left after the cleaning (counted, not an error).
    pub fn add(&mut self, s: &Spectrum) -> Result<bool> {
        if self.keys.len() as u64 >= MAX_SPECTRA as u64 - 1 {
            return Err(Error::too_large("spectra"));
        }
        let orig = self.next_orig;
        self.next_orig = self.next_orig.wrapping_add(1);
        let (mz, it) = clean(s.prec, &s.mz, &s.it, &self.opts.clean);
        // quantise; peaks that land on the same m/z unit are merged
        let mut q: Vec<(u32, f64)> = Vec::with_capacity(mz.len());
        for (m, i) in mz.iter().zip(&it) {
            if *m >= MAX_MZ {
                continue;
            }
            let mq = quant_mz(*m);
            match q.last_mut() {
                Some(l) if l.0 == mq => l.1 += i,
                _ => q.push((mq, *i)),
            }
        }
        if q.is_empty() || !(s.prec > 0.0 && s.prec < MAX_MZ) {
            self.emptied += 1;
            return Ok(false);
        }
        let mx = q.iter().fold(0.0f64, |a, x| a.max(x.1));
        let mut rec: Vec<u8> = Vec::with_capacity(32 + q.len() * 6 + 64);
        let flags = if q.len() < 3 { FLAG_FEW_PEAKS } else { 0 };
        rec.extend_from_slice(&s.prec.to_le_bytes());
        rec.push(s.pol as u8);
        rec.push(licence::class_of(&s.meta.license));
        rec.extend_from_slice(&flags.to_le_bytes());
        rec.extend_from_slice(&orig.to_le_bytes());
        rec.extend_from_slice(&(q.len() as u16).to_le_bytes());
        for (mq, i) in &q {
            rec.extend_from_slice(&mq.to_le_bytes());
            let iq = ((i / mx) * INT_MAX).round().clamp(1.0, INT_MAX) as u16;
            rec.extend_from_slice(&iq.to_le_bytes());
        }
        for f in s.meta.fields() {
            put_varint(&mut rec, f.len() as u64);
            rec.extend_from_slice(f.as_bytes());
        }
        let off = self.store.len(S_REC);
        self.store.append(S_REC, &rec)?;
        self.keys.push(Key {
            prec: s.prec,
            pol_key: pol_key(s.pol),
            orig,
            off,
            len: rec.len() as u32,
        });
        Ok(true)
    }

    /// Writes the library. Consumes the builder; the scratch space can be dropped afterwards.
    pub fn finish<K: Sink>(mut self, out: &mut K) -> Result<BuildInfo> {
        if self.keys.is_empty() {
            return Err(Error::new("lib.err.noSpectrum").with("dropped", self.emptied));
        }
        self.store.flush_all()?;
        self.keys.sort_by(|a, b| {
            a.pol_key
                .cmp(&b.pol_key)
                .then(a.prec.total_cmp(&b.prec))
                .then(a.orig.cmp(&b.orig))
        });
        let keys = std::mem::take(&mut self.keys);
        let n = keys.len() as u32;

        // ---- layout of the peak section: directory first (written at the end), chunks after it
        let n_peak_chunks = n.div_ceil(SPECTRA_PER_PEAK_CHUNK);
        let peaks_off = HEAD_RESERVED;
        let peaks_dir_len = chunk_dir_len(n_peak_chunks);
        let mut cursor = align8(peaks_off + peaks_dir_len);
        let mut peak_dir: Vec<ChunkEntry> = Vec::new();
        let mut meta_dir: Vec<ChunkEntry> = Vec::new();
        let mut table_dir: Vec<ChunkEntry> = Vec::new();

        let mut pk_delta: Vec<u32> = Vec::new();
        let mut pk_it: Vec<u16> = Vec::new();
        let mut pk_first = 0u32;
        let mut meta_raw: Vec<u8> = Vec::new();
        let mut meta_first = 0u32;
        let mut meta_bytes = 0u64;
        let mut n_peaks = 0u64;
        let (mut pos_n, mut neg_n) = (0u32, 0u32);

        for (i, k) in keys.iter().enumerate() {
            let i = i as u32;
            let mut buf = crate::util::zeroed(k.len as usize)?;
            self.store.read_at(S_REC, k.off, &mut buf)?;
            // record: prec f64 | pol | licence | flags u16 | orig u32 | n u16 | n x (mz u32, it u16) | 10 x (varint len, bytes)
            let n_pk = le_u16(&buf, 16) as usize;
            let peaks_at = 18;
            let meta_at = peaks_at + n_pk * 6;
            let prec = le_f64(&buf, 0);
            let pol = buf[8] as i8;
            if pol > 0 {
                pos_n += 1;
            } else if pol < 0 {
                neg_n += 1;
            }
            if i.is_multiple_of(RECORDS_PER_TABLE_CHUNK) {
                table_dir.push(ChunkEntry {
                    offset: 0,
                    first_prec: prec,
                    stored: 0,
                    raw: 0,
                    first: i,
                    crc: 0,
                    method: METHOD_RAW,
                    first_pol_key: pol_key(pol),
                });
            }
            if i.is_multiple_of(SPECTRA_PER_PEAK_CHUNK) {
                pk_first = i;
            }
            if i.is_multiple_of(RECORDS_PER_META_CHUNK) {
                meta_first = i;
            }
            let first_peak = u32::try_from(n_peaks).map_err(|_| Error::too_large("peaks"))?;
            // peaks
            let mut prev = 0u32;
            let mut mzq_list: Vec<u32> = Vec::with_capacity(n_pk);
            let mut iq_list: Vec<u16> = Vec::with_capacity(n_pk);
            for j in 0..n_pk {
                let mq = le_u32(&buf, peaks_at + j * 6);
                let iq = le_u16(&buf, peaks_at + j * 6 + 4);
                pk_delta.push(mq - prev);
                prev = mq;
                pk_it.push(iq);
                mzq_list.push(mq);
                iq_list.push(iq);
            }
            n_peaks += n_pk as u64;
            // weighted intensities for the index, from the stored (quantised) values
            let raw_it: Vec<f64> = iq_list.iter().map(|v| *v as f64 / INT_MAX).collect();
            let p = weighted(&raw_it);
            let pmax = p.iter().fold(0.0f64, |a, x| a.max(*x));
            let scale = pmax as f32;
            let rec = SpecRec {
                prec,
                first_peak,
                n_peaks: n_pk as u16,
                pol,
                licence: buf[9],
                flags: le_u16(&buf, 10),
                orig: le_u32(&buf, 12),
                scale,
            };
            self.store.append(S_TABLE, &rec.to_bytes())?;
            for j in 0..n_pk {
                let q = ((p[j] / scale as f64) * INT_MAX)
                    .round()
                    .clamp(1.0, INT_MAX) as u16;
                let mq = mzq_list[j];
                let b = ((mq / BUCKET_WIDTH) as usize).min(BUCKETS - 1);
                self.store.append(S_FRAG + b, &entry_bytes(mq, i, q))?;
                let nl = ((prec - dequant_mz(mq)) * crate::clean::MZ_SCALE).round();
                if nl >= 1.0 && nl < u32::MAX as f64 {
                    let nq = nl as u32;
                    let b = ((nq / BUCKET_WIDTH) as usize).min(BUCKETS - 1);
                    self.store.append(S_NL + b, &entry_bytes(nq, i, q))?;
                }
            }
            // metadata
            meta_raw.extend_from_slice(&buf[meta_at..]);
            // flush blocks
            if (i + 1).is_multiple_of(SPECTRA_PER_PEAK_CHUNK) || i + 1 == n {
                let raw = shuffle_peaks(&pk_delta, &pk_it);
                let (method, stored) = compress(&raw);
                out.write_at(cursor, &stored)?;
                peak_dir.push(ChunkEntry {
                    offset: cursor - peaks_off,
                    first_prec: 0.0,
                    stored: stored.len() as u32,
                    raw: raw.len() as u32,
                    first: pk_first,
                    crc: crc32c(&stored),
                    method,
                    first_pol_key: 0,
                });
                cursor = align8(cursor + stored.len() as u64);
                pk_delta.clear();
                pk_it.clear();
            }
            if (i + 1).is_multiple_of(RECORDS_PER_META_CHUNK) || i + 1 == n {
                let (method, stored) = compress(&meta_raw);
                meta_dir.push(ChunkEntry {
                    offset: meta_bytes,
                    first_prec: 0.0,
                    stored: stored.len() as u32,
                    raw: meta_raw.len() as u32,
                    first: meta_first,
                    crc: crc32c(&stored),
                    method,
                    first_pol_key: 0,
                });
                self.store.append(S_META, &stored)?;
                let next = align8(meta_bytes + stored.len() as u64);
                self.store.append(
                    S_META,
                    &vec![0u8; (next - meta_bytes - stored.len() as u64) as usize],
                )?;
                meta_bytes = next;
                meta_raw.clear();
            }
        }
        drop(keys);
        self.store.flush_all()?;
        let mut sections: Vec<SectionEntry> = Vec::new();

        // ---- peaks directory
        let dir = chunk_dir_bytes(&peak_dir);
        out.write_at(peaks_off, &dir)?;
        sections.push(SectionEntry {
            kind: SEC_PEAKS,
            offset: peaks_off,
            length: cursor - peaks_off,
            dir_len: dir.len() as u32,
            crc: crc32c(&dir),
        });

        // ---- metadata section: directory, then the blocks copied from the scratch stream
        let meta_off = cursor;
        let n_meta_chunks = meta_dir.len() as u32;
        let meta_dir_len = chunk_dir_len(n_meta_chunks);
        let meta_data_off = align8(meta_off + meta_dir_len);
        for d in meta_dir.iter_mut() {
            d.offset += meta_data_off - meta_off;
        }
        copy_stream(&self.store, S_META, out, meta_data_off)?;
        let dir = chunk_dir_bytes(&meta_dir);
        out.write_at(meta_off, &dir)?;
        cursor = meta_data_off + self.store.len(S_META);
        sections.push(SectionEntry {
            kind: SEC_META,
            offset: meta_off,
            length: cursor - meta_off,
            dir_len: dir.len() as u32,
            crc: crc32c(&dir),
        });

        // ---- spectra table
        let table_off = align8(cursor);
        let n_table_chunks = table_dir.len() as u32;
        let table_dir_len = chunk_dir_len(n_table_chunks);
        let table_data_off = align8(table_off + table_dir_len);
        let table_len = self.store.len(S_TABLE);
        copy_stream(&self.store, S_TABLE, out, table_data_off)?;
        let mut tbuf = crate::util::zeroed(SPEC_REC_LEN * RECORDS_PER_TABLE_CHUNK as usize)?;
        for (c, d) in table_dir.iter_mut().enumerate() {
            let first = c as u64 * RECORDS_PER_TABLE_CHUNK as u64;
            let cnt = (n as u64 - first).min(RECORDS_PER_TABLE_CHUNK as u64) as usize;
            let bytes = cnt * SPEC_REC_LEN;
            self.store
                .read_at(S_TABLE, first * SPEC_REC_LEN as u64, &mut tbuf[..bytes])?;
            d.offset = table_data_off - table_off + first * SPEC_REC_LEN as u64;
            d.stored = bytes as u32;
            d.raw = bytes as u32;
            d.crc = crc32c(&tbuf[..bytes]);
        }
        let dir = chunk_dir_bytes(&table_dir);
        out.write_at(table_off, &dir)?;
        cursor = table_data_off + table_len;
        sections.push(SectionEntry {
            kind: SEC_SPECTRA,
            offset: table_off,
            length: cursor - table_off,
            dir_len: dir.len() as u32,
            crc: crc32c(&dir),
        });

        // ---- Flash indexes: fragments, then neutral losses
        for (kind, base) in [(SEC_FRAG, S_FRAG), (SEC_NL, S_NL)] {
            let off = align8(cursor);
            let (len, dir) = write_index(&self.store, base, out, off)?;
            sections.push(SectionEntry {
                kind,
                offset: off,
                length: len,
                dir_len: dir.len() as u32,
                crc: crc32c(&dir),
            });
            cursor = off + len;
        }

        // ---- header and section table
        let hdr = Header {
            version: VERSION,
            min_reader: MIN_READER,
            n_spectra: n,
            n_sections: sections.len() as u32,
            n_peaks,
            flags: 0,
        };
        let mut head = hdr.to_bytes().to_vec();
        for s in &sections {
            head.extend_from_slice(&s.to_bytes());
        }
        let c = crc32c(&head);
        head.extend_from_slice(&c.to_le_bytes());
        if head.len() as u64 > HEAD_RESERVED {
            return Err(Error::too_large("sections"));
        }
        out.write_at(0, &head)?;
        Ok(BuildInfo {
            n_spectra: n,
            n_peaks,
            bytes: cursor,
            emptied: self.emptied,
            pos: pos_n,
            neg: neg_n,
        })
    }
}

fn entry_bytes(mz: u32, spec: u32, q: u16) -> [u8; ENTRY_BYTES] {
    let mut b = [0u8; ENTRY_BYTES];
    b[0..4].copy_from_slice(&mz.to_le_bytes());
    b[4..8].copy_from_slice(&spec.to_le_bytes());
    b[8..10].copy_from_slice(&q.to_le_bytes());
    b
}

fn chunk_dir_bytes(dir: &[ChunkEntry]) -> Vec<u8> {
    let mut b = Vec::with_capacity(8 + dir.len() * CHUNK_ENTRY_LEN);
    b.extend_from_slice(&(dir.len() as u32).to_le_bytes());
    b.extend_from_slice(&[0u8; 4]);
    for d in dir {
        b.extend_from_slice(&d.to_bytes());
    }
    b
}

fn copy_stream<T: Scratch, K: Sink>(
    st: &ExtentStore<T>,
    s: usize,
    out: &mut K,
    at: u64,
) -> Result<()> {
    let total = st.len(s);
    let mut buf = crate::util::zeroed(1 << 20)?;
    let mut done = 0u64;
    while done < total {
        let take = ((total - done) as usize).min(buf.len());
        st.read_at(s, done, &mut buf[..take])?;
        out.write_at(at + done, &buf[..take])?;
        done += take as u64;
    }
    Ok(())
}

/// Reads the 128 bucket streams of one index in order, sorts each and lays the entries into 64 KB pages.
/// Page: `count` entries as three arrays (m/z `u32`, spectrum `u32`, intensity `u16`), zero padded to 65536 bytes.
fn write_index<T: Scratch, K: Sink>(
    st: &ExtentStore<T>,
    base: usize,
    out: &mut K,
    off: u64,
) -> Result<(u64, Vec<u8>)> {
    let total: u64 = (0..BUCKETS)
        .map(|b| st.len(base + b) / ENTRY_BYTES as u64)
        .sum();
    let n_pages = total.div_ceil(ENTRIES_PER_PAGE as u64) as u32;
    let dir_len = page_dir_len(n_pages);
    let data_off = off + dir_len;
    let mut dir: Vec<(u32, u32, u32)> = Vec::with_capacity(n_pages as usize);
    let mut page: Vec<(u32, u32, u16)> = Vec::with_capacity(ENTRIES_PER_PAGE as usize);
    let mut written = 0u64;
    let flush = |page: &mut Vec<(u32, u32, u16)>,
                 dir: &mut Vec<(u32, u32, u32)>,
                 out: &mut K|
     -> Result<()> {
        if page.is_empty() {
            return Ok(());
        }
        let c = page.len();
        let mut b = vec![0u8; c * ENTRY_BYTES];
        for (j, e) in page.iter().enumerate() {
            b[j * 4..j * 4 + 4].copy_from_slice(&e.0.to_le_bytes());
            b[c * 4 + j * 4..c * 4 + j * 4 + 4].copy_from_slice(&e.1.to_le_bytes());
            b[c * 8 + j * 2..c * 8 + j * 2 + 2].copy_from_slice(&e.2.to_le_bytes());
        }
        let at = data_off + dir.len() as u64 * PAGE_BYTES as u64;
        out.write_at(at, &b)?;
        dir.push((page[0].0, c as u32, crc32c(&b)));
        page.clear();
        Ok(())
    };
    for b in 0..BUCKETS {
        let len = st.len(base + b) as usize;
        if len == 0 {
            continue;
        }
        let mut raw = crate::util::zeroed(len)?;
        st.read_at(base + b, 0, &mut raw)?;
        let mut es: Vec<(u32, u32, u16)> = crate::util::vec_with_capacity(len / ENTRY_BYTES)?;
        for c in raw.chunks_exact(ENTRY_BYTES) {
            es.push((le_u32(c, 0), le_u32(c, 4), le_u16(c, 8)));
        }
        drop(raw);
        es.sort_unstable();
        for e in es {
            page.push(e);
            written += 1;
            if page.len() == ENTRIES_PER_PAGE as usize {
                flush(&mut page, &mut dir, out)?;
            }
        }
    }
    flush(&mut page, &mut dir, out)?;
    debug_assert_eq!(written, total);
    let mut d = Vec::with_capacity(dir_len as usize);
    d.extend_from_slice(&total.to_le_bytes());
    d.extend_from_slice(&(dir.len() as u32).to_le_bytes());
    d.extend_from_slice(&ENTRIES_PER_PAGE.to_le_bytes());
    for (first, count, crc) in &dir {
        d.extend_from_slice(&first.to_le_bytes());
        d.extend_from_slice(&count.to_le_bytes());
        d.extend_from_slice(&crc.to_le_bytes());
    }
    d.resize(dir_len as usize, 0);
    out.write_at(off, &d)?;
    // the file ends after the last page's used bytes (the slot is padded only when more pages follow)
    let last_end = dir.len().saturating_sub(1) as u64 * PAGE_BYTES as u64
        + dir.last().map_or(0, |p| p.1 as u64 * ENTRY_BYTES as u64);
    Ok((dir_len + last_end, d))
}

/// Metadata field decoder shared with the reader: `META_FIELDS` strings in a row.
pub(crate) fn read_meta_record(raw: &[u8], pos: &mut usize) -> Result<Meta> {
    let mut f: [String; META_FIELDS] = Default::default();
    for slot in f.iter_mut() {
        let l = get_varint(raw, pos)? as usize;
        let end = pos
            .checked_add(l)
            .filter(|e| *e <= raw.len())
            .ok_or_else(|| Error::corrupt("meta"))?;
        *slot = String::from_utf8_lossy(&raw[*pos..end]).into_owned();
        *pos = end;
    }
    Ok(Meta::from_fields(f))
}

/// Convenience for tests and small libraries: builds in memory.
pub fn build_in_memory(spectra: &[Spectrum], opts: BuildOpts) -> Result<(Vec<u8>, BuildInfo)> {
    let mut b = Builder::new(Vec::<u8>::new(), opts);
    for s in spectra {
        b.add(s)?;
    }
    let mut out = Vec::new();
    let info = b.finish(&mut out)?;
    Ok((out, info))
}
