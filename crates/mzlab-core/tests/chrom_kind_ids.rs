//! Multi-experiment chromatogram ids ("TIC sample=.. experiment=..", "BPC sample=.. experiment=..") keep their kind in both mzML readers.

use base64::Engine;
use mzlab_core::mzml::{self, ChromKind};
use mzlab_core::mzmlhead::Mzml;

fn b64(v: &[f64]) -> String {
    base64::engine::general_purpose::STANDARD
        .encode(v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>())
}

fn file() -> Vec<u8> {
    let t = b64(&[1.0, 2.0, 3.0]);
    let y = b64(&[4.0, 5.0, 6.0]);
    let mut s = String::from("<mzML><run id=\"r\"><chromatogramList count=\"4\">");
    for (i, id) in [
        "TIC sample=1 period=1 experiment=2",
        "BPC sample=1 period=1 experiment=2",
        "TIC",
        "BPC",
    ]
    .iter()
    .enumerate()
    {
        s += &format!(
            "<chromatogram index=\"{i}\" id=\"{id}\" defaultArrayLength=\"3\"><binaryDataArrayList count=\"2\">\
<binaryDataArray><cvParam accession=\"MS:1000523\"/><cvParam accession=\"MS:1000595\" unitName=\"minute\"/><binary>{t}</binary></binaryDataArray>\
<binaryDataArray><cvParam accession=\"MS:1000523\"/><cvParam accession=\"MS:1000515\"/><binary>{y}</binary></binaryDataArray>\
</binaryDataArrayList></chromatogram>"
        );
    }
    (s + "</chromatogramList></run></mzML>").into_bytes()
}

#[test]
fn full_reader_keeps_tic_and_bpc_kinds() {
    let run = mzml::read(&file()[..]).unwrap();
    let kinds: Vec<ChromKind> = run.chromatograms.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        [
            ChromKind::Tic,
            ChromKind::Bpc,
            ChromKind::Tic,
            ChromKind::Bpc
        ]
    );
}

#[test]
fn head_reader_keeps_tic_and_bpc_kinds() {
    let bytes = file();
    let size = bytes.len() as u64;
    let mut m = Mzml::new(bytes, size);
    let kinds: Vec<String> = m
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .map(|c| c.kind)
        .collect();
    assert_eq!(kinds, ["tic", "bpc", "tic", "bpc"]);
}
