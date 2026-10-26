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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_known_values() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn empty_archive_is_the_end_record_only() {
        let z = zip_stored(&[]).unwrap();
        assert_eq!(z.len(), 22);
        assert_eq!(&z[..4], &[0x50, 0x4B, 5, 6]);
    }

    #[test]
    fn packed_round_trip_layout() {
        let z = zip_stored(&[("a.txt", b"hello"), ("d/vuoto", b"")]).unwrap();
        // 2 local headers (30 + name + data), 2 central entries (46 + name), end record 22
        assert_eq!(z.len(), (30 + 5 + 5) + (30 + 7) + (46 + 5) + (46 + 7) + 22);
        let mut packed = Vec::new();
        for (n, d) in [("a.txt", &b"hello"[..]), ("d/vuoto", &b""[..])] {
            packed.extend_from_slice(&(n.len() as u16).to_le_bytes());
            packed.extend_from_slice(&(d.len() as u32).to_le_bytes());
            packed.extend_from_slice(n.as_bytes());
            packed.extend_from_slice(d);
        }
        assert_eq!(zip_packed(&packed).unwrap(), z);
        assert!(unpack(&packed[..8]).is_err());
    }

    /// With `MZLAB_ZIP_OUT=<dir>`: writes the archives that `tests/test_zip_stored.py` reads back with Python's `zipfile`.
    #[test]
    fn write_for_the_independent_reader() {
        let Ok(dir) = std::env::var("MZLAB_ZIP_OUT") else {
            return;
        };
        let big: Vec<u8> = (0..70_000u32)
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
            .collect();
        let files: Vec<(&str, &[u8])> = vec![
            ("manifesto.json", b"{\"v\":1}"),
            ("vuoto.txt", b""),
            ("sotto/\u{e8}.bin", &big),
            ("z", &[0, 255, 0]),
        ];
        std::fs::write(format!("{dir}/n.zip"), zip_stored(&files).unwrap()).unwrap();
        std::fs::write(format!("{dir}/zero.zip"), zip_stored(&[]).unwrap()).unwrap();
        std::fs::write(
            format!("{dir}/uno_vuoto.zip"),
            zip_stored(&[("v", b"")]).unwrap(),
        )
        .unwrap();
        std::fs::write(format!("{dir}/big.bin"), &big).unwrap();
    }
}
