//! Writers behind `Builder::finish`: the Flash index pages, chunk directories, stream copies and the v2 sections (library info, InChIKey hashes).

use super::*;
use crate::util::get_varint;

impl<T: Scratch> Builder<T> {
    /// Appends `SEC_LIBINFO` and `SEC_INCHI` after `cursor`; returns the new end of the data.
    pub(super) fn write_extras<K: Sink>(
        &mut self,
        out: &mut K,
        sections: &mut Vec<SectionEntry>,
        mut cursor: u64,
    ) -> Result<u64> {
        let mut info = self.detect.finish(self.opts.declared);
        [info.source, info.version, info.sha256] = std::mem::take(&mut self.source);
        let json = info.to_json();
        let off = align8(cursor);
        out.write_at(off, &json)?;
        sections.push(SectionEntry {
            kind: SEC_LIBINFO,
            offset: off,
            length: json.len() as u64,
            dir_len: u32::try_from(json.len()).map_err(|_| Error::too_large("libinfo"))?,
            crc: crc32c(&json),
        });
        cursor = off + json.len() as u64;
        let off = align8(cursor);
        let len = self.store.len(S_IK);
        let crc = copy_stream(&self.store, S_IK, out, off)?;
        sections.push(SectionEntry {
            kind: SEC_INCHI,
            offset: off,
            length: len,
            dir_len: u32::try_from(len).map_err(|_| Error::too_large("inchikey"))?,
            crc,
        });
        cursor = off + len;
        Ok(cursor)
    }
}

pub(super) fn entry_bytes(mz: u32, spec: u32, q: u16) -> [u8; ENTRY_BYTES] {
    let mut b = [0u8; ENTRY_BYTES];
    b[0..4].copy_from_slice(&mz.to_le_bytes());
    b[4..8].copy_from_slice(&spec.to_le_bytes());
    b[8..10].copy_from_slice(&q.to_le_bytes());
    b
}

pub(super) fn chunk_dir_bytes(dir: &[ChunkEntry]) -> Vec<u8> {
    let mut b = Vec::with_capacity(8 + dir.len() * CHUNK_ENTRY_LEN);
    b.extend_from_slice(&(dir.len() as u32).to_le_bytes());
    b.extend_from_slice(&[0u8; 4]);
    for d in dir {
        b.extend_from_slice(&d.to_bytes());
    }
    b
}

/// Copies a scratch stream to `out` at `at`; returns the CRC32C of what was copied.
pub(super) fn copy_stream<T: Scratch, K: Sink>(
    st: &ExtentStore<T>,
    s: usize,
    out: &mut K,
    at: u64,
) -> Result<u32> {
    let total = st.len(s);
    let mut buf = crate::util::zeroed(1 << 20)?;
    let mut crc = 0u32;
    let mut done = 0u64;
    while done < total {
        let take = ((total - done) as usize).min(buf.len());
        st.read_at(s, done, &mut buf[..take])?;
        out.write_at(at + done, &buf[..take])?;
        crc = crate::util::crc32c_update(crc, &buf[..take]);
        done += take as u64;
    }
    Ok(crc)
}

/// Reads the 128 bucket streams of one index in order, sorts each and lays the entries into 64 KB pages.
/// Page: `count` entries as three arrays (m/z `u32`, spectrum `u32`, intensity `u16`), zero padded to 65536 bytes.
pub(super) fn write_index<T: Scratch, K: Sink>(
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
        sortutil::sort_by(&mut es, |a, b| a.cmp(b));
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
