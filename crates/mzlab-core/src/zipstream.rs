//! The bytes of a "stored" ZIP written as a stream: a local header before each file, a data descriptor after it (the CRC is only known at the end),
//! and the central directory last. ZIP64 is used per entry (size or offset over 4 GiB) and for the end record (65535 entries or more), so a campaign of
//! any size is one valid archive; the sizes are known up front (a `stat`), so nothing is held in memory. Used by `pack` (docs/agenti/file-pack.md).

/// A file of the stream, as the central directory lists it.
#[derive(Clone, Debug)]
pub struct Member {
    pub name: String,
    pub size: u64,
    pub crc: u32,
    /// Where the local header starts in the stream.
    pub at: u64,
}

/// Sizes and offsets from here on need the ZIP64 fields (the 32-bit field holds 0xFFFFFFFF).
pub const ZIP64_AT: u64 = 0xFFFF_FFFF;
const FLAGS: u16 = (1 << 3) | (1 << 11);
const DOS_DATE: u16 = 0x0021;

fn u16le(o: &mut Vec<u8>, v: u16) {
    o.extend_from_slice(&v.to_le_bytes());
}
fn u32le(o: &mut Vec<u8>, v: u32) {
    o.extend_from_slice(&v.to_le_bytes());
}
fn u64le(o: &mut Vec<u8>, v: u64) {
    o.extend_from_slice(&v.to_le_bytes());
}

/// Bytes of a name as the archive writes it, or why it cannot be written.
pub fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > usize::from(u16::MAX) {
        return Err(format!("zip: bad name length {}", name.len()));
    }
    Ok(())
}

fn short(v: u64, zip64_at: u64) -> u32 {
    if v >= zip64_at {
        u32::MAX
    } else {
        u32::try_from(v).unwrap_or(u32::MAX)
    }
}

/// The local header of a file of `size` bytes (the CRC and the sizes follow in the descriptor). Its length is `30 + name + 20` for a ZIP64 entry, else `30 + name`.
pub fn local_header(name: &str, size: u64, zip64_at: u64) -> Vec<u8> {
    let big = size >= zip64_at;
    let mut o = Vec::with_capacity(64 + name.len());
    u32le(&mut o, 0x0403_4B50);
    u16le(&mut o, if big { 45 } else { 20 });
    u16le(&mut o, FLAGS);
    u16le(&mut o, 0);
    u16le(&mut o, 0);
    u16le(&mut o, DOS_DATE);
    u32le(&mut o, 0);
    u32le(&mut o, if big { u32::MAX } else { 0 });
    u32le(&mut o, if big { u32::MAX } else { 0 });
    u16le(&mut o, u16::try_from(name.len()).unwrap_or(u16::MAX));
    u16le(&mut o, if big { 20 } else { 0 });
    o.extend_from_slice(name.as_bytes());
    if big {
        u16le(&mut o, 1);
        u16le(&mut o, 16);
        u64le(&mut o, 0);
        u64le(&mut o, 0);
    }
    o
}

/// The data descriptor that follows the bytes of a file (8-byte sizes for a ZIP64 entry).
pub fn descriptor(crc: u32, size: u64, zip64_at: u64) -> Vec<u8> {
    let mut o = Vec::with_capacity(24);
    u32le(&mut o, 0x0807_4B50);
    u32le(&mut o, crc);
    if size >= zip64_at {
        u64le(&mut o, size);
        u64le(&mut o, size);
    } else {
        u32le(&mut o, short(size, zip64_at));
        u32le(&mut o, short(size, zip64_at));
    }
    o
}

/// Bytes the local header and the descriptor of a file of `size` bytes take around its data.
pub fn framing(name: &str, size: u64, zip64_at: u64) -> (u64, u64) {
    let big = size >= zip64_at;
    let head = 30 + name.len() as u64 + if big { 20 } else { 0 };
    (head, if big { 24 } else { 16 })
}

/// The ZIP64 end record and its locator, written after the central directory of `dir_len` bytes that starts at `dir_at`.
fn zip64_end(o: &mut Vec<u8>, entries: u64, dir_len: u64, dir_at: u64) {
    u32le(o, 0x0606_4B50);
    u64le(o, 44);
    u16le(o, 45);
    u16le(o, 45);
    u32le(o, 0);
    u32le(o, 0);
    u64le(o, entries);
    u64le(o, entries);
    u64le(o, dir_len);
    u64le(o, dir_at);
    u32le(o, 0x0706_4B50);
    u32le(o, 0);
    u64le(o, dir_at + dir_len);
    u32le(o, 1);
}

/// The central directory and the end record(s) for `members`, which the stream wrote from offset 0 to `dir_at`.
pub fn central_directory(members: &[Member], dir_at: u64, zip64_at: u64) -> Vec<u8> {
    let mut o = Vec::new();
    for m in members {
        let (big, far) = (m.size >= zip64_at, m.at >= zip64_at);
        let mut extra = Vec::new();
        if big || far {
            u16le(&mut extra, 1);
            u16le(&mut extra, 8 * (2 * u16::from(big) + u16::from(far)));
            if big {
                u64le(&mut extra, m.size);
                u64le(&mut extra, m.size);
            }
            if far {
                u64le(&mut extra, m.at);
            }
        }
        u32le(&mut o, 0x0201_4B50);
        u16le(&mut o, 45);
        u16le(&mut o, if big || far { 45 } else { 20 });
        u16le(&mut o, FLAGS);
        u16le(&mut o, 0);
        u16le(&mut o, 0);
        u16le(&mut o, DOS_DATE);
        u32le(&mut o, m.crc);
        u32le(&mut o, short(m.size, zip64_at));
        u32le(&mut o, short(m.size, zip64_at));
        u16le(&mut o, u16::try_from(m.name.len()).unwrap_or(u16::MAX));
        u16le(&mut o, u16::try_from(extra.len()).unwrap_or(0));
        u16le(&mut o, 0);
        u16le(&mut o, 0);
        u16le(&mut o, 0);
        u32le(&mut o, 0);
        u32le(&mut o, short(m.at, zip64_at));
        o.extend_from_slice(m.name.as_bytes());
        o.extend_from_slice(&extra);
    }
    let dir_len = o.len() as u64;
    let many = members.len() >= usize::from(u16::MAX);
    let wide = many || dir_at >= zip64_at || dir_len >= zip64_at;
    if wide {
        zip64_end(&mut o, members.len() as u64, dir_len, dir_at);
    }
    let count = u16::try_from(members.len()).unwrap_or(u16::MAX);
    u32le(&mut o, 0x0605_4B50);
    u16le(&mut o, 0);
    u16le(&mut o, 0);
    u16le(&mut o, if wide { u16::MAX } else { count });
    u16le(&mut o, if wide { u16::MAX } else { count });
    u32le(
        &mut o,
        if wide {
            u32::MAX
        } else {
            short(dir_len, zip64_at)
        },
    );
    u32le(
        &mut o,
        if wide {
            u32::MAX
        } else {
            short(dir_at, zip64_at)
        },
    );
    u16le(&mut o, 0);
    o
}
