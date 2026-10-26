//! Times every phase the site goes through for a Thermo `.raw` (`crates/mzlab-wasm/src/nativi.rs`): fingerprint, scan table, `.mzidx`,
//! «Scarica .mzML», each with the reader as it is and with the centroids of `lettori_centroidi`. The member is read through a 256 KB block
//! cache like the `thermo` module's `JsMember` (`--blocks N` blocks, 3 by default), and the number of reads asked of it, the blocks fetched and the bytes asked, fetched and
//! fetched again are counted (in the browser each fetch is a JS call).
//! `cargo run --release --example raw_tempi -- FILE.raw [--no-cen] [--no-mzml] [--blocks N] [--threads N] [--sha]`; numbers in `docs/studi/RAW_VELOCE.md`.
use std::cell::Cell;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::rc::Rc;
use std::time::Instant;

use sha2::{Digest, Sha256};

use mzlab_core::blocchi::BlockCache;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{self, FileSet, Lettore, Limiti, Membro};
use mzlab_core::{lettori_centroidi, lettori_mzml, mzidx, upload};

const BLOCK: usize = 256 << 10;

/// Page size of the re-read count.
const PAGE: u64 = 4096;

/// What the reader asked of a `Blocks` and what was read from the file for it, in a phase: `asked` the bytes of the reads, `fetched` the bytes read from the file,
/// `reread` the bytes of the pages (4 KB) fetched that an earlier fetch of the phase had already brought.
#[derive(Default)]
struct Bytes {
    asked: Cell<u64>,
    fetched: Cell<u64>,
    reread: Cell<u64>,
    seen: std::cell::RefCell<Vec<u64>>,
}

impl Bytes {
    fn fetch(&self, off: u64, len: u64) {
        self.fetched.set(self.fetched.get() + len);
        let mut seen = self.seen.borrow_mut();
        for page in off / PAGE..(off + len).div_ceil(PAGE) {
            let (w, bit) = ((page / 64) as usize, 1u64 << (page % 64));
            if seen.len() <= w {
                seen.resize(w + 1, 0);
            }
            if seen[w] & bit != 0 {
                self.reread.set(self.reread.get() + PAGE);
            }
            seen[w] |= bit;
        }
    }
}

/// A file read through the block cache of `mzlab-lettori-wasm` (`mzlab_core::blocchi::BlockCache`, `slots` blocks of 256 KB); `calls` counts the reads asked of it, `fills` the
/// blocks (and big reads) fetched from the file.
struct Blocks<R> {
    inner: R,
    calls: Rc<Cell<u64>>,
    bytes: Rc<Bytes>,
}

impl<R: ReadAt> ReadAt for Blocks<R> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        self.calls.set(self.calls.get() + 1);
        self.bytes
            .asked
            .set(self.bytes.asked.get() + buf.len() as u64);
        self.inner.read_at(off, buf)
    }
}

struct Counters {
    calls: Rc<Cell<u64>>,
    fills: Rc<Cell<u64>>,
    bytes: Rc<Bytes>,
}

impl Counters {
    /// Reads, blocks, and the bytes (asked, fetched, re-read) of the phase that ends; the next phase starts from zero.
    fn take(&self) -> (u64, u64, [u64; 3]) {
        self.bytes.seen.borrow_mut().clear();
        (
            self.calls.replace(0),
            self.fills.replace(0),
            [
                self.bytes.asked.replace(0),
                self.bytes.fetched.replace(0),
                self.bytes.reread.replace(0),
            ],
        )
    }
}

/// Number of blocks of the cache (`--blocks N`, 3 like the `thermo` module; 1 is the single block it had before).
fn slots() -> usize {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == "--blocks")
        .and_then(|i| a.get(i + 1))
        .and_then(|n| n.parse().ok())
        .unwrap_or(3)
}

/// Encoding threads of the mzML writer (`--threads N`; the default of the writer, the cores minus one, without it).
fn threads() -> usize {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == "--threads")
        .and_then(|i| a.get(i + 1))
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(mzlab_core::mzmlwrite::thread_predefinito)
}

fn set(path: &str) -> (FileSet, Counters) {
    let dim = std::fs::metadata(path).unwrap().len();
    let c = Counters {
        calls: Rc::default(),
        fills: Rc::default(),
        bytes: Rc::default(),
    };
    let mut f = File::open(path).unwrap();
    let (fills, bytes) = (c.fills.clone(), c.bytes.clone());
    let fetch = move |off: u64, buf: &mut [u8]| -> mzlab_core::Result<()> {
        fills.set(fills.get() + 1);
        bytes.fetch(off, buf.len() as u64);
        f.seek(SeekFrom::Start(off)).unwrap();
        f.read_exact(buf)
            .map_err(|_| mzlab_core::Error::new("error.badIndex"))
    };
    let b = Blocks {
        inner: BlockCache::new(fetch, dim, BLOCK, slots()),
        calls: c.calls.clone(),
        bytes: c.bytes.clone(),
    };
    let nome = std::path::Path::new(path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    (
        FileSet {
            membri: vec![Membro {
                nome,
                dim,
                dati: Box::new(b),
                sha1: None,
            }],
        },
        c,
    )
}

/// Counts the bytes written and, with `--sha`, keeps their SHA-256 (the mzML must be byte for byte the same from one build to the next; hashing 40 MB on the
/// writing thread is a cost of its own, so the times are taken without it).
#[derive(Default)]
struct Sink(u64, Option<Sha256>);
impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0 += b.len() as u64;
        if let Some(h) = self.1.as_mut() {
            h.update(b);
        }
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn peak_rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(f64::NAN, |kb| kb / 1024.0)
}

fn open(path: &str, cen: bool) -> (Box<dyn Lettore>, Counters, f64) {
    let (fs, c) = set(path);
    let t = Instant::now();
    let l = lettori::apri(fs, &Limiti::sito()).expect("open");
    let l = if cen {
        lettori_centroidi::centroidato(l)
    } else {
        l
    };
    (l, c, t.elapsed().as_secs_f64())
}

/// First 12 hex digits of the SHA-256 of `b` (the `.mzidx` must be byte for byte the same from one build to the next).
fn hash12(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .take(6)
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn line(what: &str, secs: f64, c: &Counters, extra: &str) {
    let (calls, fills, [asked, fetched, reread]) = c.take();
    let mb = |b: u64| b as f64 / 1e6;
    println!(
        "{what:<28} {secs:>8.3} s  reads {calls:>10}  blocks {fills:>7}  MB asked {:>8.1} fetched {:>8.1} re-read {:>8.1}  {extra}",
        mb(asked),
        mb(fetched),
        mb(reread)
    );
}

/// The index with a scratch file (the browser's second OPFS file, a temporary file on the desktop) and the desktop way with the fingerprints in a second thread.
fn scratch(tmp: &std::path::Path) -> File {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(tmp)
        .expect("scratch file")
}

/// The desktop way, from the open of the file: the fingerprints (SHA-256, SHA-1) in a second thread while the scans are decoded once.
fn desktop_phase(path: &str, name: &str, tmp: &std::path::Path, c: &Counters) {
    let t = Instant::now();
    let (mut fs, _) = set(path);
    upload::senza_sha1(&mut fs);
    let l2 = lettori::apri(fs, &Limiti::sito()).expect("open");
    let mut l2 = l2;
    let mut out = Cursor::new(Vec::new());
    let mut sp = scratch(tmp);
    let file = path.to_string();
    let (st, _) = upload::con_impronte_parallele(
        move || {
            let dim = std::fs::metadata(&file)
                .map_err(|_| mzlab_core::Error::new("error.badIndex"))?
                .len();
            let f = File::open(&file).map_err(|_| mzlab_core::Error::new("error.badIndex"))?;
            Ok(FileSet {
                membri: vec![Membro {
                    nome: file.rsplit('/').next().unwrap_or("").to_string(),
                    dim,
                    dati: Box::new(f),
                    sha1: None,
                }],
            })
        },
        |imp| {
            upload::indicizza_lettore_con(
                &mut *l2,
                0,
                imp,
                &format!("{name}.mzML"),
                &mut out,
                192 << 20,
                Some(&mut sp),
                &mut |_| {},
            )
        },
    )
    .expect("index");
    line(
        "open + fingerprint || index",
        t.elapsed().as_secs_f64(),
        c,
        &format!("{:.1} MB, {} passes", st.bytes as f64 / 1e6, st.passes),
    );
}

fn scratch_phases(
    path: &str,
    name: &str,
    cen: bool,
    l: &mut dyn Lettore,
    imp: &[upload::Impronta],
    c: &Counters,
) {
    // the same with a scratch file for the peaks that do not fit in the budget (the browser's second OPFS file, a temporary file on the desktop): the scans are decoded once
    let tmp = std::env::temp_dir().join(format!("raw_tempi_{}.sp", std::process::id()));
    let t = Instant::now();
    let mut out = Cursor::new(Vec::new());
    let mut sp = scratch(&tmp);
    let (st, _) = upload::indicizza_lettore_con(
        &mut *l,
        0,
        &mut || Ok(imp.to_vec()),
        &format!("{name}.mzML"),
        &mut out,
        192 << 20,
        Some(&mut sp),
        &mut |_| {},
    )
    .expect("index");
    line(
        "table + .mzidx + scratch",
        t.elapsed().as_secs_f64(),
        c,
        &format!(
            "{:.1} MB, {} passes, scratch {:.1} MB, sha256 {}",
            st.bytes as f64 / 1e6,
            st.passes,
            sp.metadata().map_or(0.0, |m| m.len() as f64 / 1e6),
            hash12(out.get_ref())
        ),
    );
    drop(sp);

    if !cen {
        desktop_phase(path, name, &tmp, c);
    }
    let _ = std::fs::remove_file(&tmp);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .expect("usage: raw_tempi FILE.raw [--no-cen] [--no-mzml]")
        .clone();
    let with_cen = !args.iter().any(|a| a == "--no-cen");
    let with_mzml = !args.iter().any(|a| a == "--no-mzml");
    let name = std::path::Path::new(&path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    println!(
        "{name}: {:.1} MB",
        std::fs::metadata(&path).unwrap().len() as f64 / 1e6
    );

    for cen in [false, true] {
        if !cen || with_cen {
            phases(&path, &name, cen, with_mzml);
        }
    }
    println!("peak memory {:.0} MB", peak_rss_mb());
}

/// The producer of the mzML writer alone: every scan and its extras, nothing written (the part of the writer that stays on one thread).
fn producer(l: &mut dyn Lettore, n: usize, c: &Counters) {
    let t = Instant::now();
    let mut pts = 0usize;
    for k in 0..n {
        let s = l.scansione(0, k).unwrap();
        let extra = l.extra_scansione(0, k, &s);
        pts += s.mz.len() + extra.params.len();
    }
    line(
        "producer (scan + extras)",
        t.elapsed().as_secs_f64(),
        c,
        &format!("{pts} values"),
    );
}

/// The mzML writer: its producer alone, the header, then the whole write (with `--threads N` encoding threads and, with `--sha`, the SHA-256 of the file).
fn mzml(l: &mut dyn Lettore, n: usize, c: &Counters) {
    producer(&mut *l, n, c);
    let t = Instant::now();
    let _ = l.testata_mzml(0).expect("header");
    line("header (testata_mzml)", t.elapsed().as_secs_f64(), c, "");
    let t = Instant::now();
    let mut sink = Sink(0, std::env::args().any(|a| a == "--sha").then(Sha256::new));
    let opz = mzlab_core::mzmlwrite::Opzioni {
        thread: threads(),
        ..Default::default()
    };
    lettori_mzml::scrivi_con(&mut *l, 0, &mut sink, &mut |_, _| {}, opz).expect("mzML");
    line(
        "write mzML (scrivi)",
        t.elapsed().as_secs_f64(),
        c,
        &format!(
            "{:.1} MB{}",
            sink.0 as f64 / 1e6,
            sink.1.map_or(String::new(), |h| format!(
                ", sha256 {}",
                h.finalize()
                    .iter()
                    .take(6)
                    .map(|x| format!("{x:02x}"))
                    .collect::<String>()
            ))
        ),
    );
}

/// Every phase of the site for one reading of the file (`cen`: through `lettori_centroidi`).
fn phases(path: &str, name: &str, cen: bool, with_mzml: bool) {
    println!(
        "-- {}",
        if cen {
            "centroids (lettori_centroidi)"
        } else {
            "as in the file"
        }
    );
    // phase 1: the fingerprint (SHA-256 + probe) of the member
    let (mut fs, c) = set(path);
    let t = Instant::now();
    let imp = upload::impronte(&mut fs).expect("fingerprint");
    line("fingerprint (SHA-256)", t.elapsed().as_secs_f64(), &c, "");
    let (mut l, c, t_open) = open(path, cen);
    let n = l.indice(0).unwrap().len();
    line("open (indexes)", t_open, &c, &format!("{n} scans"));

    // the decoding alone: every scan once
    let t = Instant::now();
    let mut peaks = 0u64;
    let mut levels = [0usize; 4];
    for k in 0..n {
        let s = l.scansione(0, k).unwrap();
        peaks += s.mz.len() as u64;
        levels[usize::from(s.level.min(3))] += 1;
    }
    line(
        "decode every scan",
        t.elapsed().as_secs_f64(),
        &c,
        &format!(
            "{peaks} peaks, MS1 {} MS2 {} MSn {}",
            levels[1], levels[2], levels[3]
        ),
    );

    // phase 2: the scan table (`righe`: every scan decoded once, the rows built from the scan with no mzML text)
    let t = Instant::now();
    let p = upload::prepare_lettore(&mut *l, 0, &imp, &format!("{name}.mzML")).expect("table");
    line("scan table (righe)", t.elapsed().as_secs_f64(), &c, "");

    // phase 3: the .mzidx by itself (pass 1 and the rounds decode the scans again)
    let t = Instant::now();
    let mut out = Cursor::new(Vec::new());
    let st = mzidx::build_lettore(&p.table, &mut *l, 0, &mut out, 192 << 20).expect("build");
    line(
        "build .mzidx",
        t.elapsed().as_secs_f64(),
        &c,
        &format!(
            "{:.1} MB, {} sections, {} passes, sha256 {}",
            st.bytes as f64 / 1e6,
            st.sections,
            st.passes,
            hash12(out.get_ref())
        ),
    );

    // phases 2 and 3 in one call, as the site does: the table and pass 1 share one decoding of the scans
    let t = Instant::now();
    let mut out = Cursor::new(Vec::new());
    let (st, _) = upload::indicizza_lettore(
        &mut *l,
        0,
        &imp,
        &format!("{name}.mzML"),
        &mut out,
        192 << 20,
        &mut |_| {},
    )
    .expect("index");
    line(
        "table + .mzidx (one call)",
        t.elapsed().as_secs_f64(),
        &c,
        &format!("{:.1} MB, {} passes", st.bytes as f64 / 1e6, st.passes),
    );

    scratch_phases(path, name, cen, &mut *l, &imp, &c);

    if with_mzml {
        mzml(&mut *l, n, &c);
    }
}
