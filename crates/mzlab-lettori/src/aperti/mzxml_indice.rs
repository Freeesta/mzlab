//! Where the scans of an mzXML lie (ISB mzXML 2.x and 3.x): the `<index name="scan">` at the end of the file when it is consistent, else one pass over the bytes that finds every `<scan `.
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

/// The block of the pass over the file and the tail where `<indexOffset>` is looked for.
const BLOCCO: usize = 1 << 20;
const CODA: u64 = 4096;

/// The rows of the scans of the file: from the index when it holds up, else from a pass over the bytes.
pub(super) fn righe(dati: &mut dyn ReadAt, dim: u64, limiti: &Limiti) -> Result<Vec<Riga>> {
    if let Some(r) = da_indice(dati, dim, limiti)? {
        return Ok(r);
    }
    passata(dati, dim, limiti)
}

fn leggi(dati: &mut dyn ReadAt, da: u64, n: u64) -> Result<Vec<u8>> {
    let mut b = vec![0u8; usize::try_from(n).map_err(|_| Error::memory_budget())?];
    dati.read_at(da, &mut b)?;
    Ok(b)
}

/// The number between `<tag>` and `</tag>` in `s`.
fn numero_di(s: &str, tag: &str) -> Option<u64> {
    let a = s.find(&format!("<{tag}>"))? + tag.len() + 2;
    let b = s.get(a..)?.find('<')? + a;
    s.get(a..b)?.trim().parse().ok()
}

/// The offsets of the index named by `<indexOffset>`; `None` when the file has none or it does not hold up (the pass finds the scans then).
fn da_indice(dati: &mut dyn ReadAt, dim: u64, limiti: &Limiti) -> Result<Option<Vec<Riga>>> {
    let n = dim.min(CODA);
    let coda = String::from_utf8_lossy(&leggi(dati, dim - n, n)?).into_owned();
    let Some(inizio) = numero_di(&coda, "indexOffset").filter(|&o| o > 0 && o < dim) else {
        return Ok(None);
    };
    if dim - inizio > limiti.byte {
        return Ok(None);
    }
    let testo = String::from_utf8_lossy(&leggi(dati, inizio, dim - inizio)?).into_owned();
    let Some(a) = testo.find("<index name=\"scan\"") else {
        return Ok(None);
    };
    let corpo = &testo[a..testo[a..].find("</index>").map_or(testo.len(), |f| a + f)];
    let mut offset: Vec<u64> = Vec::new();
    for pezzo in corpo.split("<offset").skip(1) {
        let Some(v) = pezzo
            .split_once('>')
            .and_then(|(_, r)| r.split('<').next())
            .and_then(|t| t.trim().parse::<u64>().ok())
        else {
            return Ok(None);
        };
        limiti.voci_ok(offset.len() as u64 + 1)?;
        offset.push(v);
    }
    let ordinati = offset.windows(2).all(|w| w[0] < w[1]);
    if offset.is_empty() || !ordinati || offset.last().is_some_and(|&u| u >= inizio) {
        return Ok(None);
    }
    for u in [offset[0], offset[offset.len() - 1]] {
        if !leggi(dati, u, 5.min(dim - u))?.starts_with(b"<scan") {
            return Ok(None);
        }
    }
    let fine = |i: usize| offset.get(i + 1).copied().unwrap_or(inizio);
    Ok(Some(
        (0..offset.len())
            .map(|i| Riga {
                start: offset[i],
                end: fine(i),
            })
            .collect(),
    ))
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
