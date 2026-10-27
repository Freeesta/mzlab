//! Directory readers of the `.mzlib` sections (chunked sections, Flash index pages, the v2 info and InChIKey sections); used by `Library::open`.

use super::*;

pub(super) fn read_chunked<S: Source>(
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
        let e = ChunkEntry::from_bytes(tail(&d, 8 + i * CHUNK_ENTRY_LEN, "directory")?);
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

pub(super) fn read_index_dir<S: Source>(src: &S, s: &SectionEntry) -> Result<IndexDir> {
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
    let need = pages.last().map_or(0, |p| {
        (n_pages as u64 - 1) * PAGE_BYTES as u64 + p.1 as u64 * ENTRY_BYTES as u64
    });
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

/// The v2 sections: the library info (default in a v1 file) and the location of the InChIKey hashes (checked against the spectrum count).
pub(super) fn read_v2_parts<S: Source>(
    src: &S,
    secs: &[SectionEntry],
    n: u32,
) -> Result<(LibInfo, Option<SectionEntry>)> {
    let info = match secs.iter().find(|s| s.kind == SEC_LIBINFO) {
        Some(e) if e.dir_len <= 1 << 20 => {
            let b = read_vec(src, e.offset, e.dir_len as u64)?;
            if crc32c(&b) != e.crc {
                return Err(Error::corrupt("libinfo"));
            }
            LibInfo::from_json(&b)?
        }
        Some(_) => return Err(Error::corrupt("libinfo")),
        None => LibInfo::default(),
    };
    let inchi_at = secs.iter().find(|s| s.kind == SEC_INCHI).copied();
    if inchi_at.is_some_and(|e| e.dir_len as u64 != n as u64 * 8 || e.length != e.dir_len as u64) {
        return Err(Error::corrupt("inchikey"));
    }
    Ok((info, inchi_at))
}

/// The section table that follows the header, each entry checked against the file length and the reserved head.
pub(super) fn read_section_table(
    head: &[u8],
    n_sections: u32,
    flen: u64,
    reserved: u64,
) -> Result<Vec<SectionEntry>> {
    let mut secs = Vec::new();
    for i in 0..n_sections as usize {
        let at = HEADER_LEN + i * SECTION_ENTRY_LEN;
        let e = SectionEntry::from_bytes(tail(head, at, "sections")?);
        if e.offset.checked_add(e.length).is_none_or(|end| end > flen)
            || e.offset < reserved
            || e.dir_len as u64 > e.length
        {
            return Err(Error::corrupt("section"));
        }
        secs.push(e);
    }
    Ok(secs)
}
