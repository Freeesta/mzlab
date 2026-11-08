//! Where the scans of an mzXML lie (ISB mzXML 2.x and 3.x): one pass over the bytes that finds every `<scan ` (the `<index>` at the end of the file is not trusted: the heads of all scans are read anyway).
//! A scan owns the bytes from its `<scan ` to the next `<scan ` (nested scans of mzXML 2: the MS2 scans inside an MS1 start later, so the MS1 ends where the first one begins).
//! A damaged copy (cut short, or a block of zeros) loses scans: a scan holding a NUL byte (never in XML) or cut before its peaks close is left out and counted, and the `<offset>`s of an
//! `<index>` after the run say how many the file had.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;

/// The bytes of one scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Row {
    pub(super) start: u64,
    pub(super) end: u64,
}

/// The block of the pass over the file.
const BLOCK: usize = 1 << 20;

/// What the pass over the file finds.
pub(super) struct Passata {
    /// The intact scans.
    pub(super) rows: Vec<Row>,
    /// The scans left out: they hold a NUL byte, or the file ends before their peaks close.
    pub(super) discarded: usize,
    /// The `<offset>`s of the `<index>` after the run (0: no index).
    pub(super) indicizzate: usize,
}

/// The rows of the scans of the file, from one pass over its bytes.
pub(super) fn rows(data: &mut dyn ReadAt, dim: u64, limits: &Limiti) -> Result<Passata> {
    passata(data, dim, limits)
}

fn read(data: &mut dyn ReadAt, da: u64, n: u64) -> Result<Vec<u8>> {
    let mut b = vec![0u8; usize::try_from(n).map_err(|_| Error::memory_budget())?];
    data.read_at(da, &mut b)?;
    Ok(b)
}

/// The tag that starts at `b[0]` (`b[0] == b'<'`): 1 a `<scan `, 2 a `</scan>`, 3 a `</msRun>`, 4 a `</peaks>`, 0 anything else.
fn tag(b: &[u8]) -> u8 {
    let blanco = |c: Option<&u8>| matches!(c, Some(b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/'));
    if b.starts_with(b"<scan") && blanco(b.get(5)) {
        1
    } else if b.starts_with(b"</scan") && blanco(b.get(6)) {
        2
    } else if b.starts_with(b"</msRun") {
        3
    } else if b.starts_with(b"</peaks") {
        4
    } else {
        0
    }
}

/// The `<offset` tags in the bytes after the end of the run (at most `limiti.byte` of them read).
fn offset_after(data: &mut dyn ReadAt, da: u64, dim: u64, limits: &Limiti) -> Result<usize> {
    let b = read(data, da, (dim - da).min(limits.byte))?;
    Ok(b.windows(7).filter(|w| w == b"<offset").count())
}

/// One pass over the file in blocks of 1 MB: the starts of the scans, the nesting depth against the limit, the end of the run, and what is damaged.
fn passata(data: &mut dyn ReadAt, dim: u64, limits: &Limiti) -> Result<Passata> {
    let (mut starts, mut fine_run): (Vec<u64>, Option<u64>) = (Vec::new(), None);
    let (mut pos, mut profondita, mut closed) = (0u64, 0usize, 0u64);
    let (mut remainder, mut nul): (Vec<u8>, Vec<u64>) = (Vec::new(), Vec::new());
    'file: while pos < dim || !remainder.is_empty() {
        let n = (dim - pos).min(BLOCK as u64);
        let mut buf = std::mem::take(&mut remainder);
        let base = pos - buf.len() as u64;
        buf.extend(read(data, pos, n)?);
        pos += n;
        let last_one = pos >= dim;
        let mut i = 0;
        while let Some(p) = buf[i..].iter().position(|&c| c == b'<' || c == 0) {
            let at = i + p;
            if buf[at] == 0 {
                // one NUL per scan is enough to leave it out
                if nul
                    .last()
                    .is_none_or(|&u| starts.last().is_some_and(|&s| u < s))
                {
                    nul.push(base + at as u64);
                }
                i = at + 1;
                continue;
            }
            if !last_one && buf.len() - at < 8 {
                remainder = buf[at..].to_vec();
                continue 'file;
            }
            match tag(&buf[at..]) {
                1 => {
                    profondita += 1;
                    limits.profondita_ok(profondita)?;
                    limits.voci_ok(starts.len() as u64 + 1)?;
                    starts.push(base + at as u64);
                }
                2 => {
                    profondita = profondita.saturating_sub(1);
                    closed = base + at as u64;
                }
                3 => {
                    fine_run = Some(base + at as u64);
                    break 'file;
                }
                4 => closed = base + at as u64,
                _ => {}
            }
            i = at + 1;
        }
        if last_one {
            break;
        }
    }
    let fine = fine_run.unwrap_or(dim);
    let all_of_them = starts.iter().enumerate().map(|(k, &start)| Row {
        start,
        end: starts.get(k + 1).copied().unwrap_or(fine).max(start),
    });
    let intatta = |r: &Row| {
        let k = nul.partition_point(|&p| p < r.start);
        nul.get(k).is_none_or(|&p| p >= r.end)
            && (fine_run.is_some() || r.end < dim || closed > r.start)
    };
    let righe: Vec<Row> = all_of_them.filter(intatta).collect();
    let indicizzate = match fine_run {
        Some(f) => offset_after(data, f, dim, limits)?,
        None => 0,
    };
    Ok(Passata {
        discarded: starts.len() - righe.len(),
        rows: righe,
        indicizzate,
    })
}
