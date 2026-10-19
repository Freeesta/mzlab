//! What the page needs when a file is uploaded, without Python (`mzidx.builder_inputs`, `source_and_run`, `probe` of `mzlab/reader/mzidx.py`):
//! the arrays of the scan table for the index builder, the `source` and `run` objects of the metadata, and the SHA-256 of the file;
//! `prepare_lettore` the same for a sample of a vendor file set (`mzidx_lettore`).

use sha1::{Digest as _, Sha1};
use sha2::Sha256;

use crate::budget::MemBudget;
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

/// The three fingerprints of one file taken in one pass, fed piece by piece in file order (`aggiorna`): SHA-256 (identity), SHA-1 of every byte (the
/// `sourceFile` of the mzML) and the `probe` (SHA-1 of the size, the first and the last 64 KB).
pub struct Passata {
    size: u64,
    seen: u64,
    sha256: Sha256,
    /// `None` when the caller does not want the SHA-1 of every byte (an mzML upload: only the SHA-256 and the `probe` are used).
    sha1: Option<Sha1>,
    head: Vec<u8>,
    tail: Vec<u8>,
}

impl Passata {
    pub fn new(size: u64) -> Passata {
        Passata {
            size,
            seen: 0,
            sha256: Sha256::new(),
            sha1: Some(Sha1::new()),
            head: Vec::new(),
            tail: Vec::new(),
        }
    }
    /// Only the SHA-256 and the `probe`: `fine` gives an empty SHA-1.
    pub fn senza_sha1(size: u64) -> Passata {
        Passata {
            sha1: None,
            ..Passata::new(size)
        }
    }
    pub fn aggiorna(&mut self, b: &[u8]) {
        self.sha256.update(b);
        if let Some(h) = self.sha1.as_mut() {
            h.update(b);
        }
        let n = (PROBE as usize)
            .saturating_sub(self.head.len())
            .min(b.len());
        self.head.extend_from_slice(&b[..n]);
        let old = self.tail.len();
        let from = (old + b.len()).saturating_sub(PROBE as usize);
        self.tail.drain(..from.min(old));
        self.tail.extend_from_slice(&b[from.saturating_sub(old)..]);
        self.seen += b.len() as u64;
    }
    /// `(sha256, sha1, probe)` as lower-case hex; `None` when the pieces given are not exactly `size` bytes.
    pub fn fine(self) -> Option<(String, String, String)> {
        if self.seen != self.size {
            return None;
        }
        let mut p = Sha1::new();
        p.update(self.size.to_string().as_bytes());
        p.update(&self.head);
        p.update(&self.tail);
        Some((
            hex(&self.sha256.finalize()),
            self.sha1.map(|h| hex(&h.finalize())).unwrap_or_default(),
            hex(&p.finalize()),
        ))
    }
}

/// Reads the whole file once for its three fingerprints (`Passata`): `(sha256, sha1, probe)`.
pub fn fingerprint3(r: &mut dyn ReadAt, size: u64) -> Result<(String, String, String)> {
    fingerprint(r, size, Passata::new(size))
}

fn fingerprint(r: &mut dyn ReadAt, size: u64, mut p: Passata) -> Result<(String, String, String)> {
    let mut at = 0;
    let mut buf = vec![0u8; STEP as usize];
    while at < size {
        let n = (size - at).min(STEP) as usize;
        r.read_at(at, &mut buf[..n])?;
        p.aggiorna(&buf[..n]);
        at += n as u64;
    }
    p.fine().ok_or_else(|| Error::new("err.file.unknown"))
}

/// Everything the index builder and the loading table need of an mzML: the scan table, the metadata objects and the fingerprints.
pub fn prepare<R: ReadAt>(r: &mut R, size: u64, name: &str) -> Result<Prepared> {
    prepare_con(r, size, name, None)
}

/// `prepare` with the `(sha256, probe)` of the file when the page took them while it copied the file (`Passata`): the file is then not read again for them.
pub fn prepare_con<R: ReadAt>(
    r: &mut R,
    size: u64,
    name: &str,
    known: Option<(String, String)>,
) -> Result<Prepared> {
    prepare_in(r, size, name, known, None)
}

/// `prepare_con` that refuses in order (`error.memoryBudget`, phase `table`, `stima`) a file whose scan table does not fit in `budget`.
pub fn prepare_in<R: ReadAt>(
    r: &mut R,
    size: u64,
    name: &str,
    known: Option<(String, String)>,
    budget: Option<&MemBudget>,
) -> Result<Prepared> {
    let t = scantable::read_in(&mut Windowed::new(r, size), size, budget)?;
    let (sha256, probe) = match known {
        Some(k) => k,
        None => {
            let (sha256, _, probe) = fingerprint(r, size, Passata::senza_sha1(size))?;
            (sha256, probe)
        }
    };
    Ok(da_tabella_mzml(t, name, size, sha256, probe))
}

/// Native only: `prepare` with the fingerprints taken by a second thread (`r2`, a second reader of the same file) while the scan table is read.
#[cfg(not(target_arch = "wasm32"))]
pub fn prepare_parallelo<R: ReadAt, R2: ReadAt + Send>(
    r: &mut R,
    mut r2: R2,
    size: u64,
    name: &str,
) -> Result<Prepared> {
    std::thread::scope(|s| {
        let h = s.spawn(move || fingerprint(&mut r2, size, Passata::senza_sha1(size)));
        let t = scantable::read(&mut Windowed::new(r, size), size);
        let prints = h
            .join()
            .unwrap_or_else(|_| Err(Error::new("err.file.unknown")));
        let (sha256, _, probe) = prints?;
        Ok(da_tabella_mzml(t?, name, size, sha256, probe))
    })
}

fn da_tabella_mzml(mut t: Table, name: &str, size: u64, sha256: String, probe: String) -> Prepared {
    let mut source = Out::default();
    source_head(&mut source, name, size, &probe, &sha256);
    source.raw("}");
    let (json, c) = t.drain_json();
    let run = format!(
        "{{\"nce\":{},\"hr1\":{},\"n_chromatograms\":{}}}",
        t.nce, t.hr1, t.n_chromatograms
    );
    Prepared {
        table: ScanTable {
            level: c.level,
            polarity: c.polarity,
            rt: c.rt,
            tic: c.tic,
            scans_json: json.into_bytes(),
            source_json: source.0,
            run_json: run,
            hr1: t.hr1,
        },
        sha256,
        probe,
    }
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
    pub sha1: String,
    pub probe: String,
}

/// The fingerprints of every member of `fs`, in its order: taken before `lettori::apri`, which consumes the set. The SHA-1 goes to the member too,
/// so the reader does not read the file again for the header of the mzML.
pub fn impronte(fs: &mut FileSet) -> Result<Vec<Impronta>> {
    fs.membri
        .iter_mut()
        .map(|m| {
            let (sha256, sha1, probe) = fingerprint3(&mut m.dati, m.dim)?;
            m.sha1 = Some(sha1.clone());
            Ok(Impronta {
                nome: m.nome.clone(),
                dim: m.dim,
                sha256,
                sha1,
                probe,
            })
        })
        .collect()
}

/// The fingerprints of a set already taken (during the copy of the file): `valori` are `(sha256, sha1, probe)` of each member of `fs`, in its order;
/// the SHA-1 goes to the members. `None` when they do not match the members (the caller takes them with `impronte`).
pub fn impronte_note(
    fs: &mut FileSet,
    valori: &[(String, String, String)],
) -> Option<Vec<Impronta>> {
    if valori.len() != fs.membri.len() {
        return None;
    }
    Some(
        fs.membri
            .iter_mut()
            .zip(valori)
            .map(|(m, (sha256, sha1, probe))| {
                m.sha1 = Some(sha1.clone());
                Impronta {
                    nome: m.nome.clone(),
                    dim: m.dim,
                    sha256: sha256.clone(),
                    sha1: sha1.clone(),
                    probe: probe.clone(),
                }
            })
            .collect(),
    )
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
    let t = mzidx_lettore::righe(l, c)?;
    dalla_tabella(t, l, c, membri, name)
}

/// The table, the index and the SHA-256 of sample `c` of a vendor file set with one decoding of the scans for the table and for pass 1 of the builder
/// (the later passes, when the peaks do not fit in `budget`, decode again). `fase(3)` is called when the table is done and the index starts.
/// Returns the statistics of the build and the `sha256` of the set. The index is the one `prepare_lettore` + `mzidx::build_lettore` write.
pub fn indicizza_lettore(
    l: &mut dyn Lettore,
    c: usize,
    membri: &[Impronta],
    name: &str,
    out: &mut dyn crate::mzidx::Output,
    budget: usize,
    fase: &mut dyn FnMut(u32),
) -> Result<(crate::mzidx::Stats, String)> {
    indicizza_lettore_con(
        l,
        c,
        &mut || Ok(membri.to_vec()),
        name,
        out,
        budget,
        None,
        fase,
    )
}

/// `indicizza_lettore` for a caller that gives the fingerprints late and a scratch file:
/// - `membri` is called once, when the table is done: a desktop build hashes the members in another thread while the scans are decoded (`con_impronte_parallele`);
/// - `spill`: where the peaks go when they do not fit in `budget` (`mzidx_spill`), so the scans are decoded ONCE whatever the size of the file; without it the later rounds decode again.
#[allow(clippy::too_many_arguments)]
pub fn indicizza_lettore_con(
    l: &mut dyn Lettore,
    c: usize,
    membri: &mut dyn FnMut() -> Result<Vec<Impronta>>,
    name: &str,
    out: &mut dyn crate::mzidx::Output,
    budget: usize,
    spill: Option<&mut dyn crate::mzidx_spill::Spill>,
    fase: &mut dyn FnMut(u32),
) -> Result<(crate::mzidx::Stats, String)> {
    let (t, prima) = mzidx_lettore::righe_prima(l, c, budget, spill)?;
    let membri = membri()?;
    let p = dalla_tabella(t, l, c, &membri, name)?;
    fase(3);
    let n = l.indice(c)?.len();
    let st = crate::mzidx::build_dopo(
        &p.table,
        prima,
        &mut |on| (0..n).try_for_each(|k| mzidx_lettore::spettro(l, c, k).map(&mut *on)),
        out,
        budget,
        true,
    )?;
    Ok((st, p.sha256))
}

/// The two `.mzidx` of a sample whose scans may be profile, from ONE decoding of them: `out` gets the index with the scans as the file has them (what
/// `indicizza_lettore_con` writes) and `out_cen` the one with the profile scans as centroids (what it writes for a reader wrapped by `centroidato`).
/// The bytes are those of the two separate builds; the budget is shared (half each) and so is the scratch file (`mzidx_spill::Meta`).
#[allow(clippy::too_many_arguments)]
pub fn indicizza_lettore_doppio(
    l: &mut crate::lettori_centroidi::Centroidato,
    c: usize,
    membri: &mut dyn FnMut() -> Result<Vec<Impronta>>,
    name: &str,
    out: &mut dyn crate::mzidx::Output,
    out_cen: &mut dyn crate::mzidx::Output,
    budget: usize,
    spill: Option<&mut dyn crate::mzidx_spill::Spill>,
    fase: &mut dyn FnMut(u32),
) -> Result<((crate::mzidx::Stats, String), (crate::mzidx::Stats, String))> {
    use crate::mzidx_spill::{Meta, Spill};
    let meta = budget / 2;
    let cell = spill.map(std::cell::RefCell::new);
    let mut m0 = cell.as_ref().map(|sp| Meta { sp, quale: 0 });
    let mut m1 = cell.as_ref().map(|sp| Meta { sp, quale: 1 });
    let mut prima = crate::mzidx::Prima::new(meta, m0.as_mut().map(|m| m as &mut dyn Spill));
    let mut prima_cen = crate::mzidx::Prima::new(meta, m1.as_mut().map(|m| m as &mut dyn Spill));
    let (t, t_cen) = mzidx_lettore::righe_prima_doppie(l, c, &mut prima, &mut prima_cen)?;
    let membri = membri()?;
    let p = dalla_tabella(t, l.semplice(), c, &membri, name)?;
    let p_cen = dalla_tabella(t_cen, l.semplice(), c, &membri, name)?;
    fase(3);
    let n = l.indice(c)?.len();
    let st = crate::mzidx::build_dopo(
        &p.table,
        prima,
        &mut |on| (0..n).try_for_each(|k| mzidx_lettore::spettro(l.semplice(), c, k).map(&mut *on)),
        out,
        meta,
        true,
    )?;
    let st_cen = crate::mzidx::build_dopo(
        &p_cen.table,
        prima_cen,
        &mut |on| (0..n).try_for_each(|k| mzidx_lettore::spettro(l, c, k).map(&mut *on)),
        out_cen,
        meta,
        true,
    )?;
    Ok(((st, p.sha256), (st_cen, p_cen.sha256)))
}

/// Native only: the fingerprints of the members are taken by a second thread (`recipe` makes its own file set, since a reader does not cross threads)
/// while `lavoro` decodes the scans. `lavoro` gets the function that waits for them (to hand to `indicizza_lettore_con`); the members of the set
/// it opens carry an empty SHA-1 (`senza_sha1`): the table does not read it. SHA-256 and SHA-1 are the same as `impronte` gives.
#[cfg(not(target_arch = "wasm32"))]
pub fn con_impronte_parallele<R>(
    recipe: impl FnOnce() -> Result<FileSet> + Send,
    lavoro: impl FnOnce(&mut dyn FnMut() -> Result<Vec<Impronta>>) -> Result<R>,
) -> Result<R> {
    std::thread::scope(|s| {
        let mut h = Some(s.spawn(move || recipe().and_then(|mut fs| impronte(&mut fs))));
        let mut attendi = || match h.take() {
            Some(h) => h
                .join()
                .unwrap_or_else(|_| Err(Error::new("err.file.unknown"))),
            None => Err(Error::new("err.file.unknown")),
        };
        lavoro(&mut attendi)
    })
}

/// Marks the members as having a SHA-1 already (empty), so the reader does not read the whole file for the header of an mzML nobody asked for: the
/// index never uses it.
pub fn senza_sha1(fs: &mut FileSet) {
    for m in &mut fs.membri {
        m.sha1 = Some(String::new());
    }
}

fn dalla_tabella(
    mut t: Table,
    l: &mut dyn Lettore,
    c: usize,
    membri: &[Impronta],
    name: &str,
) -> Result<Prepared> {
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
