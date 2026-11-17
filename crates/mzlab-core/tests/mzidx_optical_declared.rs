//! An mzML whose `spectrumList` count includes optical (DAD) spectra: the index holds the scans only, and `Stats::missing` stays 0 (the browser's
//! «partial read» notice and the CLI report both read it). A list cut before it closes still reports what it lacks. Synthetic data.

use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare_in;

const MS: &str = r#"<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="2"/>"#;

fn spectrum(i: usize, optical: bool) -> String {
    let kind = if optical {
        r#"<cvParam cvRef="MS" accession="MS:1000804" name="electromagnetic radiation spectrum" value=""/>"#.to_string()
    } else {
        MS.to_string()
    };
    format!(
        "<spectrum index=\"{i}\" id=\"scan={i}\" defaultArrayLength=\"0\">{kind}<scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"minute\"/></scan></scanList><binaryDataArrayList count=\"0\"></binaryDataArrayList></spectrum>\n",
        i as f64 * 0.01
    )
}

/// `scans` MS2 spectra then `uv` optical ones; the list declares `count`; `close` ends the list and the file.
fn mzml(scans: usize, uv: usize, count: usize, close: bool) -> Vec<u8> {
    let mut s =
        format!("<?xml version=\"1.0\"?><mzML><run id=\"r\"><spectrumList count=\"{count}\">\n");
    (0..scans + uv).for_each(|i| s += &spectrum(i, i >= scans));
    if close {
        s += "</spectrumList></run></mzML>\n";
    }
    s.into_bytes()
}

fn missing(mut data: Vec<u8>) -> (usize, usize) {
    let size = data.len() as u64;
    let p = prepare_in(&mut data, size, "synthetic.mzML", None, None).unwrap();
    let (_, st) = build_mzml_to_vec(&p.table, &data, 64 << 20).unwrap();
    (st.scans, st.missing)
}

#[test]
fn optical_spectra_in_the_declared_count_are_not_missing_scans() {
    // 6 scans and 5000 optical spectra, as the wiff conversion writes them: the list says 5006
    assert_eq!(missing(mzml(6, 5000, 5006, true)), (6, 0));
}

#[test]
fn a_list_cut_before_it_closes_still_names_what_is_missing() {
    assert_eq!(missing(mzml(6, 3, 20, false)), (6, 11));
}
