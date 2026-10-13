//! The mzML reader against the files that XML parsers get wrong: false offsets of the index, a truncated base64, an `defaultArrayLength` at the maximum,
//! tags that never close, a zlib array that inflates to hundreds of megabytes, millions of `cvParam` in one spectrum. Each case gives a key `err.*` or a
//! reading, within a ceiling of memory that does not grow with what the file declares (`tests/robustezza/mod.rs`).

mod robustezza;

use std::io::{Read, Write};

use base64::Engine;
use mzlab_core::mzml;
use robustezza::*;

#[global_allocator]
static ALLOCATORE: Contatore = Contatore;

fn b64(b: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(b)
}

fn f64s(v: &[f64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn array(accessione: &str, nome: &str, zlib: bool, testo: &str) -> String {
    let compressione = if zlib {
        r#"<cvParam accession="MS:1000574" name="zlib compression"/>"#
    } else {
        r#"<cvParam accession="MS:1000576" name="no compression"/>"#
    };
    format!(
        r#"<binaryDataArray><cvParam accession="MS:1000523" name="64-bit float"/>{compressione}<cvParam accession="{accessione}" name="{nome}"/><binary>{testo}</binary></binaryDataArray>"#
    )
}

/// A one-spectrum mzML with the two arrays given as text of base64.
fn mzml_con(lunghezza: &str, mz: &str, inten: &str, zlib: bool) -> String {
    let a = array("MS:1000514", "m/z array", zlib, mz);
    let b = array("MS:1000515", "intensity array", zlib, inten);
    format!(
        r#"<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml"><run id="r"><spectrumList count="1"><spectrum index="0" id="scan=1" defaultArrayLength="{lunghezza}"><cvParam accession="MS:1000511" name="ms level" value="1"/><binaryDataArrayList count="2">{a}{b}</binaryDataArrayList></spectrum></spectrumList></run></mzML>"#
    )
}

fn valido() -> String {
    let (mz, inten) = (
        b64(&f64s(&[100.0, 200.0, 300.0])),
        b64(&f64s(&[1.0, 2.0, 3.0])),
    );
    mzml_con("3", &mz, &inten, false)
}

fn zlib(b: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    z.write_all(b).unwrap();
    z.finish().unwrap()
}

/// `testa`, then `pezzo` repeated `n` times, then `coda`: a file of hundreds of megabytes that is never in memory.
struct Ripeti {
    parti: Vec<(Vec<u8>, usize)>,
    parte: usize,
    pos: usize,
}

impl Ripeti {
    fn nuovo(testa: &str, pezzo: &str, n: usize, coda: &str) -> Ripeti {
        Ripeti {
            parti: vec![(testa.into(), 1), (pezzo.into(), n), (coda.into(), 1)],
            parte: 0,
            pos: 0,
        }
    }
}

impl Read for Ripeti {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut scritti = 0;
        while scritti < buf.len() && self.parte < self.parti.len() {
            let (p, n) = &self.parti[self.parte];
            let totale = p.len() * n;
            if self.pos >= totale || p.is_empty() {
                self.parte += 1;
                self.pos = 0;
                continue;
            }
            let k = (buf.len() - scritti).min(p.len() - self.pos % p.len());
            buf[scritti..scritti + k].copy_from_slice(&p[self.pos % p.len()..][..k]);
            scritti += k;
            self.pos += k;
        }
        Ok(scritti)
    }
}

fn legge(testo: String) -> Vec<String> {
    let mut k = Vec::new();
    nota(mzml::read(testo.as_bytes()), &mut k);
    k
}

fn caso(nome: &str, testo: String) -> Vec<String> {
    let totale = testo.len();
    uno(nome, totale, Box::new(move || legge(testo)))
}

fn verde(guasti: Vec<String>) {
    verifica("mzml", 1, &guasti);
}

#[test]
fn the_valid_mzml_reads() {
    let mut k = Vec::new();
    let run = nota(mzml::read(valido().as_bytes()), &mut k).unwrap();
    assert!(k.is_empty() && run.spectra.len() == 1 && run.spectra[0].mz.len() == 3);
}

#[test]
fn false_offsets_of_the_index() {
    let v = valido();
    let indice = r#"<indexList count="1"><index name="spectrum"><offset idRef="scan=1">99999999999999</offset></index></indexList><indexListOffset>18446744073709551615</indexListOffset><fileChecksum>0</fileChecksum>"#;
    verde(caso(
        "indexed",
        format!(r#"<indexedmzML>{v}{indice}</indexedmzML>"#),
    ));
}

#[test]
fn a_truncated_or_broken_base64() {
    let mz = b64(&f64s(&[100.0, 200.0, 300.0]));
    let mut g = Vec::new();
    for (nome, t) in [
        ("cut", &mz[..mz.len() - 3]),
        ("one char", "A"),
        ("not base64", "@@@@!!!!"),
        ("padding", "===="),
    ] {
        g.extend(caso(nome, mzml_con("3", t, t, false)));
    }
    verde(g);
}

#[test]
#[ignore = "bug 1: a zlib array reserves the memory its declared length asks for (34 GB), without comparing it with the file"]
fn the_declared_length_at_the_maximum() {
    let (mz, inten) = (b64(&f64s(&[100.0])), b64(&f64s(&[1.0])));
    let comp = |b: &[f64]| b64(&zlib(&f64s(b)));
    let mut g = Vec::new();
    for l in ["4294967295", "18446744073709551615", "-1", "1e300"] {
        g.extend(caso(&format!("raw {l}"), mzml_con(l, &mz, &inten, false)));
        g.extend(caso(
            &format!("zlib {l}"),
            mzml_con(l, &comp(&[100.0]), &comp(&[1.0]), true),
        ));
    }
    verde(g);
}

#[test]
fn tags_that_never_close() {
    let v = valido();
    let mut g = Vec::new();
    for (nome, t) in [
        (
            "cut in the binary",
            v[..v.find("<binary>").unwrap() + 12].to_string(),
        ),
        (
            "only opened",
            "<mzML><run><spectrumList><spectrum><binaryDataArrayList><binaryDataArray><binary>AAAA"
                .repeat(1000),
        ),
        ("deep", "<a>".repeat(200_000)),
        (
            "unclosed attribute",
            r#"<mzML><spectrum id="scan=1 defaultArrayLength="3"#.to_string(),
        ),
    ] {
        g.extend(caso(nome, t));
    }
    verde(g);
}

#[test]
#[ignore = "bug 2: a zlib array is inflated without a limit (a 2 MB mzML makes 200 MB)"]
fn a_zlib_array_that_inflates_to_hundreds_of_megabytes() {
    let bomba = b64(&zlib(&vec![0u8; 200 << 20]));
    verde(caso("zlib bomb", mzml_con("3", &bomba, &bomba, true)));
}

#[test]
fn millions_of_cvparams_in_one_spectrum() {
    let testa = r#"<mzML><run><spectrumList count="1"><spectrum index="0" id="scan=1" defaultArrayLength="0">"#;
    let r = Ripeti::nuovo(
        testa,
        r#"<cvParam accession="MS:1000511" name="ms level" value="1"/>"#,
        2_000_000,
        "</spectrum></spectrumList></run></mzML>",
    );
    let g = uno(
        "2M cvParam",
        0,
        Box::new(move || {
            let mut k = Vec::new();
            nota(mzml::read(r), &mut k);
            k
        }),
    );
    verde(g);
}
