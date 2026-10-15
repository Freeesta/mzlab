#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
use crate::mzml::read;
use base64::Engine;
use std::io::Write;

fn mzml(zlib: &[u8]) -> String {
    let b = base64::engine::general_purpose::STANDARD.encode(zlib);
    let a = |acc: &str| {
        format!(
            r#"<binaryDataArray><cvParam accession="MS:1000523"/><cvParam accession="MS:1000574"/><cvParam accession="{acc}"/><binary>{b}</binary></binaryDataArray>"#
        )
    };
    format!(
        r#"<mzML><run><spectrumList count="1"><spectrum index="0" id="scan=1" defaultArrayLength="3"><binaryDataArrayList>{}{}</binaryDataArrayList></spectrum></spectrumList></run></mzML>"#,
        a("MS:1000514"),
        a("MS:1000515")
    )
}

fn deflate(raw: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(raw).unwrap();
    e.finish().unwrap()
}

fn key(z: &[u8]) -> (String, String) {
    let e = read(mzml(z).as_bytes()).unwrap_err();
    let detail = e
        .params
        .iter()
        .find(|(k, _)| *k == "detail")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    (e.key.to_string(), detail)
}

#[test]
fn zlib_errors_keep_their_keys_and_details() {
    let raw: Vec<u8> = [1.0f64, 2.0, 3.0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let good = deflate(&raw);
    let ok = read(mzml(&good).as_bytes()).unwrap();
    assert_eq!(ok.spectra[0].mz, vec![1.0, 2.0, 3.0]);
    let trailing = [good.clone(), b"xyz".to_vec()].concat();
    assert_eq!(
        read(mzml(&trailing).as_bytes()).unwrap().spectra[0]
            .mz
            .len(),
        3
    );
    let cut = &good[..good.len() - 6];
    assert_eq!(
        key(cut),
        ("err.file.zlib".into(), "incomplete deflate stream".into())
    );
    assert_eq!(key(&good[..5]).1, "incomplete deflate stream");
    assert_eq!(key(b"hello world 12345").1, "corrupt deflate stream");
    // 3 values declared, far more inflated: a bomb is a memory-budget error, not a zlib one
    let bomb = deflate(&vec![0u8; 8 << 20]);
    assert_eq!(key(&bomb), ("error.memoryBudget".into(), String::new()));
}
