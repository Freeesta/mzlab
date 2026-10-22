//! The scan table of an mzML without `indexList` (read by walking the file): every row spans exactly one `<spectrum ...</spectrum>`,
//! also when a spectrum is larger than the read-ahead buffer and when many small ones share it.

use mzlab_core::scantable;

fn spectrum(i: usize, payload: usize) -> String {
    format!(
        "<spectrum index=\"{i}\" id=\"scan={}\" defaultArrayLength=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"1\"/>\
         <scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"minute\"/></scan></scanList>\
         <binaryDataArrayList count=\"1\"><binaryDataArray><binary>{}</binary></binaryDataArray></binaryDataArrayList></spectrum>",
        i + 1,
        0.1 * i as f64,
        "A".repeat(payload)
    )
}

#[test]
fn rows_span_whole_spectra_without_an_index() {
    // small ones, then three larger than the 1 MiB read-ahead, then small again
    let sizes = [
        200usize, 300, 5000, 1_500_000, 400, 1_100_000, 700, 2_300_000, 150,
    ];
    let mut doc =
        String::from("<?xml version=\"1.0\"?><mzML><run id=\"r\"><spectrumList count=\"9\">");
    for (i, n) in sizes.iter().enumerate() {
        doc.push_str(&spectrum(i, *n));
    }
    doc.push_str("</spectrumList></run></mzML>");
    let bytes = doc.clone().into_bytes();
    let size = bytes.len() as u64;
    let table = scantable::read(&mut bytes.clone(), size).unwrap();
    assert_eq!(table.rows.len(), sizes.len());
    for (i, row) in table.rows.iter().enumerate() {
        let span = &doc[row.start as usize..row.end as usize];
        assert!(
            span.starts_with("<spectrum "),
            "row {i} starts at {}",
            row.start
        );
        assert!(span.ends_with("</spectrum>"), "row {i} ends at {}", row.end);
        assert!(
            span.contains(&format!("scan={}\"", i + 1)),
            "row {i} holds its own spectrum"
        );
        assert_eq!(
            span.matches("<spectrum ").count(),
            1,
            "row {i} holds one spectrum"
        );
    }
}
