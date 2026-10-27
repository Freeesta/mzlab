//! A chromatogram array that declares few points but inflates to far more stops with a clean error (no zlib bomb), and an honest one still reads.

use std::io::Write;

use base64::Engine;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use mzlab_core::mzmlhead::Mzml;

fn zlib_b64(raw: &[u8]) -> String {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::best());
    e.write_all(raw).unwrap();
    base64::engine::general_purpose::STANDARD.encode(e.finish().unwrap())
}

fn file(declared: usize, time: &str, inten: &str) -> Vec<u8> {
    format!(
        "<mzML><run><chromatogramList count=\"1\"><chromatogram index=\"0\" id=\"TIC\" defaultArrayLength=\"{declared}\">\
<binaryDataArrayList count=\"2\">\
<binaryDataArray encodedLength=\"0\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"MS:1000574\" name=\"zlib compression\"/><cvParam accession=\"MS:1000595\" name=\"time array\"/><binary>{time}</binary></binaryDataArray>\
<binaryDataArray encodedLength=\"0\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"MS:1000574\" name=\"zlib compression\"/><cvParam accession=\"MS:1000515\" name=\"intensity array\"/><binary>{inten}</binary></binaryDataArray>\
</binaryDataArrayList></chromatogram></chromatogramList></run></mzML>"
    )
    .into_bytes()
}

fn read(bytes: Vec<u8>) -> mzlab_core::error::Result<(Vec<f64>, Vec<f64>)> {
    let size = bytes.len() as u64;
    let mut m = Mzml::new(bytes, size);
    let chroms = m.chromatograms(0)?;
    m.chromatogram(&chroms[0])
}

#[test]
fn honest_array_reads() {
    let v: Vec<u8> = [1.0f64, 2.0, 3.0].iter().flat_map(|x| x.to_le_bytes()).collect();
    let (t, y) = read(file(3, &zlib_b64(&v), &zlib_b64(&v))).unwrap();
    assert_eq!((t, y), (vec![1.0, 2.0, 3.0], vec![1.0, 2.0, 3.0]));
}

#[test]
fn bomb_is_an_error_not_an_oom() {
    // 200 MB of zeros compress to ~200 KB; the array declares 3 points
    let bomb = zlib_b64(&vec![0u8; 200 << 20]);
    let ok: Vec<u8> = [1.0f64, 2.0, 3.0].iter().flat_map(|x| x.to_le_bytes()).collect();
    let e = read(file(3, &bomb, &zlib_b64(&ok))).unwrap_err();
    assert_eq!(e.key, "error.memoryBudget");
}
