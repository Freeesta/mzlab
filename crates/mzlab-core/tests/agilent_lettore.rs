//! The Agilent `.D` read through the `Lettore` contract: the same scans, the same index and chromatograms as `AgilentD::run`, read by position from the files of the
//! folder (real runs, skipped without the data repository), and the refusals of a damaged or oversized set (synthetic).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod agilent_folder;
mod agilent_synth;

use agilent_folder::*;
use agilent_synth::*;
use mzlab_core::lettori::{self, FileSet, Formato, Lettore, Limiti, Membro};
use mzlab_core::mzml::{ChromKind, Run};

/// The files of a real folder as a `FileSet`: the members are read from disk by offset, never loaded.
fn set_of(dir: &std::path::Path) -> FileSet {
    let mut membri = Vec::new();
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    for entry in walk(dir) {
        let rel = entry
            .strip_prefix(dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let file = std::fs::File::open(&entry).unwrap();
        membri.push(Membro {
            nome: format!("{name}/{rel}"),
            dim: file.metadata().unwrap().len(),
            dati: Box::new(file),
        });
    }
    FileSet { membri }
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out.sort();
    out
}

#[test]
fn the_reader_gives_the_scans_the_chromatograms_and_the_method_of_the_run() {
    let Some(all) = folders() else { return };
    for f in all {
        let mut fo = Folder::load(&f);
        let d = fo.open();
        let label = d.name.clone();
        let run = d.run(&mut fo.peak).unwrap();
        let mut fs = set_of(&f);
        assert_eq!(
            lettori::riconosci(&mut fs).map(|v| v.id),
            Some("agilent-d"),
            "{label}"
        );
        let mut l = lettori::apri(set_of(&f), &Limiti::sito()).unwrap();
        assert_eq!(l.formato(), Formato::AgilentD);
        assert_eq!(l.campioni().len(), 1);
        assert_eq!(l.campioni()[0].scansioni, run.spectra.len(), "{label}");
        let index = l.indice(0).unwrap().to_vec();
        assert_eq!(index.len(), run.spectra.len(), "{label}");
        for (k, (e, s)) in index.iter().zip(&run.spectra).enumerate() {
            assert_eq!(
                (e.id.as_str(), e.rt, e.livello, e.polarita),
                (s.id.as_str(), s.rt, s.level, s.polarity),
                "{label} {k}"
            );
            let p = e.posizione.unwrap();
            let r = d
                .records
                .iter()
                .find(|r| format!("scan={}", r.id) == e.id)
                .unwrap();
            assert_eq!(
                (p.offset, p.lunghezza, e.tic),
                (r.offset, r.bytes, r.tic),
                "{label} {k}"
            );
            assert!(
                s.filter.starts_with(&e.filtro),
                "{label}: {} / {}",
                s.filter,
                e.filtro
            );
        }
        // by position, in any order: the last scan first, then the first
        for k in [index.len() - 1, 0, index.len() / 2] {
            assert_eq!(
                l.scansione(0, k).unwrap(),
                run.spectra[k],
                "{label}: scan {k}"
            );
        }
        assert!(l.scansione(0, index.len()).is_err() && l.scansione(1, 0).is_err());
        let h = l.intestazione(0, 0).unwrap();
        let r0 = d.records.iter().find(|r| r.points > 0).unwrap();
        assert!(
            h.len() >= 19 + r0.extra.len(),
            "{label}: every field of the record"
        );
        assert!(
            h.iter()
                .any(|c| c.nome == "TIC" && c.valore == r0.tic.to_string()),
            "{label}"
        );
        chromatograms_and_method(l.as_mut(), &run, &label);
    }
}

/// The TIC and the BPC from the records, one SIC per ion of a SIM method, and the `metodo/1` document.
fn chromatograms_and_method(l: &mut dyn Lettore, run: &Run, label: &str) {
    let listed = l.cromatogrammi(0).unwrap();
    for c in &run.chromatograms {
        if matches!(c.kind, ChromKind::Tic | ChromKind::Bpc | ChromKind::Sic) {
            assert!(
                listed.iter().any(|v| v.id == c.id && v.tipo == c.kind),
                "{label}: {} is not listed",
                c.id
            );
            assert_eq!(&l.cromatogramma(0, &c.id).unwrap(), c, "{label}: {}", c.id);
        }
    }
    assert!(l.cromatogramma(0, "nothing").is_err());
    let doc = l.metodo(0).unwrap();
    assert!(
        doc.contains("\"formato\":\"agilent-d\"") && doc.contains("MSPeak.bin"),
        "{label}"
    );
}

#[test]
fn a_damaged_or_oversized_set_is_refused_with_a_key() {
    let (idx, peak) = encode(&sample());
    let member = |nome: &str, b: &[u8]| Membro {
        nome: nome.into(),
        dim: b.len() as u64,
        dati: Box::new(b.to_vec()),
    };
    let set = |scan: &[u8], with_peak: bool| FileSet {
        membri: [
            Some(member("x.D/AcqData/MSScan.bin", scan)),
            with_peak.then(|| member("x.D/AcqData/MSPeak.bin", &peak)),
        ]
        .into_iter()
        .flatten()
        .collect(),
    };
    // the good set opens and reads
    let mut l = lettori::apri(set(&idx, true), &Limiti::sito()).unwrap();
    assert_eq!(l.indice(0).unwrap().len(), 3);
    assert_eq!(l.scansione(0, 1).unwrap().mz.len(), 3);
    // without MSPeak.bin the folder is only possible, not sure
    assert_eq!(
        lettori::riconosci(&mut set(&idx, false)).map(|v| v.id),
        Some("agilent-d")
    );
    // a header that is not one, a file of another kind
    let mut bad = idx.clone();
    bad[88..92].copy_from_slice(&i32::MAX.to_le_bytes());
    assert!(lettori::riconosci(&mut set(&bad, true)).is_none());
    assert!(lettori::riconosci(&mut set(b"not an index", true)).is_none());
    assert_eq!(
        lettori::apri(set(b"not an index", true), &Limiti::sito())
            .err()
            .unwrap()
            .key,
        "err.file.notAccepted"
    );
    // a set larger than the bytes allowed is never read
    let tiny = Limiti {
        profondita: 8,
        voci: 5_000_000,
        byte: 100,
    };
    assert_eq!(
        lettori::apri(set(&idx, true), &tiny).err().unwrap().key,
        "error.memoryBudget"
    );
    // a table of more entries than allowed
    let few = Limiti {
        profondita: 8,
        voci: 2,
        byte: 1 << 20,
    };
    assert!(lettori::apri(set(&idx, true), &few).is_err());
}
