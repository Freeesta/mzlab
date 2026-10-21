//! `spectra.msn_graph@1` on a file that is not mzML (a mzXML stands for the vendor files) and with the molecule given by name from a list: same graph as the
//! command with the formula written out.

use std::path::{Path, PathBuf};

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64(bytes: &[u8]) -> String {
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = c.iter().fold(0u32, |a, b| a << 8 | u32::from(*b)) << (8 * (3 - c.len()));
        for k in 0..=c.len() {
            out.push(B64[(n >> (18 - 6 * k) & 63) as usize] as char);
        }
        out.push_str(&"=".repeat(3 - c.len()));
    }
    out
}

fn peaks(p: &[(f64, f64)]) -> String {
    let bytes: Vec<u8> = p
        .iter()
        .flat_map(|(m, i)| [m.to_be_bytes(), i.to_be_bytes()])
        .flatten()
        .collect();
    format!(
        r#"<peaks precision="64" byteOrder="network" pairOrder="m/z-int">{}</peaks>"#,
        b64(&bytes)
    )
}

/// An infusion of a synthetic compound: one MS1 scan and one MS2 scan (CID, CE 35) of its ion at m/z 305.1.
fn mzxml() -> String {
    let ms2 = [(217.11998, 23.4), (244.06001, 51.4), (261.08001, 100.0)];
    format!(
        r#"<?xml version="1.0"?><mzXML xmlns="http://sashimi.sourceforge.net/schema_revision/mzXML_3.2"><msRun scanCount="2"><scan num="1" msLevel="1" peaksCount="1" polarity="+" retentionTime="PT0.1S" filterLine="FTMS + p ESI Full ms [50.0000-400.0000]">{}</scan><scan num="2" msLevel="2" peaksCount="3" polarity="+" retentionTime="PT0.2S" collisionEnergy="35" filterLine="FTMS + p ESI d Full ms2 305.1000@cid35.00 [50.0000-400.0000]"><precursorMz precursorScanNum="1" activationMethod="CID" windowWideness="1">305.1</precursorMz>{}</scan></msRun></mzXML>"#,
        peaks(&[(305.1, 1e6)]),
        peaks(&ms2)
    )
}

const LIST: &str = "id;name;synonyms;molecular_formula;smiles;inchikey\n;Testium alfa;TA|alfa-testium;C14H15N3O5;;AAAAAAAAAAAAAA-BBBBBBBBCC-N\n;Testium beta;;C13H10NO4;;\n;Pseudium uno;;C2H6O;;\n;Pseudium due;;C3H8O;;\n";

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_msnraw_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("inf.mzXML"), mzxml()).unwrap();
    std::fs::write(d.join("molecules.csv"), LIST).unwrap();
    d
}

fn run(d: &Path, params: &str, molecules: bool) -> mzlab_core::Result<String> {
    let (inputs, wire) = if molecules {
        (
            r#",{"name":"molecules.csv","type":"structure"}"#,
            r#","molecules":"file:molecules.csv""#,
        )
    } else {
        ("", "")
    };
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"inf.mzXML","type":"raw"}}{inputs}],"steps":[
        {{"id":"g","step":"spectra.msn_graph@1","in":{{"source":"file:inf.mzXML"{wire}}},"params":{params}}}]}}"#
    );
    let _ = std::fs::remove_file(d.join("work/g.json"));
    let (_, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)?;
    match std::fs::read_to_string(d.join("work/g.json")) {
        Ok(t) => Ok(t),
        Err(_) => Err(mzlab_core::Error::new("err.test.run").with("prov", prov)),
    }
}

fn cli(d: &Path, args: &[&str]) -> String {
    let mut a = vec![
        "msn-graph".to_string(),
        d.join("inf.mzXML").to_str().unwrap().to_string(),
    ];
    a.extend(args.iter().map(|s| s.to_string()));
    let o = crate::execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn a_file_that_is_not_mzml_gives_the_graph_of_the_command() {
    let d = dir("a");
    let got = run(&d, r#"{"formula":"C14H15N3O5+"}"#, false).unwrap();
    assert_eq!(got, cli(&d, &["--formula", "C14H15N3O5+"]));
    // the root and the three peaks of the MS2 scan are nodes, the root has its sub-formula
    for mz in ["305.1", "217.11998", "244.06001", "261.08001"] {
        assert!(
            got.contains(&format!("\"mz\":{mz}")),
            "{mz} is not a node: {got}"
        );
    }
    assert!(got.contains("\"sorgente\":{\"nome\":\"inf.mzXML\""));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_molecule_by_name_gives_the_graph_of_its_formula() {
    let d = dir("b");
    let want = run(&d, r#"{"formula":"C14H15N3O5 [M+H]+"}"#, false).unwrap();
    for name in [
        "Testium alfa",
        "ta",
        "alfa-testium",
        "ik:AAAAAAAAAAAAAA-BBBBBBBBCC-N",
        "C14H15N3O5",
    ] {
        let got = run(&d, &format!(r#"{{"molecule":"{name}"}}"#), true).unwrap();
        // the options echo the name's resolution: formula and nothing else changes
        assert_eq!(got, want, "{name}");
    }
    // another adduct is written after the formula
    let neg = run(
        &d,
        r#"{"molecule":"Testium alfa","adduct":"[M+Na]+"}"#,
        true,
    )
    .unwrap();
    assert_eq!(
        neg,
        run(&d, r#"{"formula":"C14H15N3O5 [M+Na]+"}"#, false).unwrap()
    );
    // `formula` wins over the name
    let both = run(
        &d,
        r#"{"molecule":"Testium beta","formula":"C14H15N3O5 [M+H]+"}"#,
        true,
    )
    .unwrap();
    assert_eq!(both, want);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_name_that_is_not_one_molecule_stops_the_step() {
    let d = dir("c");
    // not in the list, ambiguous (two molecules start with it), and no list at all
    for (params, list) in [
        (r#"{"molecule":"Nessuno"}"#, true),
        (r#"{"molecule":"Pseudium"}"#, true),
        (r#"{"molecule":"Testium alfa"}"#, false),
    ] {
        let e = run(&d, params, list).unwrap_err();
        let text = e.to_string();
        assert!(
            text.contains("molecule") || text.contains("molecules"),
            "{params}: {text}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}
