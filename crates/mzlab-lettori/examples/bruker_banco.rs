//! Speed bench of the Bruker timsTOF reader (docs/agenti/misure-formati.md) on a synthetic DDA-PASEF folder of realistic size (invented numbers, no lab data):
//! `scrivi` writes `analysis.tdf` + `analysis.tdf_bin` (zstd, codec 2), `misura` times open, index, TIC, BPC, first spectrum, all spectra and the mzML conversion.
//! A tool of the bench: it never enters a .wasm.
//!   bruker_banco scrivi DIR.d [--cicli 100] [--ms1 60000] [--ms2 15000]
//!   bruker_banco misura DIR.d [--giri 3]
use std::path::Path;
use std::time::Instant;

use mzlab_core::lettori::{FileSet, Limiti, Membro};

#[path = "../tests/comune_sqlite/mod.rs"]
mod comune_sqlite;
use comune_sqlite::pasef;

fn scrivi(dir: &Path, cicli: usize, n1: usize, n2: usize) {
    std::fs::create_dir_all(dir).unwrap();
    let (tdf, bin) = pasef::cartella(cicli, n1, n2);
    std::fs::write(dir.join("analysis.tdf"), tdf).unwrap();
    std::fs::write(dir.join("analysis.tdf_bin"), &bin).unwrap();
    eprintln!("{}: tdf_bin {} MB", dir.display(), bin.len() / 1_000_000);
}

fn insieme(dir: &Path) -> FileSet {
    let nome = dir.file_name().unwrap().to_string_lossy().to_string();
    let membri = ["analysis.tdf", "analysis.tdf_bin"]
        .iter()
        .map(|m| {
            let f = std::fs::File::open(dir.join(m)).unwrap();
            let dim = f.metadata().unwrap().len();
            Membro {
                nome: format!("{nome}/{m}"),
                dim,
                dati: Box::new(f),
                sha1: None,
            }
        })
        .collect();
    FileSet { membri }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn misura(dir: &Path, giri: usize) {
    for g in 0..giri {
        let t = Instant::now();
        let mut l = mzlab_lettori::apri(insieme(dir), &Limiti::desktop()).unwrap();
        let apri = ms(t);
        let t = Instant::now();
        let n = l.indice(0).unwrap().len();
        let indice = ms(t);
        let t = Instant::now();
        std::hint::black_box(l.cromatogramma(0, "TIC").unwrap());
        let tic = ms(t);
        let t = Instant::now();
        std::hint::black_box(l.cromatogramma(0, "BPC").unwrap());
        let bpc = ms(t);
        let t = Instant::now();
        std::hint::black_box(l.scansione(0, 0).unwrap());
        let primo = ms(t);
        let t = Instant::now();
        let (mut punti, mut firma) = (0usize, 0xcbf2_9ce4_8422_2325u64);
        for k in 0..n {
            let s = l.scansione(0, k).unwrap();
            punti += s.mz.len();
            // FNV-1a over the bits of every point: the same number before and after a change of the reader
            for (m, y) in s.mz.iter().zip(&s.intensity) {
                for b in [m.to_bits(), u64::from(y.to_bits())] {
                    firma = (firma ^ b).wrapping_mul(0x100_0000_01b3);
                }
            }
        }
        let tutti = ms(t);
        let t = Instant::now();
        let mut l = mzlab_lettori::apri(insieme(dir), &Limiti::desktop()).unwrap();
        mzlab_core::lettori_mzml::scrivi(&mut *l, 0, std::io::sink(), &mut |_, _| {}).unwrap();
        let mzml = ms(t);
        println!(
            "{{\"giro\":{g},\"spettri\":{n},\"punti\":{punti},\"firma\":\"{firma:016x}\",\"apri_ms\":{apri:.1},\"indice_ms\":{indice:.1},\"tic_ms\":{tic:.1},\"bpc_ms\":{bpc:.1},\"primo_ms\":{primo:.2},\"tutti_ms\":{tutti:.1},\"mzml_ms\":{mzml:.1}}}"
        );
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let opz = |k: &str, d: usize| {
        a.iter()
            .position(|x| x == k)
            .and_then(|i| a.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    match (a.first().map(String::as_str), a.get(1)) {
        (Some("scrivi"), Some(d)) => scrivi(Path::new(d), opz("--cicli", 100), opz("--ms1", 60_000), opz("--ms2", 15_000)),
        (Some("misura"), Some(d)) => misura(Path::new(d), opz("--giri", 3)),
        _ => eprintln!("usage: bruker_banco scrivi DIR.d [--cicli N] [--ms1 N] [--ms2 N] | misura DIR.d [--giri N]"),
    }
}
