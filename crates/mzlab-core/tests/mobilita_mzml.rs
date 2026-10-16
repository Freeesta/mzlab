//! Ion mobility in an mzML: the arrays (`MS:1003006`, `MS:1002477`...) and the scan attributes (`MS:1002815`, `MS:1002476`) are read, shown as `IMS` in the
//! information of the file and as a column of the scan. Files built here; a file without mobility does not change.

use mzlab_core::idxread::Index;
use mzlab_core::iteminfo::{facts_from, item_info, Meta};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::{self, Item};
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::sorgente::Scansioni;
use mzlab_core::upload::prepare;

fn b64(vals: &[f64]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = vals.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut s = String::new();
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            s.push(if i <= c.len() {
                A[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    s
}

fn array(acc: &str, name: &str, vals: &[f64]) -> String {
    format!("<binaryDataArray encodedLength=\"{}\"><cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam cvRef=\"MS\" accession=\"MS:1000576\" name=\"no compression\"/><cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\"/><binary>{}</binary></binaryDataArray>", vals.len() * 12, b64(vals))
}

/// A spectrum with 3 peaks; `scan_cv` goes in its `<scan>`, `extra` after the m/z and intensity arrays.
fn spectrum(i: usize, rt: f64, scan_cv: &str, extra: &str) -> String {
    format!(
        "<spectrum index=\"{i}\" id=\"scan={}\" defaultArrayLength=\"3\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"1\"/><cvParam cvRef=\"MS\" accession=\"MS:1000130\" name=\"positive scan\"/><cvParam cvRef=\"MS\" accession=\"MS:1000127\" name=\"centroid spectrum\"/><cvParam cvRef=\"MS\" accession=\"MS:1000285\" name=\"total ion current\" value=\"30\"/><scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{rt}\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/>{scan_cv}</scan></scanList><binaryDataArrayList count=\"3\">{}{}{extra}</binaryDataArrayList></spectrum>",
        i + 1,
        array("MS:1000514", "m/z array", &[100.0, 200.0, 300.0]),
        array("MS:1000515", "intensity array", &[10.0, 20.0, 30.0]),
    )
}

fn mzml(spectra: &[String]) -> Vec<u8> {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<mzML xmlns=\"http://psi.hupo.org/ms/mzml\" version=\"1.1.0\"><cvList count=\"1\"><cv id=\"MS\" fullName=\"PSI-MS\" version=\"4.1.0\" URI=\"x\"/></cvList><run id=\"r\"><spectrumList count=\"{}\" defaultDataProcessingRef=\"d\">{}</spectrumList></run></mzML>",
        spectra.len(),
        spectra.concat()
    )
    .into_bytes()
}

fn k0(v: f64) -> String {
    format!("<cvParam cvRef=\"MS\" accession=\"MS:1002815\" name=\"inverse reduced ion mobility\" value=\"{v}\" unitCvRef=\"MS\" unitAccession=\"MS:1002814\" unitName=\"volt-second per square centimeter\"/>")
}

fn index_of(bytes: &[u8]) -> Index<Vec<u8>> {
    let mut src = bytes.to_vec();
    let p = prepare(&mut src, bytes.len() as u64, "t.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, bytes, 64 << 20).unwrap();
    let len = idx.len() as u64;
    Index::open(idx, len).unwrap()
}

fn info_of(bytes: Vec<u8>) -> String {
    let mut idx = index_of(&bytes);
    let end = idx.scans.last().map_or(0, |s| s.end);
    let size = bytes.len() as u64;
    let mut mz = Mzml::new(bytes, size);
    let facts = facts_from(&mut mz, end, 0, false).unwrap();
    let meta = Meta {
        file: "t.mzML".into(),
        label: "t".into(),
        ..Meta::default()
    };
    item_info(&mut idx, &facts, &meta, None).unwrap()
}

fn mobility_items(bytes: &[u8]) -> Vec<mzml::Mobility> {
    let mut out = vec![];
    mzml::parse(bytes, |it| {
        if let Item::Mobility(m) = it {
            out.push(m);
        }
    })
    .unwrap();
    out
}

#[test]
fn the_array_and_the_scan_attribute_are_parsed() {
    let sp = vec![
        spectrum(
            0,
            1.0,
            &k0(0.85),
            &array(
                "MS:1003006",
                "mean inverse reduced ion mobility array",
                &[0.8, 0.9, 1.0],
            ),
        ),
        spectrum(
            1,
            2.0,
            "",
            &array(
                "MS:1002477",
                "mean ion mobility drift time array",
                &[5.0, 6.0, 7.0],
            ),
        ),
        spectrum(2, 3.0, "", ""),
    ];
    let items = mobility_items(&mzml(&sp));
    assert_eq!(items.len(), 2, "a spectrum without mobility gives no item");
    assert_eq!(
        (
            items[0].index,
            items[0].unit,
            items[0].values.clone(),
            items[0].scan
        ),
        (0, "k0", Some(vec![0.8, 0.9, 1.0]), Some((0.85, "k0")))
    );
    assert_eq!(
        (
            items[1].index,
            items[1].unit,
            items[1].values.clone(),
            items[1].scan
        ),
        (1, "ms", Some(vec![5.0, 6.0, 7.0]), None)
    );
}

#[test]
fn an_array_of_another_length_is_dropped() {
    let bad = array(
        "MS:1003006",
        "mean inverse reduced ion mobility array",
        &[0.8, 0.9],
    );
    assert!(mobility_items(&mzml(&[spectrum(0, 1.0, "", &bad)])).is_empty());
    let items = mobility_items(&mzml(&[spectrum(0, 1.0, &k0(0.9), &bad)]));
    assert_eq!(
        (items.len(), items[0].values.clone(), items[0].scan),
        (1, None, Some((0.9, "k0")))
    );
}

#[test]
fn the_scan_attribute_gives_ims_and_a_range() {
    let sp = vec![
        spectrum(0, 1.0, &k0(0.8), ""),
        spectrum(1, 2.0, &k0(1.2), ""),
        spectrum(2, 3.0, "", ""),
    ];
    let info = info_of(mzml(&sp));
    assert!(info.contains("\"acq\":[\"MS1\",\"IMS\"]"), "{info}");
    assert!(
        info.contains("\"ims\":{\"unit\":\"k0\",\"min\":0.8,\"max\":1.2,\"n\":2}"),
        "{info}"
    );
    let idx = index_of(&mzml(&sp));
    assert_eq!(idx.scans[0].mob, Some((0.8, "k0".to_string())));
    assert_eq!(idx.scans[2].mob, None);
}

#[test]
fn the_array_alone_gives_ims_without_a_range() {
    let sp = vec![spectrum(
        0,
        1.0,
        "",
        &array(
            "MS:1003006",
            "mean inverse reduced ion mobility array",
            &[0.8, 0.9, 1.0],
        ),
    )];
    let info = info_of(mzml(&sp));
    assert!(
        info.contains("\"acq\":[\"MS1\",\"IMS\"]") && info.contains("\"ims\":{\"unit\":\"k0\"}"),
        "{info}"
    );
}

#[test]
fn a_file_without_mobility_is_unchanged() {
    let sp = vec![spectrum(0, 1.0, "", ""), spectrum(1, 2.0, "", "")];
    let bytes = mzml(&sp);
    let info = info_of(bytes.clone());
    assert!(
        info.contains("\"acq\":[\"MS1\"]") && !info.contains("\"ims\""),
        "{info}"
    );
    let mut src = bytes.clone();
    let p = prepare(&mut src, bytes.len() as u64, "t.mzML").unwrap();
    let rows: serde_json::Value = serde_json::from_slice(&p.table.scans_json).unwrap();
    assert!(rows
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r.as_array().unwrap().len() == 20));
}

#[test]
fn the_source_gives_the_mobility_of_a_scan() {
    let sp = vec![
        spectrum(
            0,
            1.0,
            &k0(0.85),
            &array(
                "MS:1003006",
                "mean inverse reduced ion mobility array",
                &[0.8, 0.9, 1.0],
            ),
        ),
        spectrum(1, 2.0, "", ""),
    ];
    let bytes = mzml(&sp);
    let idx = index_of(&bytes);
    let mut src = bytes;
    let m = src.mobilita(&idx.scans[0]).unwrap().unwrap();
    assert_eq!(
        (m.unit, m.values, m.scan),
        ("k0", Some(vec![0.8, 0.9, 1.0]), Some((0.85, "k0")))
    );
    assert!(src.mobilita(&idx.scans[1]).unwrap().is_none());
}
