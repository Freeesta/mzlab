//! The parent of an MS3 scan without a Thermo filter string is the scan of its immediate precursor, in whichever order the two
//! `<precursor>` elements are listed (the scan of the MS2 and the one of the MS1 that it came from).

use mzlab_core::scantable;
use serde_json::Value;

fn spectrum(i: usize, level: u8, precursors: &[(usize, f64)]) -> String {
    let list: String = precursors
        .iter()
        .map(|(scan, mz)| {
            format!(
                "<precursor spectrumRef=\"scan={scan}\"><selectedIonList count=\"1\"><selectedIon><cvParam cvRef=\"MS\" accession=\"MS:1000744\" name=\"selected ion m/z\" value=\"{mz}\"/></selectedIon></selectedIonList></precursor>"
            )
        })
        .collect();
    let list = if precursors.is_empty() {
        String::new()
    } else {
        format!(
            "<precursorList count=\"{}\">{list}</precursorList>",
            precursors.len()
        )
    };
    format!(
        "<spectrum index=\"{i}\" id=\"scan={}\" defaultArrayLength=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"{level}\"/>\
         <scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"minute\"/></scan></scanList>{list}\
         <binaryDataArrayList count=\"1\"><binaryDataArray><binary>QUFB</binary></binaryDataArray></binaryDataArrayList></spectrum>",
        i + 1,
        0.1 * i as f64
    )
}

/// The parent column of each row of the table of MS1, MS2 (from the MS1) and MS3 (listing both precursors in `order`).
fn parents(order: [(usize, f64); 2]) -> Vec<Option<u64>> {
    let spectra = [
        spectrum(0, 1, &[]),
        spectrum(1, 2, &[(1, 400.0)]),
        spectrum(2, 3, &order),
    ];
    let mut doc =
        String::from("<?xml version=\"1.0\"?><mzML><run id=\"r\"><spectrumList count=\"3\">");
    spectra.iter().for_each(|s| doc.push_str(s));
    doc.push_str("</spectrumList></run></mzML>");
    let bytes = doc.into_bytes();
    let size = bytes.len() as u64;
    let table = scantable::read(&mut bytes.clone(), size).unwrap();
    let rows: Value = serde_json::from_str(&table.rows_json()).unwrap();
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| r[10].as_u64())
        .collect()
}

#[test]
fn ms3_parent_is_the_ms2_whatever_the_order() {
    // MS2 listed first, then the MS1 it came from
    assert_eq!(parents([(2, 250.0), (1, 400.0)]), [None, Some(0), Some(1)]);
    // MS1 first, then the MS2
    assert_eq!(parents([(1, 400.0), (2, 250.0)]), [None, Some(0), Some(1)]);
}
