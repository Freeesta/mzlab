//! The scan types that are not a plain full or product-ion scan (precursor ion, constant neutral loss/gain, EPI) are read from the CV term of
//! the spectrum or from the Thermo filter, written in the index as a 21st column, and shown as `acq` and as parts of a mixed file. Files built here.

use mzlab_core::idxread::{Index, Scan};
use mzlab_core::iteminfo::{facts_from, file_parts, item_info, part_label, Meta};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzmlhead::Mzml;
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
            if i <= c.len() {
                s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

/// One spectrum: `extra` is the cvParam of its type (or empty), `filter` the Thermo filter string (or empty).
fn spectrum(i: usize, level: u8, rt: f64, extra: &str, filter: &str, prec: Option<f64>) -> String {
    let filt = if filter.is_empty() {
        String::new()
    } else {
        format!("<cvParam cvRef=\"MS\" accession=\"MS:1000512\" name=\"filter string\" value=\"{filter}\"/>")
    };
    let pre = prec.map_or(String::new(), |p| {
        format!("<precursorList count=\"1\"><precursor><isolationWindow><cvParam cvRef=\"MS\" accession=\"MS:1000827\" name=\"isolation window target m/z\" value=\"{p}\"/><cvParam cvRef=\"MS\" accession=\"MS:1000828\" name=\"isolation window lower offset\" value=\"0.5\"/><cvParam cvRef=\"MS\" accession=\"MS:1000829\" name=\"isolation window upper offset\" value=\"0.5\"/></isolationWindow><selectedIonList count=\"1\"><selectedIon><cvParam cvRef=\"MS\" accession=\"MS:1000744\" name=\"selected ion m/z\" value=\"{p}\"/></selectedIon></selectedIonList><activation><cvParam cvRef=\"MS\" accession=\"MS:1000133\" name=\"collision-induced dissociation\"/><cvParam cvRef=\"MS\" accession=\"MS:1000045\" name=\"collision energy\" value=\"25\"/></activation></precursor></precursorList>")
    });
    format!(
        "<spectrum index=\"{i}\" id=\"scan={}\" defaultArrayLength=\"2\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"{level}\"/><cvParam cvRef=\"MS\" accession=\"MS:1000130\" name=\"positive scan\"/><cvParam cvRef=\"MS\" accession=\"MS:1000127\" name=\"centroid spectrum\"/>{extra}<cvParam cvRef=\"MS\" accession=\"MS:1000285\" name=\"total ion current\" value=\"30\"/><scanList count=\"1\"><scan>{filt}<cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{rt}\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/></scan></scanList>{pre}<binaryDataArrayList count=\"2\"><binaryDataArray encodedLength=\"24\"><cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam cvRef=\"MS\" accession=\"MS:1000576\" name=\"no compression\"/><cvParam cvRef=\"MS\" accession=\"MS:1000514\" name=\"m/z array\"/><binary>{}</binary></binaryDataArray><binaryDataArray encodedLength=\"24\"><cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam cvRef=\"MS\" accession=\"MS:1000576\" name=\"no compression\"/><cvParam cvRef=\"MS\" accession=\"MS:1000515\" name=\"intensity array\"/><binary>{}</binary></binaryDataArray></binaryDataArrayList></spectrum>",
        i + 1,
        b64(&[100.0, 200.0]),
        b64(&[10.0, 20.0])
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

fn index_of(bytes: Vec<u8>) -> (Index<Vec<u8>>, Vec<u8>) {
    let mut src = bytes.clone();
    let p = prepare(&mut src, bytes.len() as u64, "t.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &bytes, 64 << 20).unwrap();
    let len = idx.len() as u64;
    (Index::open(idx, len).unwrap(), bytes)
}

fn kinds(scans: &[Scan]) -> Vec<Option<&str>> {
    scans.iter().map(|s| s.kind_name()).collect()
}

fn acq_of(bytes: Vec<u8>) -> String {
    let (mut idx, raw) = index_of(bytes);
    let end = idx.scans.last().map_or(0, |s| s.end);
    let size = raw.len() as u64;
    let mut mz = Mzml::new(raw, size);
    let facts = facts_from(&mut mz, end, 0, false).unwrap();
    let meta = Meta {
        file: "t.mzML".into(),
        label: "t".into(),
        ..Meta::default()
    };
    let info = item_info(&mut idx, &facts, &meta, None).unwrap();
    let a = info.find("\"acq\":[").unwrap() + 7;
    info[a..a + info[a..].find(']').unwrap()].to_string()
}

const PREC: &str =
    "<cvParam cvRef=\"MS\" accession=\"MS:1000341\" name=\"precursor ion spectrum\"/>";
const NL: &str =
    "<cvParam cvRef=\"MS\" accession=\"MS:1000326\" name=\"constant neutral loss spectrum\"/>";
const NG: &str =
    "<cvParam cvRef=\"MS\" accession=\"MS:1000325\" name=\"constant neutral gain spectrum\"/>";
const EPI: &str =
    "<cvParam cvRef=\"MS\" accession=\"MS:1000789\" name=\"enhanced multiply charged spectrum\"/>";

#[test]
fn each_type_is_read_from_the_term() {
    let sp = vec![
        spectrum(0, 2, 1.0, PREC, "", Some(150.0)),
        spectrum(1, 2, 2.0, NL, "", Some(150.0)),
        spectrum(2, 2, 3.0, NG, "", Some(150.0)),
        spectrum(3, 2, 4.0, EPI, "", Some(150.0)),
        spectrum(4, 2, 5.0, "", "", Some(150.0)),
    ];
    let (idx, _) = index_of(mzml(&sp));
    assert_eq!(
        kinds(&idx.scans),
        vec![Some("PREC"), Some("NL"), Some("NG"), Some("EPI"), None]
    );
}

#[test]
fn thermo_filter_tokens_give_the_type() {
    let sp = vec![
        spectrum(
            0,
            2,
            1.0,
            "",
            "ITMS + c ESI pr Full ms2 150.00@cid35.00",
            Some(150.0),
        ),
        spectrum(
            1,
            2,
            2.0,
            "",
            "TQMS + c ESI cnl Full ms2 150.00@cid35.00",
            Some(150.0),
        ),
        spectrum(
            2,
            2,
            3.0,
            "",
            "ITMS + c ESI Full ms2 150.00@cid35.00 [50.00-200.00]",
            Some(150.0),
        ),
    ];
    let (idx, _) = index_of(mzml(&sp));
    assert_eq!(kinds(&idx.scans), vec![Some("PREC"), Some("NL"), None]);
}

#[test]
fn acq_lists_the_types_over_ms2() {
    for (term, want) in [(PREC, "PREC"), (NL, "NL"), (NG, "NG"), (EPI, "EPI")] {
        let sp: Vec<String> = (0..4)
            .map(|i| spectrum(i, 2, 1.0 + i as f64, term, "", Some(150.0 + i as f64)))
            .collect();
        assert_eq!(acq_of(mzml(&sp)), format!("\"{want}\""), "{want}");
    }
    // a file without any type is classified as before
    let sp: Vec<String> = (0..4)
        .map(|i| spectrum(i, 2, 1.0 + i as f64, "", "", Some(150.0 + i as f64)))
        .collect();
    assert_eq!(acq_of(mzml(&sp)), "\"MS2\"");
}

#[test]
fn a_mixed_file_splits_by_type() {
    let sp = vec![
        spectrum(0, 1, 1.0, "", "", None),
        spectrum(1, 2, 1.1, "", "", Some(150.0)),
        spectrum(2, 2, 1.2, PREC, "", Some(150.0)),
    ];
    let (idx, _) = index_of(mzml(&sp));
    let parts = file_parts(&idx.scans, false);
    let tags: Vec<&str> = parts.iter().map(|p| p.tag.as_str()).collect();
    assert_eq!(tags, vec!["MS1", "MS2", "PREC"]);
    let names: Vec<String> = parts.iter().map(|p| part_label(p, &idx.scans)).collect();
    assert_eq!(names, vec!["Full Scan", "DDA", "Precursor ion"]);
    assert!(mzlab_core::iteminfo::matches(
        &idx.scans[2],
        Some(&parts[2])
    ));
    assert!(!mzlab_core::iteminfo::matches(
        &idx.scans[1],
        Some(&parts[2])
    ));
    assert!(!mzlab_core::iteminfo::matches(
        &idx.scans[2],
        Some(&parts[1])
    ));
}

#[test]
fn an_index_without_types_keeps_20_columns() {
    let sp = vec![
        spectrum(0, 1, 1.0, "", "", None),
        spectrum(1, 2, 1.1, "", "", Some(150.0)),
    ];
    let bytes = mzml(&sp);
    let mut src = bytes.clone();
    let p = prepare(&mut src, bytes.len() as u64, "t.mzML").unwrap();
    let rows = String::from_utf8(p.table.scans_json.clone()).unwrap();
    assert!(!rows.contains("\"PREC\"") && rows.matches("\"scan=").count() == 2);
    let v: serde_json::Value = serde_json::from_str(&rows).unwrap();
    assert!(v
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r.as_array().unwrap().len() == 20));
}
