//! The Thermo `.raw` read through the `Lettore` contract gives the same spectra as `raw::read`, with an index built without reading peaks, and is
//! recognised by the registry from its first bytes. Needs the private data repository (MZLAB_DATI or `mzlab-dati` beside this one); skipped without it.
#![allow(clippy::indexing_slicing)]

use std::path::{Path, PathBuf};

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::raw;

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

#[test]
fn a_real_raw_reads_the_same_through_the_registry() {
    let files = raw_files();
    if files.is_empty() {
        eprintln!("SKIP raw_lettore: no data repository (MZLAB_DATI)");
        return;
    }
    for f in files.iter().take(2) {
        let want = raw::read(std::fs::File::open(f).unwrap()).unwrap();
        let mut l = lettori::apri(set(f), &Limiti::sito()).unwrap();
        assert_eq!(l.formato().id(), "thermo-raw");
        assert_eq!(l.campioni().len(), 1);
        let index = l.indice(0).unwrap().to_vec();
        assert!(index.len() >= want.spectra.len());
        let mut checked = 0;
        for w in &want.spectra {
            let k = index
                .iter()
                .position(|r| r.id == w.id)
                .unwrap_or_else(|| panic!("{}: not in the index", w.id));
            assert_eq!(
                (index[k].livello, index[k].rt, index[k].polarita),
                (w.level, w.rt, w.polarity),
                "{}",
                w.id
            );
            assert_eq!(index[k].filtro, w.filter);
            assert!(index[k].posizione.is_some_and(|p| p.lunghezza > 0));
            if checked % 7 == 0 {
                let got = l.scansione(0, k).unwrap();
                assert_eq!(
                    (
                        &got.id,
                        got.level,
                        got.profile,
                        got.precursor,
                        &got.mz,
                        &got.intensity
                    ),
                    (&w.id, w.level, w.profile, w.precursor, &w.mz, &w.intensity)
                );
                assert!(!l.intestazione(0, k).unwrap().is_empty());
            }
            checked += 1;
        }
        let tic = l.cromatogramma(0, "TIC").unwrap();
        assert_eq!(tic.time.len(), tic.intensity.len());
        assert!(!tic.time.is_empty());
        let doc = l.metodo(0).unwrap();
        assert!(doc.contains("thermo-raw"), "{doc}");
        eprintln!("{}: {} scans checked", f.display(), checked);
    }
}

#[test]
fn not_a_raw_is_refused() {
    let mut fs = FileSet {
        membri: vec![Membro {
            nome: "x.raw".into(),
            dim: 20,
            dati: Box::new(vec![7u8; 20]),
        }],
    };
    assert!(lettori::riconosci(&mut fs).is_none());
}
