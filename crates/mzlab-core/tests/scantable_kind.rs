//! The scan table keeps `kind` only for MS2+ and still reads the collision energy of an MS1 scan (all-ion fragmentation).

use mzlab_core::scantable;

fn spectrum(i: usize, level: u8, extra: &str, filter: &str, prec: &str) -> String {
    format!(
        "<spectrum index=\"{i}\" id=\"scan={}\" defaultArrayLength=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"{level}\"/>{extra}\
         <scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000512\" name=\"filter string\" value=\"{filter}\"/>\
         <cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"minute\"/></scan></scanList>{prec}\
         <binaryDataArrayList count=\"1\"><binaryDataArray><binary>QUFB</binary></binaryDataArray></binaryDataArrayList></spectrum>",
        i + 1,
        0.1 * i as f64
    )
}

fn precursor(mz: f64) -> String {
    format!(
        "<precursorList count=\"1\"><precursor><selectedIonList count=\"1\"><selectedIon><cvParam cvRef=\"MS\" accession=\"MS:1000744\" name=\"selected ion m/z\" value=\"{mz}\"/></selectedIon></selectedIonList></precursor></precursorList>"
    )
}

#[test]
fn kind_is_for_msn_and_ms1_energy_is_kept() {
    let term = |acc: &str| format!("<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"t\"/>");
    let ce =
        "<cvParam cvRef=\"MS\" accession=\"MS:1000045\" name=\"collision energy\" value=\"35\"/>";
    let spectra = [
        spectrum(0, 1, "", "", ""),
        // the survey scan of an all-ion run: a scan-type term and an energy, but no kind
        spectrum(1, 1, &(term("MS:1000341") + ce), "", ""),
        spectrum(2, 2, &term("MS:1000326"), "", &precursor(200.5)),
        spectrum(3, 2, "", "+ c ESI pr 150.0 [50.0-300.0]", &precursor(150.0)),
        spectrum(4, 2, &term("MS:1000789"), "", &precursor(310.2)),
        spectrum(5, 2, "", "", &precursor(180.0)),
    ];
    let mut doc =
        String::from("<?xml version=\"1.0\"?><mzML><run id=\"r\"><spectrumList count=\"6\">");
    for s in &spectra {
        doc.push_str(s);
    }
    doc.push_str("</spectrumList></run></mzML>");
    let bytes = doc.into_bytes();
    let size = bytes.len() as u64;
    let table = scantable::read(&mut bytes.clone(), size).unwrap();
    let kinds: Vec<_> = table.rows.iter().map(|r| r.kind).collect();
    assert_eq!(
        kinds,
        [None, None, Some("NL"), Some("PREC"), Some("EMC"), None]
    );
    let rows = table.rows_json();
    assert_eq!(rows, EXPECTED);
}

/// The rows as the reader wrote them before `kind` was left out for MS1 (the MS1 energy `35.0` is there).
const EXPECTED: &str = r#"[[63,521,1,0.0,0,0.0,null,null,"",false,null,null,null,null,"?",null,null,[],null,"scan=1"],[521,1113,1,0.1,0,0.0,null,35.0,"",false,null,null,null,null,"?",null,null,[],null,"scan=2"],[1113,1844,2,0.2,0,0.0,200.5,null,"",false,1,[200.0,201.0],null,null,"?",null,null,[{"target":null,"lo":null,"hi":null,"selected":200.5,"charge":null,"ref":null,"act":null,"ce":null,"pint":null}],null,"scan=3","NL"],[1844,2565,2,0.30000000000000004,0,0.0,150.0,null,"+ c ESI pr 150.0 [50.0-300.0]",false,1,[149.5,150.5],null,null,"?",null,null,[{"target":null,"lo":null,"hi":null,"selected":150.0,"charge":null,"ref":null,"act":null,"ce":null,"pint":null}],null,"scan=4","PREC"],[2565,3296,2,0.4,0,0.0,310.2,null,"",false,1,[309.7,310.7],null,null,"?",null,null,[{"target":null,"lo":null,"hi":null,"selected":310.2,"charge":null,"ref":null,"act":null,"ce":null,"pint":null}],null,"scan=5","EMC"],[3296,3972,2,0.5,0,0.0,180.0,null,"",false,1,[179.5,180.5],null,null,"?",null,null,[{"target":null,"lo":null,"hi":null,"selected":180.0,"charge":null,"ref":null,"act":null,"ce":null,"pint":null}],null,"scan=6"]]"#;
