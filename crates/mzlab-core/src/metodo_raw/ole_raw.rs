//! The method embedded in a `.raw` is an OLE2 compound document (streams `LCQ Header`, `AuditData` and, per device, `Text`, `Data`, `Header`, `Comments`) that
//! starts a few KB after the header of the file. It is read with the OLE reader of the `.wiff` (`ole.rs`) from the offset of its signature, so each text comes
//! with the name of its stream; `opentfraw` finds the same texts by scanning for UTF-16 and that stays as the fallback for a file without the container.
//! `Data` streams are XML (UTF-16 for the Exactive, UTF-8 for the SII/Chromeleon driver).

use crate::ole::Ole;
use opentfraw::RawFileReader;
use std::io::{Read, Seek, SeekFrom};

const SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
/// The container starts within the first bytes of the file (5-7 KB in the files read); the search for its signature does not go further.
const SEARCH: usize = 64 * 1024;
/// The container is never read beyond this many bytes (the scan data come after it).
const MAX: u64 = 8 * 1024 * 1024;
const NAMES: [&str; 3] = ["Text", "Data", "Comments"];

/// A text stream of the container: its path (`Device/Text`) and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub path: String,
    pub text: String,
}

/// UTF-16 (little endian, with or without byte order mark) or UTF-8 text; `None` for anything else.
pub fn decode(b: &[u8]) -> Option<String> {
    let nul = b.iter().take(256).filter(|x| **x == 0).count();
    let t = if b.starts_with(&[0xFF, 0xFE]) || nul > 16 {
        let body = b.strip_prefix(&[0xFF, 0xFE][..]).unwrap_or(b);
        let u: Vec<u16> = body
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&u).ok()?
    } else {
        String::from_utf8(
            b.strip_prefix(&[0xEF, 0xBB, 0xBF][..])
                .unwrap_or(b)
                .to_vec(),
        )
        .ok()?
    };
    let t = t.trim_matches(|c: char| c == '\u{feff}' || c == '\0' || c.is_whitespace());
    (t.chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .count()
        >= 32)
        .then(|| t.to_string())
}

/// The text streams of the container of the method, in the order of the container (`None`: the file has no method or no container).
pub fn streams<R: Read + Seek>(raw: &RawFileReader, source: &mut R) -> Option<Vec<Stream>> {
    if !raw.raw_file_info.preamble.method_file_present {
        return None;
    }
    let len = MAX.min(raw.data_addr) as usize;
    if len < SIGNATURE.len() || source.seek(SeekFrom::Start(0)).is_err() {
        return None;
    }
    let mut buf = vec![0u8; len];
    source.read_exact(&mut buf).ok()?;
    let at = buf[..len.min(SEARCH)]
        .windows(SIGNATURE.len())
        .position(|w| w == SIGNATURE)?;
    let ole = Ole::open(buf.split_off(at), "raw").ok()?;
    let out: Vec<Stream> = ole
        .paths_list()
        .filter(|p| NAMES.contains(&p.rsplit('/').next().unwrap_or("")))
        .filter_map(|p| {
            Some(Stream {
                path: p.to_string(),
                text: decode(&ole.read(p)?)?,
            })
        })
        .collect();
    (!out.is_empty()).then_some(out)
}

/// The blocks of text of the method: those of the container, then those the scan finds that the container does not hold (the same text twice is no loss, a text
/// only the scan finds is no loss either).
pub fn merge(container: &[Stream], scanned: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = container.iter().map(|s| s.text.clone()).collect();
    for b in scanned {
        let probe: String = b.trim().chars().take(200).collect();
        if !out.iter().any(|t| t.contains(&probe)) {
            out.push(b);
        }
    }
    out
}

#[cfg(test)]
mod tests;
