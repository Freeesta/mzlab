//! `cargo run --release -p mzlab-lib --example build -- IN.(msp|mgf|json|txt) OUT.mzlib [SCRATCH]`
//! Builds a library from a file read in blocks (the scratch space is a temporary file) and prints time, size and memory.
use mzlab_lib::build::{BuildOpts, Builder};
use mzlab_lib::error::{Error, Result};
use mzlab_lib::io::{Sink, Source};
use mzlab_lib::parse::{detect, Parser};
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

pub struct FileIo(RefCell<File>);

impl Source for FileIo {
    fn len(&self) -> u64 {
        self.0.borrow().metadata().map(|m| m.len()).unwrap_or(0)
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<()> {
        let mut f = self.0.borrow_mut();
        f.seek(SeekFrom::Start(off))
            .and_then(|_| f.read_exact(buf))
            .map_err(|_| Error::new("lib.err.io"))
    }
}
impl Sink for FileIo {
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()> {
        let mut f = self.0.borrow_mut();
        f.seek(SeekFrom::Start(off))
            .and_then(|_| f.write_all(data))
            .map_err(|_| Error::new("lib.err.io"))
    }
}

fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find(|l| l.starts_with("VmHWM:")).and_then(|l| {
                l.split_whitespace()
                    .nth(1)
                    .and_then(|v| v.parse::<f64>().ok())
            })
        })
        .map_or(0.0, |kb| kb / 1024.0)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (input, output) = (&a[1], &a[2]);
    let scratch = a
        .get(3)
        .cloned()
        .unwrap_or_else(|| format!("{output}.scratch"));
    let t0 = std::time::Instant::now();
    let mut f = File::open(input).unwrap();
    let mut head = vec![0u8; 8192];
    let n = f.read(&mut head).unwrap();
    head.truncate(n);
    let fmt = detect(input, &head).unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    let sc = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&scratch)
        .unwrap();
    let mut b = Builder::new(FileIo(RefCell::new(sc)), BuildOpts::default());
    let mut err = None;
    let mut p = Parser::new(fmt, |s| {
        if let Err(e) = b.add(&s) {
            err = Some(e);
        }
    });
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        p.push(&buf[..n]);
    }
    p.end();
    let st = p.stats;
    drop(p);
    if let Some(e) = err {
        panic!("{e}");
    }
    let t_parse = t0.elapsed();
    let out = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(output)
        .unwrap();
    let mut sink = FileIo(RefCell::new(out));
    let info = b.finish(&mut sink).unwrap();
    let total = t0.elapsed();
    let in_len = std::fs::metadata(input).unwrap().len();
    let _ = std::fs::remove_file(&scratch);
    println!(
        "{:?}: {} spectra ({} dropped, {} emptied), {} peaks; {:.1} MB -> {:.1} MB ({:.1}x); parse+clean {:.1}s, total {:.1}s; peak RSS {:.0} MB",
        fmt,
        info.n_spectra,
        st.dropped,
        info.emptied,
        info.n_peaks,
        in_len as f64 / 1e6,
        info.bytes as f64 / 1e6,
        in_len as f64 / info.bytes as f64,
        t_parse.as_secs_f64(),
        total.as_secs_f64(),
        rss_mb()
    );
}
