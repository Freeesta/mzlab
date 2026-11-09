//! Damaged input never panics: a `.mzlib` truncated at every length or with any byte changed gives an error or a readable file,
//! and the text readers (MSP, MGF, MassBank, JSON) survive being cut at every byte.
mod common;
use common::*;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::parse::{parse_all, Format};
use mzlab_lib::read::{Library, CACHE_BYTES};

fn open_and_read(bytes: &[u8]) {
    let Ok(lib) = Library::open(bytes, CACHE_BYTES) else {
        return;
    };
    let _ = lib.verify();
    for i in 0..lib.n_spectra().min(64) {
        let _ = lib.peaks_q(i);
        let _ = lib.meta(i);
    }
    let _ = lib.range_for_pol(2, 50.0, 400.0);
    let _ = lib.for_each_entry(false, 0, u32::MAX, |_, _, _| {});
}

#[test]
fn mzlib_truncated_at_every_length() {
    let (file, _) = build_in_memory(&synth_set(5, 20, 8), BuildOpts::default()).unwrap();
    for n in 0..file.len() {
        open_and_read(&file[..n]);
    }
}

#[test]
fn mzlib_with_any_byte_changed() {
    let (file, _) = build_in_memory(&synth_set(6, 20, 8), BuildOpts::default()).unwrap();
    let mut f = file.clone();
    for i in 0..file.len() {
        f[i] = file[i] ^ 0xFF;
        open_and_read(&f);
        f[i] = file[i];
    }
}

const MSP: &str = "Name: A\nPrecursorMZ: 123.4\nIonMode: P\nNum Peaks: 3\n50.1 10\n60.25 20.5e1\n.5 +3\n\nName: B\nPrecursorMZ: 99\nNum Peaks: 2\n1e 5\n-.e+ 7\n";
const MGF: &str = "BEGIN IONS\nPEPMASS=300.1 1e5\nCHARGE=1+\n50.1 10\n60.2 5\nEND IONS\n";
const MASSBANK: &str = "ACCESSION: X1\nCH$NAME: n\nMS$FOCUSED_ION: PRECURSOR_M/Z 300.1\nPK$PEAK: m/z int. rel.int.\n  50.1 10 5\n  60.2 20 9\n//\n";
const JSON: &str = r#"[{"name":"a","precursorMz":200.5,"peaks":[[50.1,10],[60.2,5]],"tags":["x",1]},{"name":[],"peaks":"1:2 3:4"}]"#;

#[test]
fn text_formats_cut_at_every_byte() {
    for (fmt, text) in [
        (Format::Msp, MSP),
        (Format::Mgf, MGF),
        (Format::MassBank, MASSBANK),
        (Format::Json, JSON),
    ] {
        for n in 0..=text.len() {
            let _ = parse_all(fmt, &text.as_bytes()[..n]);
        }
        let (full, _) = parse_all(fmt, text.as_bytes());
        assert!(!full.is_empty(), "{fmt:?}");
    }
}
