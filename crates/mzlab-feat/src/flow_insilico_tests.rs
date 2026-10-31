//! `spectra.insilico` text: the same bytes from a CSV list and from the pieces of a package, only the pieces of the mass window read, a `screen/1` text as the source.

use super::*;
use crate::flow_ms2::json_params;

const MGF: &str = "BEGIN IONS\nTITLE=q0\nPEPMASS=151.0754\nCHARGE=1+\nIONMODE=Positive\n105.0335 1000\n77.0386 400\n123.0441 300\n60.0000 50\nEND IONS\n";
const HEAD: &str = "Identifier,CompoundName,SMILES,MolecularFormula,MonoisotopicMass\n";
const FAR: &str = "Z9,Far away,CCCCCCCC,C8H18,114.14085\n";
const NEAR: &str = "A1,Candidate A,CCOC(=O)c1ccccc1,C9H10O2,150.06808\nA2,\"Candidate, B\",COC(=O)Cc1ccccc1,C9H10O2,150.06808\n";
const CATALOGUE: &str = r#"{"format":"insilico/1","pieces":[{"file":"000.bin","mass_min":114.14085,"mass_max":114.14085},{"file":"001.bin","mass_min":150.06808,"mass_max":150.06808}]}"#;

fn params(extra: &str) -> Vec<(String, Value)> {
    json_params(&format!(
        "{{\"db_ppm\":5,\"frag_ppm\":10,\"max_bonds\":200,\"top\":5{extra}}}"
    ))
    .unwrap()
}

fn full_csv() -> String {
    format!("{HEAD}{FAR}{NEAR}")
}

fn no_piece(_: &str) -> Result<Vec<u8>> {
    Err(Error::new("err.flow.insilico.piece"))
}

#[test]
fn a_package_gives_the_bytes_of_the_same_csv_and_reads_only_the_window() {
    let (p, spectra) = (params(""), ("q.mgf", MGF.as_bytes()));
    let direct = insilico_text(&p, spectra, full_csv().as_bytes(), &mut no_piece).unwrap();
    assert!(direct.contains("\"id\":\"A1\""), "{direct}");
    assert_eq!(
        pieces_needed(&p, spectra, CATALOGUE.as_bytes()).unwrap(),
        ["001.bin"]
    );
    let mut asked = Vec::new();
    let mut piece = |f: &str| -> Result<Vec<u8>> {
        asked.push(f.to_string());
        match f {
            "001.bin" => Ok(format!("{HEAD}{NEAR}").into_bytes()),
            _ => panic!("piece {f} is outside the window"),
        }
    };
    let packed = insilico_text(&p, spectra, CATALOGUE.as_bytes(), &mut piece).unwrap();
    assert_eq!(packed, direct);
    assert_eq!(asked, ["001.bin"], "each piece is read once");
}

#[test]
fn a_wider_window_reads_both_pieces_and_a_missing_piece_is_an_error() {
    let p = params("")
        .into_iter()
        .map(|(k, v)| {
            if k == "db_ppm" {
                (k, Value::Num(300000.0))
            } else {
                (k, v)
            }
        })
        .collect::<Vec<_>>();
    let spectra = ("q.mgf", MGF.as_bytes());
    assert_eq!(
        pieces_needed(&p, spectra, CATALOGUE.as_bytes()).unwrap(),
        ["000.bin", "001.bin"]
    );
    let e = insilico_text(&p, spectra, CATALOGUE.as_bytes(), &mut no_piece).unwrap_err();
    assert_eq!(e.key, "err.flow.insilico.piece");
}

#[test]
fn the_catalogue_is_checked() {
    assert!(Package::parse(br#"{"format":"normative/1","pieces":[]}"#).is_err());
    assert!(Package::parse(
        br#"{"format":"insilico/1","pieces":[{"file":"../x","mass_min":1,"mass_max":2}]}"#
    )
    .is_err());
    assert!(Package::parse(br#"{"format":"insilico/1","pieces":[]}"#).is_err());
    assert!(is_package(CATALOGUE.as_bytes()) && !is_package(full_csv().as_bytes()));
}

#[test]
fn the_structures_of_a_screen_text_are_the_candidates() {
    let screen = r#"{"format":"screen/1","hits":[{"formula":"C9H10O2","candidates":[
        {"skeleton":"S1","name":"Candidate A","smiles":"CCOC(=O)c1ccccc1","neutral_mass":150.06808},
        {"skeleton":"S1","name":"again","smiles":"CCOC(=O)c1ccccc1","neutral_mass":150.06808},
        {"skeleton":"S2","name":"no structure","smiles":"","neutral_mass":150.06808}]}]}"#;
    let t = insilico_text(
        &params(""),
        ("q.mgf", MGF.as_bytes()),
        screen.as_bytes(),
        &mut no_piece,
    )
    .unwrap();
    assert!(
        t.contains("\"id\":\"S1\"") && !t.contains("S2") && t.matches("\"query\"").count() == 1,
        "{t}"
    );
    let none = insilico_text(
        &params(""),
        ("q.mgf", MGF.as_bytes()),
        br#"{"format":"screen/1","hits":[]}"#,
        &mut no_piece,
    )
    .unwrap_err();
    assert_eq!(none.key, "err.flow.insilico.suspects");
}
