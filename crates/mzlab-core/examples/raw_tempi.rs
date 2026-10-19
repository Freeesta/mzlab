//! Times every phase the site goes through for a Thermo `.raw` (`crates/mzlab-wasm/src/nativi.rs`): fingerprint, scan table, `.mzidx`,
//! «Scarica .mzML», each with the reader as it is and with the centroids of `lettori_centroidi`. The member is read through a 256 KB block
//! cache like the `thermo` module's `JsMember`, and the number of reads asked of it is counted (in the browser each one is a JS call).
//! `cargo run --release --example raw_tempi -- FILE.raw [--no-cen] [--no-mzml]`; numbers in `docs/studi/RAW_VELOCE.md`.
use std::cell::Cell;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::rc::Rc;
use std::time::Instant;

use sha2::{Digest, Sha256};

use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{self, FileSet, Lettore, Limiti, Membro};
use mzlab_core::{lettori_centroidi, lettori_mzml, mzidx, upload};

const BLOCK: usize = 256 << 10;

/// A file read in blocks of 256 KB (the cache of `mzlab-lettori-wasm`); `calls` counts the reads asked of it, `fills` the blocks read.
struct Blocks {
    f: File,
    dim: u64,
    start: u64,
    block: Vec<u8>,
    calls: Rc<Cell<u64>>,
    fills: Rc<Cell<u64>>,
}

impl ReadAt for Blocks {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        self.calls.set(self.calls.get() + 1);
        let end = off + buf.len() as u64;
        if !(off >= self.start && end <= self.start + self.block.len() as u64) {
            let n = BLOCK.min(self.dim.saturating_sub(off) as usize);
            self.fills.set(self.fills.get() + 1);
            if buf.len() >= BLOCK || n < buf.len() {
                self.f.seek(SeekFrom::Start(off)).unwrap();
                self.f
                    .read_exact(buf)
                    .map_err(|_| mzlab_core::Error::new("error.badIndex"))?;
                return Ok(());
            }
            self.block.resize(n, 0);
            self.f.seek(SeekFrom::Start(off)).unwrap();
            self.f.read_exact(&mut self.block).unwrap();
            self.start = off;
        }
        let a = (off - self.start) as usize;
        buf.copy_from_slice(&self.block[a..a + buf.len()]);
        Ok(())
    }
}

struct Counters {
    calls: Rc<Cell<u64>>,
    fills: Rc<Cell<u64>>,
}

impl Counters {
    fn take(&self) -> (u64, u64) {
        (self.calls.replace(0), self.fills.replace(0))
    }
}

fn set(path: &str) -> (FileSet, Counters) {
    let dim = std::fs::metadata(path).unwrap().len();
    let c = Counters {
        calls: Rc::default(),
        fills: Rc::default(),
    };
    let b = Blocks {
        f: File::open(path).unwrap(),
        dim,
        start: 0,
        block: Vec::new(),
        calls: c.calls.clone(),
        fills: c.fills.clone(),
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

/// Counts the bytes written.
#[derive(Default)]
struct Sink(u64);
impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0 += b.len() as u64;
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
    let (calls, fills) = c.take();
    println!("{what:<28} {secs:>8.3} s  reads {calls:>10}  blocks {fills:>7}  {extra}");
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
        let t = Instant::now();
        let mut sink = Sink::default();
        lettori_mzml::scrivi(&mut *l, 0, &mut sink, &mut |_, _| {}).expect("mzML");
        line(
            "write mzML (scrivi)",
            t.elapsed().as_secs_f64(),
            &c,
            &format!("{:.1} MB", sink.0 as f64 / 1e6),
        );
    }
}
