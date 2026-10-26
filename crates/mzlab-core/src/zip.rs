//! ZIP writer with method "stored" (no compression, no dependencies, no ZIP64): the one-file export of the user's data
//! (docs/agenti/esporta-dati.md). The page packs the files, the engine returns the archive; any unzip tool opens it.

/// Largest archive and entry count the plain (non-ZIP64) format can describe.
const MAX_ENTRIES: usize = u16::MAX as usize;
const MAX_BYTES: u64 = u32::MAX as u64;
/// 1980-01-01 00:00: the archive is deterministic (the date of the export is in the manifest).
const DOS_DATE: u16 = 0x0021;
/// General-purpose flag bit 11: names are UTF-8.
const FLAG_UTF8: u16 = 1 << 11;

const fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}
static CRC_TABLE: [u32; 256] = crc_table();

/// CRC-32 (IEEE, the one of ZIP and PNG).
pub fn crc32(data: &[u8]) -> u32 {
    !data.iter().fold(!0u32, |c, &b| {
        CRC_TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8)
    })
}

fn u16le(o: &mut Vec<u8>, v: u16) {
    o.extend_from_slice(&v.to_le_bytes());
}
fn u32le(o: &mut Vec<u8>, v: u32) {
    o.extend_from_slice(&v.to_le_bytes());
}

/// The archive holding `files` (name, bytes) in order, every entry stored. Errors (a message, not a key: the caller is the
/// export, which only builds names and sizes it controls): more than 65535 entries, a name over 65535 bytes, a total over 4 GiB.
pub fn zip_stored(files: &[(&str, &[u8])]) -> Result<Vec<u8>, String> {
    if files.len() > MAX_ENTRIES {
        return Err("zip: too many entries".into());
    }
    let total: u64 = files
        .iter()
        .map(|(n, d)| (n.len() + d.len()) as u64 * 2 + 100)
        .sum();
    if total > MAX_BYTES {
        return Err("zip: archive over 4 GiB".into());
    }
    let mut out = Vec::with_capacity(total as usize / 2);
    let mut central = Vec::new();
    for (name, data) in files {
        let nl = u16::try_from(name.len()).map_err(|_| "zip: name too long".to_string())?;
        let crc = crc32(data);
        let offset = out.len() as u32;
        let size = data.len() as u32;
        // local header
        u32le(&mut out, 0x0403_4B50);
        u16le(&mut out, 20);
        u16le(&mut out, FLAG_UTF8);
        u16le(&mut out, 0);
        u16le(&mut out, 0);
        u16le(&mut out, DOS_DATE);
        u32le(&mut out, crc);
        u32le(&mut out, size);
        u32le(&mut out, size);
        u16le(&mut out, nl);
        u16le(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        // central directory entry
        u32le(&mut central, 0x0201_4B50);
        u16le(&mut central, 20);
        u16le(&mut central, 20);
        u16le(&mut central, FLAG_UTF8);
        u16le(&mut central, 0);
        u16le(&mut central, 0);
        u16le(&mut central, DOS_DATE);
        u32le(&mut central, crc);
        u32le(&mut central, size);
        u32le(&mut central, size);
        u16le(&mut central, nl);
        u16le(&mut central, 0);
        u16le(&mut central, 0);
        u16le(&mut central, 0);
        u16le(&mut central, 0);
        u32le(&mut central, 0);
        u32le(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    u32le(&mut out, 0x0605_4B50);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, files.len() as u16);
    u16le(&mut out, files.len() as u16);
    u32le(&mut out, central.len() as u32);
    u32le(&mut out, cd_offset);
    u16le(&mut out, 0);
    Ok(out)
}

/// The files of a packed buffer (what the page hands over): per entry `u16 name length`, `u32 data length` (little endian), the name (UTF-8), the data.
pub fn unpack(packed: &[u8]) -> Result<Vec<(&str, &[u8])>, String> {
    let mut v = Vec::new();
    let mut p = 0usize;
    while p < packed.len() {
        let head = packed.get(p..p + 6).ok_or("zip: truncated entry")?;
        let nl = u16::from_le_bytes([head[0], head[1]]) as usize;
        let dl = u32::from_le_bytes([head[2], head[3], head[4], head[5]]) as usize;
        p += 6;
        let name = packed.get(p..p + nl).ok_or("zip: truncated name")?;
        let data = packed
            .get(p + nl..p + nl + dl)
            .ok_or("zip: truncated data")?;
        v.push((
            std::str::from_utf8(name).map_err(|_| "zip: name is not UTF-8")?,
            data,
        ));
        p += nl + dl;
    }
    Ok(v)
}

/// `unpack` + `zip_stored`: the whole archive from a packed buffer.
pub fn zip_packed(packed: &[u8]) -> Result<Vec<u8>, String> {
    zip_stored(&unpack(packed)?)
}

/// Packs entries into the buffer format of `unpack` (the inverse; the page reads the result of `unzip_packed`).
pub fn pack(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (n, d) in files {
        out.extend_from_slice(&(n.len() as u16).to_le_bytes());
        out.extend_from_slice(&(d.len() as u32).to_le_bytes());
        out.extend_from_slice(n.as_bytes());
        out.extend_from_slice(d);
    }
    out
}

fn le16(b: &[u8], p: usize) -> Result<usize, String> {
    b.get(p..p + 2)
        .map(|x| u16::from_le_bytes([x[0], x[1]]) as usize)
        .ok_or_else(|| "zip: truncated".to_string())
}
fn le32(b: &[u8], p: usize) -> Result<usize, String> {
    b.get(p..p + 4)
        .map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize)
        .ok_or_else(|| "zip: truncated".to_string())
}

/// A name that stays inside the archive: relative, no `..`, no backslash, no NUL, not empty.
fn safe_name(n: &str) -> bool {
    !n.is_empty()
        && !n.starts_with('/')
        && !n.contains('\\')
        && !n.contains('\0')
        && n.split('/').all(|c| c != "..")
}

/// The entries of an archive made of "stored" entries (what `zip_stored` writes, and any other tool's uncompressed ZIP without ZIP64): name and bytes, the CRC-32 of
/// every entry checked. Errors (a message): not a ZIP, compressed or encrypted entry, ZIP64, bad CRC, a name that leaves the archive.
pub fn unzip_stored(z: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let lo = z.len().saturating_sub(22 + u16::MAX as usize);
    let end = (lo..=z.len().saturating_sub(22))
        .rev()
        .find(|&i| z.get(i..i + 4) == Some(&[0x50, 0x4B, 5, 6][..]))
        .ok_or("zip: no end record")?;
    let n = le16(z, end + 10)?;
    let mut p = le32(z, end + 16)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if le32(z, p)? != 0x0201_4B50 {
            return Err("zip: bad central directory".into());
        }
        let (flags, method, crc, size, nl, el, cl, off) = (
            le16(z, p + 8)?,
            le16(z, p + 10)?,
            le32(z, p + 16)?,
            le32(z, p + 24)?,
            le16(z, p + 28)?,
            le16(z, p + 30)?,
            le16(z, p + 32)?,
            le32(z, p + 42)?,
        );
        if method != 0 || flags & 1 != 0 {
            return Err("zip: entry is compressed or encrypted".into());
        }
        if size == 0xFFFF_FFFF || off == 0xFFFF_FFFF {
            return Err("zip: ZIP64 is not supported".into());
        }
        let name = std::str::from_utf8(z.get(p + 46..p + 46 + nl).ok_or("zip: truncated")?)
            .map_err(|_| "zip: name is not UTF-8")?;
        if !safe_name(name) {
            return Err("zip: unsafe name".into());
        }
        if le32(z, off)? != 0x0403_4B50 {
            return Err("zip: bad local header".into());
        }
        let start = off + 30 + le16(z, off + 26)? + le16(z, off + 28)?;
        let data = z.get(start..start + size).ok_or("zip: truncated data")?;
        if crc32(data) as usize != crc {
            return Err("zip: CRC mismatch".into());
        }
        out.push((name.to_string(), data.to_vec()));
        p += 46 + nl + el + cl;
    }
    Ok(out)
}

/// `unzip_stored` into the packed buffer of `unpack`.
pub fn unzip_packed(z: &[u8]) -> Result<Vec<u8>, String> {
    let files = unzip_stored(z)?;
    Ok(pack(
        &files
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect::<Vec<_>>(),
    ))
}

#[cfg(test)]
mod tests;
