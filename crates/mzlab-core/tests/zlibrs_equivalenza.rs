//! zlib-rs behind flate2 reads what any zlib wrote and its own streams give back the same f64 bit for bit; a damaged stream keeps the error key.

use std::io::Write;

use base64::Engine;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use mzlab_core::mzmlhead::Mzml;

const VALUES: [f64; 7] = [
    0.0,
    1.0,
    -2.5,
    std::f64::consts::PI,
    1e-300,
    123456.789,
    f64::INFINITY,
];
/// `VALUES` as little-endian f64 compressed by CPython's zlib at level 6 (a writer other than the one in the build).
const FOREIGN: &str = "eJxjYEAGH+whNMsBCV2XkN+KnA6Rn38cks9byniybN98HpV/DlB19QAbTg3o";

fn file(b64: &str, n: usize) -> Vec<u8> {
    format!(
        "<mzML><run><chromatogramList count=\"1\"><chromatogram index=\"0\" id=\"TIC\" defaultArrayLength=\"{n}\">\
<binaryDataArrayList count=\"2\">\
<binaryDataArray encodedLength=\"0\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"MS:1000574\" name=\"zlib compression\"/><cvParam accession=\"MS:1000595\" name=\"time array\"/><binary>{b64}</binary></binaryDataArray>\
<binaryDataArray encodedLength=\"0\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"MS:1000574\" name=\"zlib compression\"/><cvParam accession=\"MS:1000515\" name=\"intensity array\"/><binary>{b64}</binary></binaryDataArray>\
</binaryDataArrayList></chromatogram></chromatogramList></run></mzML>"
    )
    .into_bytes()
}

fn read(bytes: Vec<u8>) -> mzlab_core::error::Result<Vec<f64>> {
    let size = bytes.len() as u64;
    let mut m = Mzml::new(bytes, size);
    let chroms = m.chromatograms(0)?;
    m.chromatogram(&chroms[0]).map(|(t, _)| t)
}

fn bytes(v: &[f64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

#[test]
fn a_stream_written_by_another_zlib_decodes_to_the_same_bits() {
    let got = read(file(FOREIGN, VALUES.len())).unwrap();
    let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&got), bits(&VALUES));
}

#[test]
fn our_streams_round_trip_bit_for_bit_at_every_level() {
    let mut x = 987654321u64;
    let v: Vec<f64> = (0..5000)
        .map(|_| {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (x >> 11) as f64 * 1e-3
        })
        .collect();
    for level in 0..=9 {
        let mut e = ZlibEncoder::new(Vec::new(), Compression::new(level));
        e.write_all(&bytes(&v)).unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(e.finish().unwrap());
        let got = read(file(&b64, v.len())).unwrap();
        assert!(
            got.iter().zip(&v).all(|(a, b)| a.to_bits() == b.to_bits()) && got.len() == v.len(),
            "level {level}"
        );
    }
}

#[test]
fn a_truncated_or_corrupt_stream_is_the_zlib_error() {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(FOREIGN)
        .unwrap();
    let cut = base64::engine::general_purpose::STANDARD.encode(&raw[..raw.len() - 6]);
    let mut bad = raw.clone();
    bad[2] ^= 0xff;
    let bad = base64::engine::general_purpose::STANDARD.encode(&bad);
    for b64 in [cut, bad] {
        let e = read(file(&b64, VALUES.len())).unwrap_err();
        assert_eq!(e.key, "err.file.zlib");
    }
}
