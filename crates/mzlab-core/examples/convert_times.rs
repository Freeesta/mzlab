//! Time, memory and checksum of a conversion to mzML, native: `cargo run --release -p mzlab-core --example convert_times -- FILE.mzML [--thread N] [--peaks file|profile|mzlab]`
//! (FILE from `tools/mzml_enorme.py`). The core opens no mzML through its registry, so the file is read whole into a made-up reader and `Conversione::scrivi` converts that;
//! the memory of the loaded file is left out of the heap peak. The mzML goes to a sink that only counts and hashes it; the heap peak is counted by the allocator
//! (as `mzml_memory`), the process peak is the `VmHWM` of Linux.
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{ChromatogramEntry, Field, Format, Reader, Sample, ScanEntry};
use mzlab_core::lettori_mzml::{Conversion, Peaks};
use mzlab_core::mzml::{self, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::Options;
use sha2::{Digest as _, Sha256};

struct Tally;
static VIVI: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
// SAFETY: forwards every call to the system allocator and only counts the bytes.
unsafe impl GlobalAlloc for Tally {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        unsafe {
            let v = VIVI.fetch_add(l.size(), Ordering::Relaxed) + l.size();
            PEAK.fetch_max(v, Ordering::Relaxed);
            System.alloc(l)
        }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe {
            VIVI.fetch_sub(l.size(), Ordering::Relaxed);
            System.dealloc(p, l)
        }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        unsafe {
            let v = (VIVI.fetch_add(n, Ordering::Relaxed) + n).saturating_sub(l.size());
            VIVI.fetch_sub(l.size(), Ordering::Relaxed);
            PEAK.fetch_max(v, Ordering::Relaxed);
            System.realloc(p, l, n)
        }
    }
}
#[global_allocator]
static A: Tally = Tally;

/// Counts what the writer writes and hashes it on a helper thread, so the hash stays off the conversion's critical path as a file write would.
struct Pozzo {
    tx: Option<std::sync::mpsc::SyncSender<Vec<u8>>>,
    sha: Option<std::thread::JoinHandle<Sha256>>,
    byte: u64,
}
impl Pozzo {
    fn nuovo() -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(256);
        let sha = std::thread::spawn(move || {
            let mut h = Sha256::new();
            rx.iter().for_each(|b| h.update(&b));
            h
        });
        Pozzo {
            tx: Some(tx),
            sha: Some(sha),
            byte: 0,
        }
    }
    fn fine(mut self) -> (String, u64) {
        drop(self.tx.take());
        let h = self.sha.take().expect("sha").join().expect("join");
        (
            h.finalize().iter().map(|x| format!("{x:02x}")).collect(),
            self.byte,
        )
    }
}
impl Write for Pozzo {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.byte += b.len() as u64;
        self.tx
            .as_ref()
            .expect("open")
            .send(b.to_vec())
            .map_err(|_| std::io::ErrorKind::BrokenPipe)?;
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A reader over the spectra of an mzML already in memory: a scan is a copy of the spectrum, as a reader gives it.
struct DaMzml {
    run: mzml::Run,
    campioni: Vec<Sample>,
    indice: Vec<ScanEntry>,
}

impl DaMzml {
    fn nuovo(run: mzml::Run) -> Self {
        let index = run
            .spectra
            .iter()
            .map(|s| ScanEntry {
                id: s.id.clone(),
                rt: s.rt,
                livello: s.level,
                polarita: s.polarity,
                tic: 0.0,
                base_peak: None,
                filtro: s.filter.clone(),
                precursore: s.precursor,
                position: None,
            })
            .collect();
        let samples = vec![Sample {
            id: 1,
            nome: "sintetico".into(),
            scansioni: run.spectra.len(),
            size: None,
        }];
        DaMzml {
            run,
            campioni: samples,
            indice: index,
        }
    }
}

impl Reader for DaMzml {
    fn format(&self) -> Format {
        Format::Mzml
    }
    fn samples(&self) -> &[Sample] {
        &self.campioni
    }
    fn index(&mut self, _: usize) -> Result<&[ScanEntry]> {
        Ok(&self.indice)
    }
    fn scan(&mut self, _: usize, k: usize) -> Result<Spectrum> {
        self.run
            .spectra
            .get(k)
            .cloned()
            .ok_or_else(|| Error::new("error.badIndex"))
    }
    fn scan_header(&mut self, _: usize, _: usize) -> Result<Vec<Field>> {
        Ok(Vec::new())
    }
    fn chromatograms(&mut self, _: usize) -> Result<Vec<ChromatogramEntry>> {
        Ok(Vec::new())
    }
    fn chromatogram(&mut self, _: usize, _: &str) -> Result<Chromatogram> {
        Err(Error::new("error.badIndex"))
    }
    fn method(&mut self, _: usize) -> Result<String> {
        Ok(String::new())
    }
}

/// Peak resident set of the process so far, in MB (`VmHWM`).
fn rss_max() -> f64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let kb = s
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<f64>().ok());
    kb.unwrap_or(0.0) / 1024.0
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let Some(file) = a.get(1).filter(|f| !f.starts_with("--")) else {
        eprintln!("usage: convert_times FILE [--thread N] [--peaks file|profile|mzlab]");
        std::process::exit(2);
    };
    let after = |name: &str| a.iter().position(|x| x == name).and_then(|i| a.get(i + 1));
    let mut opz = Options::default();
    if let Some(n) = after("--thread") {
        opz.thread = n.parse().expect("--thread N");
    }
    let peaks = match after("--peaks").map(String::as_str) {
        None | Some("file") => Peaks::File,
        Some("profile") => Peaks::Profilo,
        Some("mzlab") => Peaks::Mzlab,
        Some(x) => panic!("--peaks file|profile|mzlab, not {x}"),
    };
    let run = mzml::read(std::io::BufReader::new(
        std::fs::File::open(file).expect("open"),
    ))
    .expect("read mzML");
    let mut l = DaMzml::nuovo(run);
    let base = VIVI.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    let t = Instant::now();
    let conv = Conversion {
        peaks,
        opz,
        ..Conversion::default()
    };
    let pozzo = conv
        .write(&mut l, 0, Pozzo::nuovo(), &mut |_, _| {})
        .expect("convert");
    let s = t.elapsed().as_secs_f64();
    let peak = PEAK.load(Ordering::Relaxed).saturating_sub(base) as f64 / 1048576.0;
    let (sha, byte) = pozzo.fine();
    println!(
        "threads {} | {s:.2} s | heap peak {peak:.1} MB | process peak {:.0} MB | {} bytes | sha256 {sha}",
        opz.thread,
        rss_max(),
        byte
    );
}
