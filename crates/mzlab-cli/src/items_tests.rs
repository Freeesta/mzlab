//! `mzlab-cli items` on a small mzML written here: a stepped CE (20/35/50 on one precursor) gives `ce_mid` 35 and `ce_spread` 15 in the JSON
//! (the same `ms2_exps` the page reads), a single CE gives null for both.

use mzlab_core::json::parse;

use crate::execute;

fn mzml(ces: &[f64]) -> String {
    let scans: String = ces.iter().enumerate().map(|(i, ce)| format!(
        "<spectrum index=\"{i}\" id=\"scan={i}\" defaultArrayLength=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"2\"/>\
<cvParam cvRef=\"MS\" accession=\"MS:1000127\" name=\"centroid spectrum\" value=\"\"/><scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/></scan></scanList>\
<precursorList count=\"1\"><precursor><selectedIonList count=\"1\"><selectedIon><cvParam cvRef=\"MS\" accession=\"MS:1000744\" name=\"selected ion m/z\" value=\"253.1\"/></selectedIon></selectedIonList>\
<activation><cvParam cvRef=\"MS\" accession=\"MS:1000045\" name=\"collision energy\" value=\"{ce}\"/></activation></precursor></precursorList>\
<binaryDataArrayList count=\"2\"><binaryDataArray encodedLength=\"12\"><cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam cvRef=\"MS\" accession=\"MS:1000576\" name=\"no compression\"/><cvParam cvRef=\"MS\" accession=\"MS:1000514\" name=\"m/z array\"/><binary>AAAAAAAAWUA=</binary></binaryDataArray>\
<binaryDataArray encodedLength=\"12\"><cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam cvRef=\"MS\" accession=\"MS:1000576\" name=\"no compression\"/><cvParam cvRef=\"MS\" accession=\"MS:1000515\" name=\"intensity array\"/><binary>AAAAAAAAWUA=</binary></binaryDataArray></binaryDataArrayList></spectrum>",
        1.0 + i as f64 * 0.01)).collect();
    format!("<?xml version=\"1.0\"?><mzML xmlns=\"http://psi.hupo.org/ms/mzml\"><run id=\"r\"><spectrumList count=\"{}\">{scans}</spectrumList></run></mzML>", ces.len())
}

fn exps(ces: &[f64], tag: &str) -> Vec<(f64, Option<f64>, Option<f64>)> {
    let p = std::env::temp_dir().join(format!("items-{}-{tag}.mzML", std::process::id()));
    std::fs::write(&p, mzml(ces)).unwrap();
    let o = execute(&["items".to_string(), p.to_string_lossy().into_owned()]);
    let _ = std::fs::remove_file(&p);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&o.stdout).unwrap();
    v.arr()[0]
        .get("info")
        .unwrap()
        .get("ms2_exps")
        .unwrap()
        .arr()
        .iter()
        .map(|e| {
            (
                e.get("ce").unwrap().f64().unwrap(),
                e.get("ce_mid").and_then(|x| x.f64()),
                e.get("ce_spread").and_then(|x| x.f64()),
            )
        })
        .collect()
}

#[test]
fn a_stepped_ce_has_centre_and_spread_in_the_cli_json() {
    let e = exps(&[20.0, 35.0, 50.0], "step");
    assert_eq!(e.len(), 3);
    assert!(
        e.iter().all(|x| x.1 == Some(35.0) && x.2 == Some(15.0)),
        "{e:?}"
    );
}

#[test]
fn a_single_ce_has_no_spread() {
    let e = exps(&[30.0, 30.0], "one");
    assert_eq!(e, vec![(30.0, None, None)]);
}
