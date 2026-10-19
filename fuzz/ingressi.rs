//! Entry points of the fuzz targets: every function takes arbitrary bytes and must return, with a result or an error, never panic or loop.
//! Shared by `fuzz_targets/*.rs` (cargo-fuzz) and by `crates/mzlab-lib/tests/fuzz_regressioni.rs` (the stored crashes run in `cargo test`).

use std::io::Cursor;

use mzlab_core::idxread::Index;
use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::ole::Ole;
use mzlab_core::{methodinfo, raw, wiff, xmltree};
use mzlab_lib::parse::{parse_all, Format};

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

/// The container of `.wiff` and `.dam`: directory, streams, hash.
pub fn ole(data: &[u8]) {
    if let Ok(o) = Ole::open(data.to_vec(), "x.wiff") {
        let percorsi: Vec<String> = o.paths_list().map(String::from).collect();
        percorsi.iter().take(64).for_each(|p| drop(o.read(p)));
        let _ = o.sha1_hex();
    }
}

/// The XML tree of the method files and the methods read from a container.
pub fn xmltree_methodinfo(data: &[u8]) {
    if let Ok(t) = std::str::from_utf8(data) {
        let _ = xmltree::parse(t);
    }
    let _ = methodinfo::read_methods(data.to_vec(), "x.dam");
}

/// The header reader of an mzML, over the bytes as they are.
pub fn mzmlhead(data: &[u8]) {
    let mut m = Mzml::new(data.to_vec(), data.len() as u64);
    let _ = m.info_head();
    let _ = m.run_head(0);
    let _ = m.header_polarity();
    let _ = m.nce();
    let _ = m.metadata();
    let _ = m.analyzers();
    if let Ok(v) = m.chromatograms(0) {
        for c in v.iter().take(4) {
            let _ = m.chromatogram(c);
            let _ = m.chromatogram_unit(c);
        }
    }
}

/// The `.mzidx` index: sections and their arrays.
pub fn idxread(data: &[u8]) {
    let Ok(mut ix) = Index::open(data.to_vec(), data.len() as u64) else {
        return;
    };
    for si in 0..ix.sections.len().min(8) {
        let _ = ix.arrays(si);
        let _ = ix.mz_range(si);
        let _ = ix.search(si, 100.0, false);
        let _ = ix.sum_window(si, 0, 50);
        let _ = ix.max_window(si, 0, 50);
    }
}

/// The Thermo `.raw` reader (opentfraw): index, method and streamed spectra.
pub fn raw(data: &[u8]) {
    let _ = raw::instrument_method_text(Cursor::new(data));
    let _ = raw::metodo(Cursor::new(data), "x.raw");
    let _ = raw::parse(Cursor::new(data), |_| {});
}

/// A `.wiff` + `.wiff.scan` pair: the input is a u32 LE length, that many bytes of `.wiff`, the rest is the `.wiff.scan`.
pub fn wiff(data: &[u8]) {
    let (Some(n), Some(corpo)) = (data.get(..4), data.get(4..)) else {
        return;
    };
    let n = (u32::from_le_bytes([n[0], n[1], n[2], n[3]]) as usize).min(corpo.len());
    let (w, scan) = corpo.split_at(n);
    if let Ok(o) = wiff::container(w.to_vec(), "x.wiff") {
        for s in wiff::samples(&o).into_iter().take(2) {
            let _ = wiff::Wiff::open(&o, "x.wiff", Some(scan.len() as u64), s);
        }
    }
    let membri = vec![("x.wiff", w.to_vec()), ("x.wiff.scan", scan.to_vec())];
    let _ = lettori::riconosci(&mut insieme(membri.clone()));
    let Ok(mut l) = lettori::apri(insieme(membri), &Limiti::sito()) else {
        return;
    };
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
        if let Some(v) = cr.first() {
            let _ = l.cromatogramma(0, &v.id);
        }
    }
}

/// The spectral library parsers (MSP, MGF, MassBank, JSON); the first byte picks the format, the rest is cut in two chunks by the second.
pub fn mzlib_msp_mgf(data: &[u8]) {
    let Some((&f, corpo)) = data.split_first() else {
        return;
    };
    let fmt = match f % 4 {
        0 => Format::Msp,
        1 => Format::Mgf,
        2 => Format::MassBank,
        _ => Format::Json,
    };
    let _ = parse_all(fmt, corpo);
}
