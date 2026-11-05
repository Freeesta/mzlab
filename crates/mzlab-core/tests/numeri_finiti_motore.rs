//! The plain-mzML path of the engine (scan table, parser, chromatogram reader): a NaN or infinite RT, TIC, m/z, intensity or trace point is
//! `err.file.nonFinite` (scan) or `err.file.nonFiniteTrace`, like the readers of `mzlab-lettori`, never a number passed on to the index or the charts.
use base64::Engine;
use mzlab_core::mzml::{parse, Item};

fn array(acc: &str, vals: &[f64]) -> String {
    let raw: Vec<u8> = vals.iter().flat_map(|v| v.to_le_bytes()).collect();
    format!(
        r#"<binaryDataArray encodedLength="0"><cvParam accession="MS:1000523" name="64-bit float"/><cvParam accession="MS:1000576" name="no compression"/><cvParam accession="{acc}"/><binary>{}</binary></binaryDataArray>"#,
        base64::engine::general_purpose::STANDARD.encode(raw)
    )
}

/// Three MS1 spectra and a TIC chromatogram; `rt`, `tic`, `mz` and `tempo` replace the values of the second spectrum and of the trace.
fn file(rt: &str, tic: &str, mz: f64, tempo: f64) -> Vec<u8> {
    let mut s = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?>
<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><run id="r"><spectrumList count="3">
"#,
    );
    for k in 0..3 {
        let (r, t, m) = if k == 1 {
            (rt.to_string(), tic.to_string(), mz)
        } else {
            (format!("{}", 0.5 + k as f64), "1000".to_string(), 150.0)
        };
        s += &format!(
            r#"<spectrum index="{k}" id="scan={}" defaultArrayLength="3"><cvParam accession="MS:1000511" name="ms level" value="1"/><cvParam accession="MS:1000285" name="total ion current" value="{t}"/><scanList count="1"><scan><cvParam accession="MS:1000016" name="scan start time" value="{r}" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>
"#,
            k + 1,
            array("MS:1000514", &[100.0, m, 200.0]),
            array("MS:1000515", &[10.0, 20.0, 30.0]),
        );
    }
    s += &format!(
        r#"</spectrumList><chromatogramList count="1"><chromatogram index="0" id="TIC" defaultArrayLength="3"><cvParam accession="MS:1000235" name="total ion current chromatogram"/><binaryDataArrayList count="2">{}{}</binaryDataArrayList></chromatogram></chromatogramList></run></mzML>
"#,
        array("MS:1000595", &[0.5, tempo, 2.5]).replace(
            r#"accession="MS:1000595"/>"#,
            r#"accession="MS:1000595" unitAccession="UO:0000031" unitName="minute"/>"#
        ),
        array("MS:1000515", &[3000.0, 3000.0, 3000.0]),
    );
    s.into_bytes()
}

fn buono() -> Vec<u8> {
    file("1.5", "1000", 150.0, 1.5)
}

fn chiave_tabella(b: &[u8]) -> String {
    let mut v = b.to_vec();
    match mzlab_core::scantable::read(&mut v, b.len() as u64) {
        Ok(_) => "ok".into(),
        Err(e) => e.key.to_string(),
    }
}

fn chiave_parser(b: &[u8]) -> String {
    match parse(std::io::Cursor::new(b), |_: Item| {}) {
        Ok(()) => "ok".into(),
        Err(e) => e.key.to_string(),
    }
}

fn chiave_cromatogramma(b: &[u8]) -> String {
    let mut m = mzlab_core::mzmlhead::Mzml::new(b.to_vec(), b.len() as u64);
    let v = match m.chromatograms(0) {
        Ok(v) => v,
        Err(e) => return e.key.to_string(),
    };
    for c in &v {
        if let Err(e) = m.chromatogram(c) {
            return e.key.to_string();
        }
    }
    "ok".into()
}

#[test]
fn il_file_buono_passa_ovunque() {
    let b = buono();
    assert_eq!(chiave_tabella(&b), "ok");
    assert_eq!(chiave_parser(&b), "ok");
    assert_eq!(chiave_cromatogramma(&b), "ok");
}

#[test]
fn un_tempo_o_un_tic_non_finito_ferma_la_tabella_delle_scansioni() {
    for (rt, tic) in [
        ("NaN", "1000"),
        ("inf", "1000"),
        ("-inf", "1000"),
        ("1.5", "inf"),
        ("1.5", "NaN"),
    ] {
        let b = file(rt, tic, 150.0, 1.5);
        assert_eq!(
            chiave_tabella(&b),
            "err.file.nonFinite",
            "rt {rt} tic {tic}"
        );
    }
}

#[test]
fn un_tempo_non_finito_ferma_il_parser() {
    for rt in ["NaN", "inf", "-inf"] {
        let b = file(rt, "1000", 150.0, 1.5);
        assert_eq!(chiave_parser(&b), "err.file.nonFinite", "rt {rt}");
    }
}

#[test]
fn un_picco_non_finito_ferma_il_parser() {
    for mz in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let b = file("1.5", "1000", mz, 1.5);
        assert_eq!(chiave_parser(&b), "err.file.nonFinite", "mz {mz}");
    }
}

#[test]
fn un_punto_non_finito_del_cromatogramma_e_un_errore_della_traccia() {
    let b = file("1.5", "1000", 150.0, f64::NAN);
    assert_eq!(chiave_cromatogramma(&b), "err.file.nonFiniteTrace");
    assert_eq!(chiave_parser(&b), "err.file.nonFiniteTrace");
}
