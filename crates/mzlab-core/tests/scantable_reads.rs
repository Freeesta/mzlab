//! The scan table of an indexed mzML reads each spectrum header with a single read (the tail of the previous spectrum rides along), and the
//! rows are the same whatever the sizes of the spectra.

use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::scantable;

struct Counting {
    bytes: Vec<u8>,
    calls: usize,
}

impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.calls += 1;
        buf.copy_from_slice(&self.bytes[off as usize..off as usize + buf.len()]);
        Ok(())
    }
}

fn spectrum(i: usize, level: u8, payload: usize) -> String {
    let prec = if level == 2 {
        "<precursorList count=\"1\"><precursor><selectedIonList count=\"1\"><selectedIon><cvParam cvRef=\"MS\" accession=\"MS:1000744\" name=\"selected ion m/z\" value=\"300.5\"/></selectedIon></selectedIonList></precursor></precursorList>"
    } else {
        ""
    };
    format!(
        "<spectrum index=\"{i}\" id=\"scan={}\" defaultArrayLength=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"{level}\"/>\
         <scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"minute\"/></scan></scanList>{prec}\
         <binaryDataArrayList count=\"1\"><binaryDataArray><binary>{}</binary></binaryDataArray></binaryDataArrayList></spectrum>\n",
        i + 1,
        0.1 * i as f64,
        "A".repeat(payload)
    )
}

/// An indexed mzML; `sizes` are the payloads of the spectra (every third is an MS2).
fn indexed(sizes: &[usize]) -> Vec<u8> {
    let mut doc = String::from(
        "<?xml version=\"1.0\"?>\n<indexedmzML><mzML><run id=\"r\"><spectrumList count=\"0\">\n",
    );
    let mut offs = Vec::new();
    for (i, n) in sizes.iter().enumerate() {
        offs.push(doc.len());
        doc.push_str(&spectrum(i, if i % 3 == 2 { 2 } else { 1 }, *n));
    }
    doc.push_str("</spectrumList></run></mzML>\n");
    let at = doc.len();
    doc.push_str("<indexList count=\"1\"><index name=\"spectrum\">\n");
    for (i, o) in offs.iter().enumerate() {
        doc.push_str(&format!("<offset idRef=\"scan={}\">{o}</offset>\n", i + 1));
    }
    doc.push_str(&format!(
        "</index></indexList>\n<indexListOffset>{at}</indexListOffset></indexedmzML>"
    ));
    doc.into_bytes()
}

fn read_rows(sizes: &[usize]) -> (usize, String) {
    let bytes = indexed(sizes);
    let size = bytes.len() as u64;
    let mut src = Counting {
        bytes: bytes.clone(),
        calls: 0,
    };
    let table = scantable::read(&mut src, size).unwrap();
    assert_eq!(table.rows.len(), sizes.len());
    for (i, row) in table.rows.iter().enumerate() {
        let span = std::str::from_utf8(&bytes[row.start as usize..row.end as usize]).unwrap();
        assert!(
            span.starts_with("<spectrum ") && span.ends_with("</spectrum>"),
            "row {i}"
        );
        assert!(span.contains(&format!("scan={}\"", i + 1)), "row {i}");
    }
    (src.calls, table.rows_json())
}

#[test]
fn one_read_per_header_and_same_rows() {
    let sizes = [
        200usize, 5000, 90_000, 0, 7, 300_000, 4000, 12_000, 50, 2_000,
    ];
    let (calls, rows) = read_rows(&sizes);
    assert_eq!(rows, EXPECTED);
    // what a spectrum adds to the reads, whatever the fixed ones of the file are
    let (half, _) = read_rows(&sizes[..5]);
    assert_eq!(calls - half, 5, "one read per further spectrum");
}

#[test]
fn spans_are_whole_for_mixed_sizes() {
    // sizes from 0 to 40 KB, many under the 4 KB header window
    let mut x = 12345u64;
    let sizes: Vec<usize> = (0..80)
        .map(|_| {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let v = (x >> 33) as usize;
            if v.is_multiple_of(7) {
                0
            } else {
                v % 40_000
            }
        })
        .collect();
    read_rows(&sizes);
}

/// The rows as the reader wrote them with two reads per header.
const EXPECTED: &str = r#"[[78,658,1,0.0,0,0.0,null,null,"",false,null,null,null,null,"?",null,null,[],null,"scan=1"],[659,6041,1,0.1,0,0.0,null,null,"",false,null,null,null,null,"?",null,null,[],null,"scan=2"],[6042,96642,2,0.2,0,0.0,300.5,null,"",false,1,[300.0,301.0],null,null,"?",null,null,[{"target":null,"lo":null,"hi":null,"selected":300.5,"charge":null,"ref":null,"act":null,"ce":null,"pint":null}],null,"scan=3"],[96643,97041,1,0.30000000000000004,0,0.0,null,null,"",false,null,null,null,null,"?",null,null,[],null,"scan=4"],[97042,97431,1,0.4,0,0.0,null,null,"",false,null,null,null,null,"?",null,null,[],null,"scan=5"],[97432,398032,2,0.5,0,0.0,300.5,null,"",false,4,[300.0,301.0],null,null,"?",null,null,[{"target":null,"lo":null,"hi":null,"selected":300.5,"charge":null,"ref":null,"act":null,"ce":null,"pint":null}],null,"scan=6"],[398033,402430,1,0.6000000000000001,0,0.0,null,null,"",false,null,null,null,null,"?",null,null,[],null,"scan=7"],[402431,414828,1,0.7000000000000001,0,0.0,null,null,"",false,null,null,null,null,"?",null,null,[],null,"scan=8"],[414829,415479,2,0.8,0,0.0,300.5,null,"",false,7,[300.0,301.0],null,null,"?",null,null,[{"target":null,"lo":null,"hi":null,"selected":300.5,"charge":null,"ref":null,"act":null,"ce":null,"pint":null}],null,"scan=9"],[415480,417863,1,0.9,0,0.0,null,null,"",false,null,null,null,null,"?",null,null,[],null,"scan=10"]]"#;
