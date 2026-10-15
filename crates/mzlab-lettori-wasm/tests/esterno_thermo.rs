#![cfg(feature = "thermo")]
//! The external reader of the engine (`mzlab_wasm::esterno::LettoreEsterno`) over the session of the `thermo` module gives, for a real `.raw`, exactly what the Thermo reader gives when it is linked
//! in the engine: the index, the scans, the headers, the TIC, the `metodo/1` document, the header of the mzML and the whole mzML byte for byte. Needs the private data repository
//! (MZLAB_DATI, or `mzlab-dati` beside this one); skipped without it. Run in release: `cargo test --release -p mzlab-lettori-wasm --features thermo --test esterno_thermo`.
#![allow(clippy::indexing_slicing)]

use std::io::Write;
use std::path::{Path, PathBuf};

use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};
use mzlab_core::{lettori_mzml, rawlettore, Result};
use mzlab_lettori_wasm::sessione::{Scansione, Sessioni};
use mzlab_wasm::esterno::{LettoreEsterno, Modulo};

fn raw_files() -> Vec<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for up in Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .skip(1)
        .take(3)
    {
        cands.push(up.join("mzlab-dati"));
    }
    let Some(d) = cands.into_iter().find(|p| p.join("HRMS").is_dir()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut stack = vec![d.join("HRMS")];
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
    let nome = path.file_name().unwrap().to_string_lossy().to_string();
    FileSet {
        membri: vec![Membro {
            nome,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
        }],
    }
}

/// The session of the module as the `Modulo` the engine sees (what `ModuloJs` does through the JS functions).
struct ViaSessione {
    s: Sessioni,
    h: u32,
}

impl Modulo for ViaSessione {
    fn campioni(&mut self) -> Result<String> {
        self.s.campioni(self.h)
    }
    fn indice(&mut self, c: usize) -> Result<String> {
        self.s.indice(self.h, c)
    }
    fn scansione(&mut self, c: usize, k: usize) -> Result<(Vec<f64>, Vec<f64>, String)> {
        let Scansione {
            mz,
            intensita,
            meta,
        } = self.s.scansione(self.h, c, k)?;
        Ok((mz, intensita, meta))
    }
    fn intestazione(&mut self, c: usize, k: usize) -> Result<String> {
        self.s.intestazione(self.h, c, k)
    }
    fn cromatogrammi(&mut self, c: usize) -> Result<String> {
        self.s.cromatogrammi(self.h, c)
    }
    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<String> {
        self.s.cromatogramma(self.h, c, id)
    }
    fn metodo(&mut self, c: usize) -> Result<String> {
        self.s.metodo(self.h, c)
    }
    fn testata(&mut self, c: usize) -> Result<String> {
        self.s.testata(self.h, c)
    }
}

/// FNV-1a over what is written, and how much.
struct Impronta(u64, u64);

impl Write for Impronta {
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

fn mzml(l: &mut dyn Lettore) -> (u64, u64) {
    let w = lettori_mzml::scrivi(l, 0, Impronta(0xcbf2_9ce4_8422_2325, 0), &mut |_, _| {}).unwrap();
    (w.0, w.1)
}

/// Scan `k` and its header, from the external reader and from the linked one.
fn same_scan(ext: &mut dyn Lettore, diretto: &mut dyn Lettore, k: usize, nome: &str) {
    let (a, b) = (
        ext.scansione(0, k).unwrap(),
        diretto.scansione(0, k).unwrap(),
    );
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
        "{nome}: scan {k}"
    );
    assert_eq!(
        (&a.intensity, &a.wide),
        (&b.intensity, &b.wide),
        "{nome}: scan {k} intensities"
    );
    assert_eq!(
        ext.intestazione(0, k).unwrap(),
        diretto.intestazione(0, k).unwrap(),
        "{nome}: header {k}"
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
        let mut diretto = (rawlettore::VOCE.apri)(set(f), &Limiti::sito()).unwrap();
        let mut s = Sessioni::default();
        let h = s.inserisci(mzlab_lettori::apri_nudo(set(f), &Limiti::sito()).unwrap());
        let mut ext = LettoreEsterno::new(ViaSessione { s, h }).unwrap();
        let nome = f.display();
        assert_eq!(ext.formato(), diretto.formato(), "{nome}");
        assert_eq!(ext.campioni(), diretto.campioni(), "{nome}");
        let indice = diretto.indice(0).unwrap().to_vec();
        assert_eq!(ext.indice(0).unwrap(), &indice[..], "{nome}: index");
        let passo = (indice.len() / 20).max(1);
        for k in (0..indice.len()).step_by(passo) {
            same_scan(&mut ext, &mut *diretto, k, &nome.to_string());
        }
        assert_eq!(
            ext.cromatogrammi(0).unwrap(),
            diretto.cromatogrammi(0).unwrap(),
            "{nome}"
        );
        assert_eq!(
            ext.cromatogramma(0, "TIC").unwrap(),
            diretto.cromatogramma(0, "TIC").unwrap(),
            "{nome}: TIC"
        );
        assert_eq!(
            ext.metodo(0).unwrap(),
            diretto.metodo(0).unwrap(),
            "{nome}: metodo/1"
        );
        assert_eq!(
            format!("{:?}", ext.testata_mzml(0).unwrap()),
            format!("{:?}", diretto.testata_mzml(0).unwrap()),
            "{nome}: header of the mzML"
        );
        let (a, b) = (mzml(&mut ext), mzml(&mut *diretto));
        assert_eq!(a, b, "{nome}: the mzML");
        eprintln!(
            "{nome}: {} scans, mzML of {} bytes identical",
            indice.len(),
            a.1
        );
    }
}
