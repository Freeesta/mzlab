//! The mzML reader against the files that XML parsers get wrong: false offsets of the index, a truncated base64, an `defaultArrayLength` at the maximum,
//! tags that never close, a zlib array that inflates to hundreds of megabytes, millions of `cvParam` in one spectrum. Each case gives a key `err.*` or a
//! reading, within a ceiling of memory that does not grow with what the file declares (`tests/robustness/mod.rs`).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod robustness;

use std::io::{Read, Write};

use base64::Engine;
use mzlab_core::mzml;
use robustness::*;

#[global_allocator]
static ALLOCATORE: Counter = Counter;

fn b64(b: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(b)
}

fn f64s(v: &[f64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn array(accession: &str, name: &str, zlib: bool, text: &str) -> String {
    let compression = if zlib {
        r#"<cvParam accession="MS:1000574" name="zlib compression"/>"#
    } else {
        r#"<cvParam accession="MS:1000576" name="no compression"/>"#
    };
    format!(
        r#"<binaryDataArray><cvParam accession="MS:1000523" name="64-bit float"/>{compression}<cvParam accession="{accession}" name="{name}"/><binary>{text}</binary></binaryDataArray>"#
    )
}

/// A one-spectrum mzML with the two arrays given as text of base64.
fn mzml_with(length: &str, mz: &str, inten: &str, zlib: bool) -> String {
    let a = array("MS:1000514", "m/z array", zlib, mz);
    let b = array("MS:1000515", "intensity array", zlib, inten);
    format!(
        r#"<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml"><run id="r"><spectrumList count="1"><spectrum index="0" id="scan=1" defaultArrayLength="{length}"><cvParam accession="MS:1000511" name="ms level" value="1"/><binaryDataArrayList count="2">{a}{b}</binaryDataArrayList></spectrum></spectrumList></run></mzML>"#
    )
}

fn valid() -> String {
    let (mz, inten) = (
        b64(&f64s(&[100.0, 200.0, 300.0])),
        b64(&f64s(&[1.0, 2.0, 3.0])),
    );
    mzml_with("3", &mz, &inten, false)
}

fn zlib(b: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    z.write_all(b).unwrap();
    z.finish().unwrap()
}

/// `testa`, then `pezzo` repeated `n` times, then `coda`: a file of hundreds of megabytes that is never in memory.
struct Repeat {
    parts: Vec<(Vec<u8>, usize)>,
    part: usize,
    pos: usize,
}

impl Repeat {
    fn nuovo(testa: &str, piece: &str, n: usize, tail: &str) -> Repeat {
        Repeat {
            parts: vec![(testa.into(), 1), (piece.into(), n), (tail.into(), 1)],
            part: 0,
            pos: 0,
        }
    }
}

impl Read for Repeat {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut scritti = 0;
        while scritti < buf.len() && self.part < self.parts.len() {
            let (p, n) = &self.parts[self.part];
            let total = p.len() * n;
            if self.pos >= total || p.is_empty() {
                self.part += 1;
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

fn reads(text: String) -> Vec<String> {
    let mut k = Vec::new();
    note(mzml::read(text.as_bytes()), &mut k);
    k
}

fn case(name: &str, text: String) -> Vec<String> {
    let total = text.len();
    one(name, total, Box::new(move || reads(text)))
}

fn verde(guasti: Vec<String>) {
    verify("mzml", 1, &guasti);
}

#[test]
fn the_valid_mzml_reads() {
    let mut k = Vec::new();
    let run = note(mzml::read(valid().as_bytes()), &mut k).unwrap();
    assert!(k.is_empty() && run.spectra.len() == 1 && run.spectra[0].mz.len() == 3);
}

#[test]
fn false_offsets_of_the_index() {
    let v = valid();
    let index = r#"<indexList count="1"><index name="spectrum"><offset idRef="scan=1">99999999999999</offset></index></indexList><indexListOffset>18446744073709551615</indexListOffset><fileChecksum>0</fileChecksum>"#;
    verde(case(
        "indexed",
        format!(r#"<indexedmzML>{v}{index}</indexedmzML>"#),
    ));
}

#[test]
fn a_truncated_or_broken_base64() {
    let mz = b64(&f64s(&[100.0, 200.0, 300.0]));
    let mut g = Vec::new();
    for (name, t) in [
        ("cut", &mz[..mz.len() - 3]),
        ("one char", "A"),
        ("not base64", "@@@@!!!!"),
        ("padding", "===="),
    ] {
        g.extend(case(name, mzml_with("3", t, t, false)));
    }
    verde(g);
}

#[test]
fn the_declared_length_at_the_maximum() {
    let (mz, inten) = (b64(&f64s(&[100.0])), b64(&f64s(&[1.0])));
    let comp = |b: &[f64]| b64(&zlib(&f64s(b)));
    let mut g = Vec::new();
    for l in ["4294967295", "18446744073709551615", "-1", "1e300"] {
        g.extend(case(&format!("raw {l}"), mzml_with(l, &mz, &inten, false)));
        g.extend(case(
            &format!("zlib {l}"),
            mzml_with(l, &comp(&[100.0]), &comp(&[1.0]), true),
        ));
    }
    verde(g);
}

#[test]
fn tags_that_never_close() {
    let v = valid();
    let mut g = Vec::new();
    for (name, t) in [
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
        g.extend(case(name, t));
    }
    verde(g);
}

#[test]
fn a_zlib_array_that_inflates_to_hundreds_of_megabytes() {
    let bomba = b64(&zlib(&vec![0u8; 200 << 20]));
    verde(case("zlib bomb", mzml_with("3", &bomba, &bomba, true)));
}

/// A spectrum that declares no points (`defaultArrayLength` 0 or missing) and holds a zlib array of 60 MB: the inflate limit follows the bytes of the file and
/// the refusal is `error.fixedLimit` (the size, not damage, and not the memory the pool retries). A truly corrupt stream keeps `err.file.zlib`.
#[test]
fn a_zlib_array_of_a_spectrum_that_declares_no_points() {
    let bomb = b64(&zlib(&vec![0u8; 60 << 20]));
    let mut g = Vec::new();
    for l in ["0", ""] {
        let text = mzml_with(l, &bomb, &bomb, true).replace(" defaultArrayLength=\"\"", "");
        assert_eq!(reads(text.clone()), ["error.fixedLimit"], "length {l:?}");
        g.extend(case(&format!("length {l:?}"), text));
    }
    let mut broken = zlib(&vec![7u8; 64 << 10]);
    broken.truncate(broken.len() / 2);
    let text = mzml_with("0", &b64(&broken), &b64(&broken), true);
    assert_eq!(reads(text), ["err.file.zlib"]);
    verde(g);
}

#[test]
fn millions_of_cvparams_in_one_spectrum() {
    let testa = r#"<mzML><run><spectrumList count="1"><spectrum index="0" id="scan=1" defaultArrayLength="0">"#;
    let r = Repeat::nuovo(
        testa,
        r#"<cvParam accession="MS:1000511" name="ms level" value="1"/>"#,
        2_000_000,
        "</spectrum></spectrumList></run></mzML>",
    );
    let g = one(
        "2M cvParam",
        0,
        Box::new(move || {
            let mut k = Vec::new();
            note(mzml::read(r), &mut k);
            k
        }),
    );
    verde(g);
}
