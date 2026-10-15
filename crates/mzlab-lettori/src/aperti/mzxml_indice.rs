//! Where the scans of an mzXML lie (ISB mzXML 2.x and 3.x): one pass over the bytes that finds every `<scan ` (the `<index>` at the end of the file is not trusted: the heads of all scans are read anyway).
//! A scan owns the bytes from its `<scan ` to the next `<scan ` (nested scans of mzXML 2: the MS2 scans inside an MS1 start later, so the MS1 ends where the first one begins).

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;

/// The bytes of one scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Riga {
    pub(super) start: u64,
    pub(super) end: u64,
}

/// The block of the pass over the file.
const BLOCCO: usize = 1 << 20;

/// The rows of the scans of the file, from one pass over its bytes.
pub(super) fn righe(dati: &mut dyn ReadAt, dim: u64, limiti: &Limiti) -> Result<Vec<Riga>> {
    passata(dati, dim, limiti)
}

fn leggi(dati: &mut dyn ReadAt, da: u64, n: u64) -> Result<Vec<u8>> {
    let mut b = vec![0u8; usize::try_from(n).map_err(|_| Error::memory_budget())?];
    dati.read_at(da, &mut b)?;
    Ok(b)
}

/// The tag that starts at `b[0]` (`b[0] == b'<'`): 1 a `<scan `, 2 a `</scan>`, 3 a `</msRun>`, 0 anything else.
fn tag(b: &[u8]) -> u8 {
    let blanco = |c: Option<&u8>| matches!(c, Some(b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/'));
    if b.starts_with(b"<scan") && blanco(b.get(5)) {
        1
    } else if b.starts_with(b"</scan") && blanco(b.get(6)) {
        2
    } else if b.starts_with(b"</msRun") {
        3
    } else {
        0
    }
}

/// One pass over the file in blocks of 1 MB: the starts of the scans, the nesting depth against the limit, and the end of the run.
fn passata(dati: &mut dyn ReadAt, dim: u64, limiti: &Limiti) -> Result<Vec<Riga>> {
    let (mut starts, mut fine_run): (Vec<u64>, u64) = (Vec::new(), dim);
    let (mut pos, mut profondita) = (0u64, 0usize);
    let mut resto: Vec<u8> = Vec::new();
    'file: while pos < dim || !resto.is_empty() {
        let n = (dim - pos).min(BLOCCO as u64);
        let mut buf = std::mem::take(&mut resto);
        let base = pos - buf.len() as u64;
        buf.extend(leggi(dati, pos, n)?);
        pos += n;
        let ultimo = pos >= dim;
        let mut i = 0;
        while let Some(p) = buf[i..].iter().position(|&c| c == b'<') {
            let at = i + p;
            if !ultimo && buf.len() - at < 8 {
                resto = buf[at..].to_vec();
                continue 'file;
            }
            match tag(&buf[at..]) {
                1 => {
                    profondita += 1;
                    limiti.profondita_ok(profondita)?;
                    limiti.voci_ok(starts.len() as u64 + 1)?;
                    starts.push(base + at as u64);
                }
                2 => profondita = profondita.saturating_sub(1),
                3 => {
                    fine_run = base + at as u64;
                    break 'file;
                }
                _ => {}
            }
            i = at + 1;
        }
        if ultimo {
            break;
        }
    }
    Ok(starts
        .iter()
        .enumerate()
        .map(|(k, &start)| Riga {
            start,
            end: starts.get(k + 1).copied().unwrap_or(fine_run).max(start),
        })
        .collect())
}
