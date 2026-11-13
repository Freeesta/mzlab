//! The `.wiff` reader of the readers module (feature `estesi`) against the synthetic pairs of `tools/wiff_riferimento.py`: a QTRAP 5500 with scan blocks of
//! versions 1 and 3 and precursor-ion / neutral-loss scans, and the files that must be refused with `err.file.wiffScanType` / `wiffPeriods` or `err.file.wiff` (the cause in `detail`), never read as a
//! full scan. Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::mzml::ChromKind;
use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    // one folder per call: the tests of this file run in parallel threads of one process and would write the same files
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let out = std::env::temp_dir().join(format!(
        "mzlab_wiff_varianti_{}_{}",
        std::process::id(),
        CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_variants: python3 not available");
        return None;
    }
    let json =
        serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap();
    Some((out, json))
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn set(dir: &Path, name: &str) -> FileSet {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Member {
            nome: file,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    }
}

fn open(dir: &Path, name: &str) -> Result<(Wiff, Vec<u8>), mzlab_core::error::Error> {
    let ole = container(
        std::fs::read(dir.join(format!("{name}.wiff"))).unwrap(),
        name,
    )
    .unwrap();
    let scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
    Wiff::open(&ole, name, Some(scan.len() as u64), 1).map(|w| (w, scan))
}

/// The cause of a refusal as one text: the keys of the readers module (`wiffScanType`, `wiffPeriods`, `wiffNeeds2`) spelled as the reference writes them, else `detail`.
fn detail(e: &mzlab_core::error::Error) -> String {
    let p = |n: &str| {
        e.params
            .iter()
            .find(|(k, _)| *k == n)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    match e.key {
        "err.file.wiffScanType" => format!(
            "scan type code {} (experiment {})",
            p("code"),
            p("experiment")
        ),
        "err.file.wiffPeriods" => format!("periods={}", p("periods")),
        "err.file.wiffNeeds2" => "wiff2".into(),
        k => {
            assert_eq!(k, "err.file.wiff");
            p("detail")
        }
    }
}

#[test]
fn a_qtrap_5500_reads_its_scan_types() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let r = &want["variante"]["1"];
    let (w, mut scan) = open(&dir, "variante").unwrap();
    assert!(w.variant() && w.model.as_deref() == r["model"].as_str());
    let run = w.run(&mut scan).unwrap();
    let sp = r["spectra"].as_array().unwrap();
    assert_eq!(run.spectra.len(), sp.len());
    for (g, e) in run.spectra.iter().zip(sp) {
        assert_eq!(g.id, e["id"].as_str().unwrap());
        assert_eq!(
            (g.level as i64, g.precursor),
            (e["level"].as_i64().unwrap(), e["precursor"].as_f64()),
            "{}",
            g.id
        );
        let (mz, it) = (nums(&e["mz"]), nums(&e["intensity"]));
        assert_eq!(g.mz.len(), mz.len(), "{}: points", g.id);
        assert!(
            (0..mz.len()).all(|k| close(g.mz[k], mz[k]) && close(g.intensity_f64(k), it[k])),
            "{}",
            g.id
        );
    }
}

#[test]
fn the_mzml_of_the_module_names_the_instrument_and_the_spectrum_types() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let mut l = readers::apri(set(&dir, "variante"), &Limits::sito()).unwrap();
    assert!(l.variant(0));
    let xml = String::from_utf8(
        mzlab_core::readers_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap(),
    )
    .unwrap();
    assert!(
        xml.contains("accession=\"MS:1000931\" name=\"QTRAP 5500\""),
        "instrument"
    );
    assert!(!xml.contains("name=\"instrument model\""));
    let content = &xml[xml.find("<fileContent>").unwrap()..xml.find("</fileContent>").unwrap()];
    assert!(
        content.contains("MS:1000341") && content.contains("MS:1000326"),
        "{content}"
    );
    for (term, name) in want["variante"]["1"]["spectra"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["term"].as_array())
        .map(|t| (t[0].as_str().unwrap(), t[1].as_str().unwrap()))
    {
        assert!(
            xml.contains(&format!("accession=\"{term}\" name=\"{name}\"")),
            "{term}"
        );
    }
    // a precursor-ion scan has its fixed m/z as the selected ion of its precursor (as ProteoWizard writes it), a neutral-loss scan its loss as a userParam
    // and no precursor; no spectrum has a product
    for s in want["variante"]["1"]["spectra"].as_array().unwrap() {
        let at = xml
            .find(&format!("id=\"{}\"", s["id"].as_str().unwrap()))
            .unwrap();
        let head = &xml[at..at + xml[at..].find("<binaryDataArrayList").unwrap()];
        assert!(!head.contains("<productList"), "{head}");
        let loss = s["loss"]
            .as_f64()
            .map(|l| format!("<userParam name=\"constant neutral loss (Da)\" value=\"{l}\""));
        assert_eq!(
            loss.as_ref().is_some_and(|l| head.contains(l.as_str())),
            loss.is_some(),
            "{head}"
        );
        let selected = s["precursor"]
            .as_f64()
            .map(|m| format!("accession=\"MS:1000744\" name=\"selected ion m/z\" value=\"{m}\""));
        assert_eq!(
            head.contains("<precursorList"),
            selected.is_some(),
            "{head}"
        );
        assert!(selected.is_none_or(|m| head.contains(&m)), "{head}");
    }
    // one precursor-ion and one neutral-loss spectrum per cycle (two cycles), each type in its own spectra
    assert_eq!(
        xml.matches("accession=\"MS:1000341\" name=\"precursor ion spectrum\"")
            .count(),
        2 + 1
    );
    assert_eq!(
        xml.matches("accession=\"MS:1000326\" name=\"constant neutral loss spectrum\"")
            .count(),
        1 + 1
    );
}

/// A 3200 QTRAP whose product-ion scan blocks are version 3 is a variant (the engine's reader decodes version 2 only and would list it with no scans); a 3200 file
/// with version 2 blocks only is not.
#[test]
fn a_3200_with_version_3_blocks_is_a_variant() {
    let Some((dir, _)) = reference() else {
        return;
    };
    let mut l = readers::apri(set(&dir, "epi3200"), &Limits::sito()).unwrap();
    assert!(l.variant(0));
    assert_eq!(l.samples()[0].scansioni, 2);
    let mut l = readers::apri(set(&dir, "prodotto"), &Limits::sito()).unwrap();
    assert!(!l.variant(0));
}

#[test]
fn what_the_reader_does_not_know_is_refused_never_read_as_a_full_scan() {
    let Some((dir, want)) = reference() else {
        return;
    };
    for (name, text) in want["rifiuti"].as_object().unwrap() {
        let text = text.as_str().unwrap();
        // a code or a period is refused when the sample is opened; a block when it is read
        let err = match open(&dir, name) {
            Err(e) => e,
            Ok((w, mut scan)) => w
                .run(&mut scan)
                .err()
                .unwrap_or_else(|| panic!("{name}: read")),
        };
        assert_eq!(detail(&err), text, "{name}");
        let via = readers::apri(set(&dir, name), &Limits::sito());
        assert_eq!(
            detail(&via.err().unwrap_or_else(|| panic!("{name}: registry"))),
            text,
            "{name}: registry"
        );
    }
}

/// A damaged file never panics: every byte of the container and of the scan file of the synthetic QTRAP 5500 pair is overwritten in turn with values that
/// make sizes, counts and codes absurd; the reader opens it or refuses it with an `err.*` key, and the method document is always written.
#[test]
fn damaged_files_are_refused_or_read_never_a_panic() {
    let Some((dir, _)) = reference() else { return };
    let wiff = std::fs::read(dir.join("variante.wiff")).unwrap();
    let scan = std::fs::read(dir.join("variante.wiff.scan")).unwrap();
    let try_open = |wiff: Vec<u8>, scan: Vec<u8>| {
        let Ok(ole) = container(wiff, "variante.wiff") else {
            return;
        };
        drop(mzlab_core::method_wiff::from_wiff(
            &ole,
            1,
            &Default::default(),
        ));
        let mut scan = scan;
        if let Ok(w) = Wiff::open(&ole, "variante.wiff", Some(scan.len() as u64), 1) {
            drop(w.run(&mut scan));
        }
    };
    for at in (0..wiff.len()).step_by(7) {
        for v in [0u8, 0xff, 0x7f] {
            let mut w = wiff.clone();
            w[at] = v;
            try_open(w, scan.clone());
        }
    }
    for at in 0..scan.len() {
        for v in [0u8, 0xff, 0x80] {
            let mut s = scan.clone();
            s[at] = v;
            try_open(wiff.clone(), s);
        }
    }
    for cut in (0..scan.len()).step_by(11) {
        try_open(wiff.clone(), scan[..cut].to_vec());
    }
}

#[test]
fn wiff_sample_names_date() {
    let Some((dir, _)) = reference() else { return };
    let l = readers::apri(set(&dir, "nomi"), &Limits::sito()).unwrap();
    let samples = l.samples();
    assert_eq!(samples.len(), 5);
    assert_eq!(samples[0].nome, "Sample_A");
    assert_eq!(samples[1].nome, "Sample_B");
    assert_eq!(samples[2].nome, "nomi_s3");
    assert_eq!(samples[3].nome, "nomi_s4");
    assert_eq!(samples[4].nome, "Sample_A");
    for c in samples {
        assert_eq!(c.size, None);
    }
}

/// A full scan experiment gives a TIC chromatogram, and the TIC of each spectrum in the mzML is the one of the header of its block when the file has one
/// (here 1234567.891, not the sum of the profile), the sum of the profile where it has none.
#[test]
fn a_full_scan_has_its_tic_and_each_spectrum_the_tic_of_its_header() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let want = &want["totale"];
    let (tic, time) = (nums(&want["tic"]), nums(&want["time"]));
    let (w, mut scan) = open(&dir, "totale").unwrap();
    let run = w.run(&mut scan).unwrap();
    assert_eq!(run.chromatograms.len(), 2);
    let c = &run.chromatograms[0];
    assert!((c.id.as_str(), c.kind) == ("TIC", ChromKind::Tic));
    assert!(c.time.iter().zip(&time).all(|(a, b)| close(*a, *b)));
    // the chromatogram is the TIC of the index as the file wrote it, a missing one (0.0) included
    assert_eq!(c.intensity, vec![tic[0], 0.0, tic[2]]);
    let bpc = &run.chromatograms[1];
    assert!((bpc.id.as_str(), bpc.kind) == ("BPC", ChromKind::Bpc));
    assert_eq!(bpc.time, c.time);
    assert!(bpc
        .intensity
        .iter()
        .zip(nums(&want["bpc"]))
        .all(|(a, b)| close(*a, b)));
    let mut l = readers::apri(set(&dir, "totale"), &Limits::sito()).unwrap();
    // listing the chromatograms decodes no scan: the BPC is there once the writer has gone through the scans
    let ids = |l: &mut dyn readers::Reader| {
        l.chromatograms(0)
            .unwrap()
            .into_iter()
            .map(|v| v.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&mut *l), ["TIC"]);
    let xml = String::from_utf8(
        mzlab_core::readers_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap(),
    )
    .unwrap();
    let got: Vec<f64> = xml
        .split("name=\"total ion current\" value=\"")
        .skip(1)
        .map(|t| t.split('"').next().unwrap().parse().unwrap())
        .collect();
    assert_eq!(got.len(), 3);
    assert!(
        got.iter().zip(&tic).all(|(a, b)| close(*a, *b)),
        "{got:?} {tic:?}"
    );
    assert!(xml.contains("<chromatogram index=\"0\" id=\"TIC\""));
    assert!(xml.contains("<chromatogram index=\"1\" id=\"BPC\""));
    assert_eq!(ids(&mut *l), ["TIC", "BPC"]);
    let back = mzlab_core::mzml::read(xml.as_bytes()).unwrap();
    assert!(back.chromatograms[1]
        .intensity
        .iter()
        .zip(&run.chromatograms[1].intensity)
        .all(|(a, b)| close(*a, *b)));
}

/// A sample of an MS3 experiment (code 13) is read as level-3 spectra whose precursor is the second isolated mass and whose first one is a userParam, and a sample the
/// reader refuses stays in the list with its reason: it never makes the pair fail, nor stops the samples before it.
#[test]
fn an_ms3_sample_is_read_and_a_refused_sample_does_not_fail_the_pair() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let r = &want["ms3"];
    let mut l = readers::apri(set(&dir, "ms3"), &Limits::sito()).unwrap();
    let names: Vec<_> = l.samples().iter().map(|s| s.nome.clone()).collect();
    let wanted: Vec<_> = r["names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, wanted);
    let xml = String::from_utf8(
        mzlab_core::readers_mzml::scrivi(&mut *l, 1, Vec::new(), &mut |_, _| {}).unwrap(),
    )
    .unwrap();
    let sp = r["spectra"].as_array().unwrap();
    assert_eq!(xml.matches("<spectrum ").count(), sp.len());
    assert_eq!(
        xml.matches("name=\"ms level\" value=\"3\"").count(),
        sp.len()
    );
    assert_eq!(
        xml.matches("userParam name=\"ms3 first precursor m/z\" value=\"237.1\"")
            .count(),
        sp.len()
    );
    assert_eq!(
        xml.matches("name=\"selected ion m/z\" value=\"194.1\"")
            .count(),
        sp.len()
    );
    // the first sample reads as before, the refused one says why when its data are asked for
    assert!(mzlab_core::readers_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).is_ok());
    let e = mzlab_core::readers_mzml::scrivi(&mut *l, 2, Vec::new(), &mut |_, _| {})
        .expect_err("sample 3 is refused");
    assert_eq!(detail(&e), "scan type code 21 (experiment 1)");
}
