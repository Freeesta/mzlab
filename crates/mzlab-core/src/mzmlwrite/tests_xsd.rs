//! The indexed file against the PSI schema `mzML1.1.2_idx.xsd` (`xmllint`). The schema files are fetched once into a cache folder; without `xmllint`,
//! `curl` or network the test says so and passes, so only a machine that can check really checks.
use super::tests_index::{indexed_header, indexed_sample};
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter, Options, ProcStep, SpectrumExtra};
use std::path::PathBuf;
use std::process::Command;

const BASE: &str = "https://raw.githubusercontent.com/HUPO-PSI/mzML/master/schema/schema_1.1/";

/// The folder with `mzML1.1.2_idx.xsd` and `mzML1.1.0.xsd`, or why it cannot be had.
fn schemas() -> Result<PathBuf, String> {
    let dir = std::env::var_os("MZLAB_XSD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("mzlab-psi-xsd"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for n in ["mzML1.1.2_idx.xsd", "mzML1.1.0.xsd"] {
        let f = dir.join(n);
        if std::fs::metadata(&f).map_or(true, |m| m.len() < 1000) {
            let ok = Command::new("curl")
                .args(["-fsS", "-m", "30", "-o"])
                .arg(&f)
                .arg(format!("{BASE}{n}"))
                .status()
                .map_err(|e| format!("curl: {e}"))?
                .success();
            if !ok {
                return Err(format!("cannot fetch {n}"));
            }
        }
    }
    Ok(dir)
}

#[test]
fn the_indexed_file_validates_against_the_psi_schema() {
    let step = ProcStep {
        accession: "MS:1000035".into(),
        name: "peak picking".into(),
        software: "features.detect@1".into(),
        version: "1".into(),
        params: vec![("min_height".into(), "1000".into())],
    };
    // a header with no source file and no component, which must leave out both lists
    let bare = indexed_header(Header {
        run: "x".into(),
        ..Default::default()
    })
    .1;
    let full = [Vec::new(), vec![step]].map(|steps| indexed_sample(steps).1);
    for idx in std::iter::once(bare).chain(full) {
        validate(&idx);
    }
}

/// A product-ion spectrum with its plain precursor, and a precursor-ion one: its fixed m/z as its `<product>`, with no precursor.
#[test]
fn a_product_ion_spectrum_with_a_plain_precursor_validates() {
    let h = Header {
        run: "x".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::indicizzato(Vec::new(), &h, Options::default()).unwrap();
    w.open_spectra(3).unwrap();
    for (i, extra) in [
        SpectrumExtra::default(),
        SpectrumExtra {
            ce: Some(25.5),
            iso_half: Some(0.35),
            ..Default::default()
        },
        SpectrumExtra {
            iso_half: Some(0.35),
            product: Some(184.0),
            ..Default::default()
        },
    ]
    .iter()
    .enumerate()
    {
        let sp = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 2,
            rt: 0.5,
            polarity: 1,
            precursor: extra.product.is_none().then_some(229.1),
            mz: vec![100.5, 200.25],
            intensity: vec![10.0, 20.0],
            ..Default::default()
        };
        w.spectrum_with(&sp, extra).unwrap();
    }
    w.close_spectra().unwrap();
    let out = w.finish().unwrap();
    let text = String::from_utf8_lossy(&out);
    let pis = &text[text.find("id=\"scan=3\"").unwrap()..];
    let pis = &pis[..pis.find("</spectrum>").unwrap()];
    assert!(
        pis.contains("<productList count=\"1\"><product><isolationWindow>")
            && !pis.contains("<precursor"),
        "{pis}"
    );
    validate(&out);
}

#[test]
fn an_srm_chromatogram_validates() {
    use crate::mzml::{ChromKind, Chromatogram};
    let h = Header {
        run: "x".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::indicizzato(Vec::new(), &h, Options::default()).unwrap();
    w.open_spectra(0).unwrap();
    w.close_spectra().unwrap();
    w.open_chromatograms(2).unwrap();
    for (i, id) in ["SRM SIC Q1=364.5 Q3=176 sample=1 period=1 experiment=1 transition=0", "SRM SIC Q1=364.5 Q3=159 sample=1 period=1 experiment=1 transition=1 ce=25 dwell=50 name=tpB"].iter().enumerate() {
        let c = Chromatogram {
            index: i,
            id: (*id).into(),
            kind: ChromKind::Srm,
            q1: Some(364.5),
            q3: Some(176.0 - 17.0 * i as f64),
            unit: "number of detector counts".into(),
            time: vec![0.5, 1.5],
            intensity: vec![30.0, 31.0],
        };
        w.chromatogram(&c).unwrap();
    }
    w.close_chromatograms().unwrap();
    validate(&w.finish().unwrap());
}

/// A negative SRM and a positive SIM chromatogram of the `.wiff` reader: the scan polarity term from the id, the SIM ion as its precursor's isolation
/// window target with its dwell and no activation term.
#[test]
fn polarity_and_a_sim_precursor_validate() {
    use crate::mzml::{ChromKind, Chromatogram};
    let h = Header {
        run: "x".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::indicizzato(Vec::new(), &h, Options::default()).unwrap();
    w.open_spectra(0).unwrap();
    w.close_spectra().unwrap();
    w.open_chromatograms(2).unwrap();
    let srm = "SRM SIC Q1=364.5 Q3=176 sample=1 period=1 experiment=1 transition=0 dwell=50 polarity=negative name=a b";
    let sim =
        "SIM SIC Q1=237.125 sample=1 period=1 experiment=2 transition=0 dwell=20 polarity=positive";
    for (i, (id, kind, q3)) in [
        (srm, ChromKind::Srm, Some(176.0)),
        (sim, ChromKind::Sic, None),
    ]
    .into_iter()
    .enumerate()
    {
        let c = Chromatogram {
            index: i,
            id: id.into(),
            kind,
            q1: Some(if q3.is_some() { 364.5 } else { 237.125 }),
            q3,
            unit: "number of detector counts".into(),
            time: vec![0.5, 1.5],
            intensity: vec![30.0, 0.0],
        };
        w.chromatogram(&c).unwrap();
    }
    w.close_chromatograms().unwrap();
    let out = w.finish().unwrap();
    let text = String::from_utf8_lossy(&out);
    let (a, b) = text.split_once("SIM SIC").unwrap();
    assert!(
        a.contains("name=\"negative scan\"") && !a.contains("positive scan"),
        "{a}"
    );
    assert!(
        b.contains("name=\"positive scan\"")
            && b.contains("<userParam name=\"MS_dwell_time\" value=\"20\"/>"),
        "{b}"
    );
    assert!(
        b.contains("name=\"isolation window target m/z\" value=\"237.125\"")
            && b.contains("<activation/>")
            && !b.contains("<product>"),
        "{b}"
    );
    validate(&out);
}

#[test]
fn the_tic_of_a_full_scan_validates() {
    use crate::mzml::{ChromKind, Chromatogram};
    let h = Header {
        run: "x".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::indicizzato(Vec::new(), &h, Options::default()).unwrap();
    w.open_spectra(0).unwrap();
    w.close_spectra().unwrap();
    w.open_chromatograms(1).unwrap();
    let tic = Chromatogram {
        index: 0,
        id: "TIC".into(),
        kind: ChromKind::Tic,
        q1: None,
        q3: None,
        unit: "number of detector counts".into(),
        time: vec![0.5, 1.5],
        intensity: vec![1.5e6, 3.0],
    };
    w.chromatogram(&tic).unwrap();
    w.close_chromatograms().unwrap();
    validate(&w.finish().unwrap());
}

/// `idx` against the indexed schema; a machine without `xmllint`, `curl` or network says so and passes.
pub(super) fn validate(idx: &[u8]) {
    let dir = match schemas() {
        Ok(d) => d,
        Err(e) => return eprintln!("skipped: {e}"),
    };
    let file = dir.join(format!(
        "sample-{}-{:?}.mzML",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&file, idx).unwrap();
    let out = Command::new("xmllint")
        .args(["--noout", "--huge", "--schema"])
        .arg(dir.join("mzML1.1.2_idx.xsd"))
        .arg(&file)
        .output();
    std::fs::remove_file(&file).ok();
    match out {
        Err(e) => eprintln!("skipped: xmllint: {e}"),
        Ok(o) => assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr)),
    }
}
