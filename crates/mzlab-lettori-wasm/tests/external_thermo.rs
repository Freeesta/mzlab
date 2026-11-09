#![cfg(feature = "thermo")]
//! The external reader of the engine (`mzlab_wasm::external::ExternalReader`) over the session of the `thermo` module gives, for a real `.raw`, exactly what the Thermo reader gives when it is linked
//! in the engine: the index, the scans, the headers, the TIC, the `metodo/1` document, the header of the mzML and the whole mzML byte for byte. Needs the private data repository
//! (MZLAB_DATI, or `mzlab-dati` beside this one); skipped without it. Run in release: `cargo test --release -p mzlab-lettori-wasm --features thermo --test external_thermo`.
#![allow(clippy::indexing_slicing)]

use std::io::Write;
use std::path::{Path, PathBuf};

use mzlab_core::readers::{FileSet, Limits, Member, Reader};
use mzlab_core::{rawreader, readers_mzml, Result};
use mzlab_lettori_wasm::session::Sessions;
use mzlab_wasm::external::{ExternalReader, Module};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn raw_files() -> Vec<PathBuf> {
    let Some(hrms) = dati_percorsi::percorso("dati/HRMS").filter(|p| p.is_dir()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut stack = vec![hrms];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("raw")) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn set(path: &Path) -> FileSet {
    let bytes = std::fs::read(path).unwrap();
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    FileSet {
        membri: vec![Member {
            nome: name,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }],
    }
}

/// The session of the module as the `Module` the engine sees (what `JsModule` does through the JS functions).
struct ViaSession {
    s: Sessions,
    h: u32,
}

impl Module for ViaSession {
    fn samples(&mut self) -> Result<String> {
        self.s.samples(self.h)
    }
    fn protocol(&mut self) -> Result<u32> {
        Ok(mzlab_core::transport::PROTOCOLLO)
    }
    fn index(&mut self, c: usize) -> Result<Vec<u8>> {
        self.s.index(self.h, c)
    }
    fn scan(&mut self, c: usize, k: usize) -> Result<Vec<u8>> {
        self.s.scan(self.h, c, k)
    }
    fn scan_header(&mut self, c: usize, k: usize) -> Result<String> {
        self.s.scan_header(self.h, c, k)
    }
    fn chromatograms(&mut self, c: usize) -> Result<String> {
        self.s.chromatograms(self.h, c)
    }
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<String> {
        self.s.chromatogram(self.h, c, id)
    }
    fn method(&mut self, c: usize) -> Result<String> {
        self.s.method(self.h, c)
    }
    fn file_header(&mut self, c: usize) -> Result<String> {
        self.s.file_header(self.h, c)
    }
}

/// FNV-1a over what is written, and how much.
struct Fingerprint(u64, u64);

impl Write for Fingerprint {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        for &x in b {
            self.0 = (self.0 ^ u64::from(x)).wrapping_mul(0x100_0000_01b3);
        }
        self.1 += b.len() as u64;
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn mzml(l: &mut dyn Reader) -> (u64, u64) {
    let w =
        readers_mzml::scrivi(l, 0, Fingerprint(0xcbf2_9ce4_8422_2325, 0), &mut |_, _| {}).unwrap();
    (w.0, w.1)
}

/// Scan `k` and its header, from the external reader and from the linked one.
fn same_scan(ext: &mut dyn Reader, diretto: &mut dyn Reader, k: usize, name: &str) {
    let (a, b) = (ext.scan(0, k).unwrap(), diretto.scan(0, k).unwrap());
    assert_eq!(
        (
            a.index,
            &a.id,
            a.level,
            a.rt,
            a.polarity,
            a.profile,
            &a.filter,
            a.precursor,
            &a.mz
        ),
        (
            k,
            &b.id,
            b.level,
            b.rt,
            b.polarity,
            b.profile,
            &b.filter,
            b.precursor,
            &b.mz
        ),
        "{name}: scan {k}"
    );
    assert_eq!(
        (&a.intensity, &a.wide),
        (&b.intensity, &b.wide),
        "{name}: scan {k} intensities"
    );
    assert_eq!(
        ext.scan_header(0, k).unwrap(),
        diretto.scan_header(0, k).unwrap(),
        "{name}: header {k}"
    );
}

#[test]
fn the_external_reader_gives_what_the_linked_one_gives() {
    let files = raw_files();
    if files.is_empty() {
        eprintln!("SKIP esterno_thermo: no data repository (MZLAB_DATI)");
        return;
    }
    for f in &files {
        let mut diretto = (rawreader::ENTRY.apri)(set(f), &Limits::sito()).unwrap();
        let mut s = Sessions::default();
        let h = s.insert(mzlab_lettori::open_bare(set(f), &Limits::sito()).unwrap());
        let mut ext = ExternalReader::new(ViaSession { s, h }).unwrap();
        let name = f.display();
        assert_eq!(ext.format(), diretto.format(), "{name}");
        assert_eq!(ext.samples(), diretto.samples(), "{name}");
        let index = diretto.index(0).unwrap().to_vec();
        assert_eq!(ext.index(0).unwrap(), &index[..], "{name}: index");
        let step = (index.len() / 20).max(1);
        for k in (0..index.len()).step_by(step) {
            same_scan(&mut ext, &mut *diretto, k, &name.to_string());
        }
        assert_eq!(
            ext.chromatograms(0).unwrap(),
            diretto.chromatograms(0).unwrap(),
            "{name}"
        );
        assert_eq!(
            ext.chromatogram(0, "TIC").unwrap(),
            diretto.chromatogram(0, "TIC").unwrap(),
            "{name}: TIC"
        );
        assert_eq!(
            ext.method(0).unwrap(),
            diretto.method(0).unwrap(),
            "{name}: metodo/1"
        );
        assert_eq!(
            format!("{:?}", ext.mzml_header(0).unwrap()),
            format!("{:?}", diretto.mzml_header(0).unwrap()),
            "{name}: header of the mzML"
        );
        let (a, b) = (mzml(&mut ext), mzml(&mut *diretto));
        assert_eq!(a, b, "{name}: the mzML");
        eprintln!(
            "{name}: {} scans, mzML of {} bytes identical",
            index.len(),
            a.1
        );
    }
}
