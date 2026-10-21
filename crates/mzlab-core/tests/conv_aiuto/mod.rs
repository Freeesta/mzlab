//! The mzML of a file or folder through the registry (`lettori::apri` and `Conversione`): what the tests that used the converters of each format call.
#![allow(dead_code)]

use std::path::Path;

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::lettori_mzml::Conversione;

pub fn membro(nome: &str, dati: &[u8]) -> Membro {
    Membro {
        nome: nome.into(),
        dim: dati.len() as u64,
        dati: Box::new(dati.to_vec()),
        sha1: None,
    }
}

/// The mzML of the first sample of `fs`.
pub fn mzml(fs: FileSet) -> Vec<u8> {
    let mut l = lettori::apri(fs, &Limiti::sito()).unwrap();
    Conversione::default()
        .scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {})
        .unwrap()
}

/// The mzML of one file on disk, named `nome` for the reader.
pub fn mzml_file(path: &Path, nome: &str) -> Vec<u8> {
    let dim = std::fs::metadata(path).unwrap().len();
    mzml(FileSet {
        membri: vec![Membro {
            nome: nome.into(),
            dim,
            dati: Box::new(std::fs::File::open(path).unwrap()),
            sha1: None,
        }],
    })
}

/// The mzML of a vendor folder (`x.D`): every file below it is a member named `x.D/<relative path>`.
pub fn mzml_cartella(folder: &Path) -> Vec<u8> {
    let base = folder.file_name().unwrap().to_string_lossy().into_owned();
    let mut membri = Vec::new();
    let mut stack = vec![folder.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let rel = p.strip_prefix(folder).unwrap().to_string_lossy().replace('\\', "/");
            membri.push(membro(&format!("{base}/{rel}"), &std::fs::read(&p).unwrap()));
        }
    }
    mzml(FileSet { membri })
}

/// The mzML of sample `c` of a `.wiff` pair (`name.wiff`, `name.wiff.scan`) in `dir`; with `centroid` the reader reduces the profile to centroids.
pub fn mzml_wiff(dir: &Path, name: &str, c: usize, centroid: bool) -> Vec<u8> {
    let membri = [name.to_string() + ".wiff", name.to_string() + ".wiff.scan"]
        .iter()
        .map(|n| {
            let p = dir.join(n);
            Membro {
                nome: n.clone(),
                dim: std::fs::metadata(&p).unwrap().len(),
                dati: Box::new(std::fs::File::open(&p).unwrap()),
                sha1: None,
            }
        })
        .collect();
    let l = lettori::apri(FileSet { membri }, &Limiti::desktop()).unwrap();
    let mut l = if centroid {
        mzlab_core::lettori_centroidi::centroidato(l)
    } else {
        l
    };
    Conversione::default()
        .scrivi(&mut *l, c, Vec::new(), &mut |_, _| {})
        .unwrap()
}
