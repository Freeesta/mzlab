//! What the page needs when a file is uploaded, without Python (`mzidx.builder_inputs`, `source_and_run`, `probe` of `mzlab/reader/mzidx.py`):
//! the arrays of the scan table for the index builder, the `source` and `run` objects of the metadata, and the SHA-256 of the file;
//! `prepare_lettore` the same for a sample of a vendor file set (`mzidx_lettore`).

use sha1::{Digest as _, Sha1};
use sha2::Sha256;

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::json::Out;
use crate::lettori::{FileSet, Lettore};
use crate::mzidx::ScanTable;
use crate::mzidx_lettore;
use crate::scantable::{self, Table};
use crate::window::Windowed;

const PROBE: u64 = 65536;
const STEP: u64 = 1 << 20;

pub struct Prepared {
    pub table: ScanTable,
    pub sha256: String,
    pub probe: String,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Reads the whole file once for its SHA-256 and keeps the first and last 64 KB for the probe (SHA-1 of the size, the head and the tail).
pub fn fingerprint<R: ReadAt + ?Sized>(r: &mut R, size: u64) -> Result<(String, String)> {
    let mut sha = Sha256::new();
    let mut head = vec![];
    let mut at = 0;
    let mut buf = vec![0u8; STEP as usize];
    while at < size {
        let n = (size - at).min(STEP) as usize;
        r.read_at(at, &mut buf[..n])?;
        sha.update(&buf[..n]);
        if head.len() < PROBE as usize {
            head.extend_from_slice(&buf[..n.min(PROBE as usize - head.len())]);
        }
        at += n as u64;
    }
    let from = size.saturating_sub(PROBE);
    let mut tail = vec![0u8; (size - from) as usize];
    r.read_at(from, &mut tail)?;
    let mut p = Sha1::new();
    p.update(size.to_string().as_bytes());
    p.update(&head);
    p.update(&tail);
    Ok((hex(&sha.finalize()), hex(&p.finalize())))
}

/// Everything the index builder and the loading table need of an mzML: the scan table, the metadata objects and the fingerprints.
pub fn prepare<R: ReadAt>(r: &mut R, size: u64, name: &str) -> Result<Prepared> {
    let t = scantable::read(&mut Windowed::new(r, size), size)?;
    let (sha256, probe) = fingerprint(r, size)?;
    let mut source = Out::default();
    source_head(&mut source, name, size, &probe, &sha256);
    source.raw("}");
    let scans_json = t.rows_json().into_bytes();
    Ok(prepared(&t, scans_json, source.0, sha256, probe))
}

/// The opening of the `source` object, up to `sha256`.
fn source_head(o: &mut Out, name: &str, size: u64, probe: &str, sha256: &str) {
    o.raw("{\"name\":")
        .str(name)
        .raw(",\"size\":")
        .int(size as i64)
        .raw(",\"mtime_ns\":0,\"probe\":")
        .str(probe)
        .raw(",\"sha256\":")
        .str(sha256);
}

fn prepared(
    t: &Table,
    scans_json: Vec<u8>,
    source_json: String,
    sha256: String,
    probe: String,
) -> Prepared {
    let run = format!(
        "{{\"nce\":{},\"hr1\":{},\"n_chromatograms\":{}}}",
        t.nce, t.hr1, t.n_chromatograms
    );
    let table = ScanTable {
        level: t.rows.iter().map(|s| s.level).collect(),
        polarity: t.rows.iter().map(|s| s.polarity).collect(),
        rt: t.rows.iter().map(|s| s.rt).collect(),
        tic: t.rows.iter().map(|s| s.tic).collect(),
        scans_json,
        source_json,
        run_json: run,
        hr1: t.hr1,
    };
    Prepared {
        table,
        sha256,
        probe,
    }
}

/// The fingerprints of one member of a vendor file set (`fingerprint`).
#[derive(Debug, Clone, PartialEq)]
pub struct Impronta {
    pub nome: String,
    pub dim: u64,
    pub sha256: String,
    pub probe: String,
}

/// The fingerprints of every member of `fs`, in its order: taken before `lettori::apri`, which consumes the set.
pub fn impronte(fs: &mut FileSet) -> Result<Vec<Impronta>> {
    fs.membri
        .iter_mut()
        .map(|m| {
            let (sha256, probe) = fingerprint(&mut m.dati, m.dim)?;
            Ok(Impronta {
                nome: m.nome.clone(),
                dim: m.dim,
                sha256,
                probe,
            })
        })
        .collect()
}

/// `sha256` and `probe` of a set: those of its member when it has one; otherwise SHA-256 (SHA-1 for the probe) of the members' values, each
/// followed by a newline, then the id of the sample (a `.wiff` gives one index per sample).
fn impronta_insieme(membri: &[Impronta], campione: u32) -> (String, String) {
    if let [m] = membri {
        return (m.sha256.clone(), m.probe.clone());
    }
    let (mut sha, mut p) = (Sha256::new(), Sha1::new());
    for m in membri {
        sha.update(format!("{}\n", m.sha256).as_bytes());
        p.update(format!("{}\n", m.probe).as_bytes());
    }
    sha.update(campione.to_string().as_bytes());
    p.update(campione.to_string().as_bytes());
    (hex(&sha.finalize()), hex(&p.finalize()))
}

/// The same for sample `c` of a vendor file set (`.mzidx` version 2, `mzidx_lettore`): `membri` are the fingerprints of the set (`impronte`),
/// `name` the name the index goes by. `source` gains `members` and `sample`; `size` is the sum of the members.
pub fn prepare_lettore(
    l: &mut dyn Lettore,
    c: usize,
    membri: &[Impronta],
    name: &str,
) -> Result<Prepared> {
    let mut t = mzidx_lettore::righe(l, c)?;
    let scans_json = mzidx_lettore::rows_json(&mut t, l.indice(c)?).into_bytes();
    let campione = l
        .campioni()
        .get(c)
        .cloned()
        .ok_or_else(|| Error::new("error.badIndex").with("c", c))?;
    let (sha256, probe) = impronta_insieme(membri, campione.id);
    let mut source = Out::default();
    source_head(
        &mut source,
        name,
        membri.iter().map(|m| m.dim).sum(),
        &probe,
        &sha256,
    );
    source.raw(",\"members\":[");
    for (i, m) in membri.iter().enumerate() {
        source
            .raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(&m.nome)
            .raw(",\"size\":")
            .int(m.dim as i64)
            .raw(",\"sha256\":")
            .str(&m.sha256)
            .raw(",\"probe\":")
            .str(&m.probe)
            .raw("}");
    }
    source
        .raw("],\"sample\":{\"id\":")
        .int(i64::from(campione.id))
        .raw(",\"nome\":")
        .str(&campione.nome)
        .raw("}}");
    Ok(prepared(&t, scans_json, source.0, sha256, probe))
}
