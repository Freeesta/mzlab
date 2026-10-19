//! Entry points of the fuzz targets of the readers modules (`fuzz_targets/lettori_<famiglia>.rs`): arbitrary bytes dressed as the members of a dataset of the family must return, with a
//! result or an error, never panic or loop. Kept apart from `ingressi.rs` (which `mzlab-lib` includes in its tests) because it needs `mzlab-lettori`;
//! `crates/mzlab-lettori/tests/fuzz_lettori.rs` runs the same functions on a few inputs in `cargo test`.

use mzlab_core::lettori::{FileSet, Limiti, Membro};

fn insieme(membri: Vec<(&str, Vec<u8>)>) -> FileSet {
    FileSet {
        membri: membri
            .into_iter()
            .map(|(nome, b)| Membro {
                nome: nome.into(),
                dim: b.len() as u64,
                dati: Box::new(b),
                sha1: None,
            })
            .collect(),
    }
}

/// Opens `data` as the members `nomi` of one dataset through the registry of the readers module and reads what a page reads first: `metodo/1`, index, a few scans and their
/// headers, the chromatograms. With two names the input is a u32 LE length, that many bytes for the first member and the rest for the second (as `ingressi::wiff`).
fn lettori_famiglia(nomi: &[&str], data: &[u8]) {
    let membri: Vec<(&str, Vec<u8>)> = match nomi {
        [uno] => vec![(*uno, data.to_vec())],
        [a, b] => {
            let (Some(n), Some(corpo)) = (data.get(..4), data.get(4..)) else {
                return;
            };
            let n = (u32::from_le_bytes([n[0], n[1], n[2], n[3]]) as usize).min(corpo.len());
            let (x, y) = corpo.split_at(n);
            vec![(*a, x.to_vec()), (*b, y.to_vec())]
        }
        _ => return,
    };
    let _ = mzlab_lettori::riconosci(&mut insieme(membri.clone()));
    let Ok(mut l) = mzlab_lettori::apri(insieme(membri), &Limiti::sito()) else {
        return;
    };
    let _ = l.metodo(0);
    let Ok(ix) = l.indice(0).map(<[_]>::to_vec) else {
        return;
    };
    for j in [0, ix.len() / 2, ix.len().saturating_sub(1)] {
        if j < ix.len() {
            let _ = l.scansione(0, j);
            let _ = l.intestazione(0, j);
        }
    }
    if let Ok(cr) = l.cromatogrammi(0) {
        for v in cr.iter().take(2) {
            let _ = l.cromatogramma(0, &v.id);
        }
    }
}

/// The reader(s) of the family `aperti` over arbitrary bytes.
pub fn lettori_aperti(data: &[u8]) {
    lettori_famiglia(&["x.mzML"], data);
    lettori_famiglia(&["x.mzXML"], data);
}

/// The reader(s) of the family `andi` over arbitrary bytes.
pub fn lettori_andi(data: &[u8]) {
    lettori_famiglia(&["x.cdf"], data);
}

/// The reader(s) of the family `thermo` over arbitrary bytes.
pub fn lettori_thermo(data: &[u8]) {
    lettori_famiglia(&["x.raw"], data);
}

/// The reader(s) of the family `agilent` over arbitrary bytes.
pub fn lettori_agilent(data: &[u8]) {
    lettori_famiglia(&["x.D/AcqData/MSScan.bin", "x.D/AcqData/MSPeak.bin"], data);
}

/// The reader(s) of the family `sciex` over arbitrary bytes.
pub fn lettori_sciex(data: &[u8]) {
    lettori_famiglia(&["x.wiff", "x.wiff.scan"], data);
}

/// The reader(s) of the family `waters` over arbitrary bytes.
pub fn lettori_waters(data: &[u8]) {
    lettori_famiglia(&["x.raw/_FUNC001.DAT", "x.raw/_HEADER.TXT"], data);
}

/// The reader(s) of the family `shimadzu` over arbitrary bytes.
pub fn lettori_shimadzu(data: &[u8]) {
    lettori_famiglia(&["x.qgd"], data);
    lettori_famiglia(&["x.lcd"], data);
}

/// The reader(s) of the family `bruker` over arbitrary bytes.
pub fn lettori_bruker(data: &[u8]) {
    lettori_famiglia(&["x.d/analysis.tdf", "x.d/analysis.tdf_bin"], data);
    lettori_famiglia(&["x.d/analysis.tsf", "x.d/analysis.tsf_bin"], data);
}
