//! The module `aperti` against hostile mzXML files, and (with the private data repository, `MZLAB_DATI`) the mzXML files of msconvert against the mzML of the same raw file; the real-data
//! tests are skipped without it.
#![cfg(feature = "aperti")]

mod comune;

use comune::mzxml::*;
use comune::{b64, file_set, zlib};
use mzlab_core::lettori::Limits;
use std::path::PathBuf;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn text(o: Opz) -> String {
    String::from_utf8(mzxml(&run(), o)).unwrap()
}

#[test]
fn index_offsets_do_not_count_scans_come_from_the_pass() {
    let buono = text(FLAT);
    for damage in [
        ("<offset id=\"1\">", "<offset id=\"1\">9"),
        ("<offset id=\"5\">", "<offset id=\"5\">99999"),
    ] {
        let b = buono.replacen(damage.0, damage.1, 1);
        let mut l = apri_mzxml(b.into_bytes());
        assert_eq!(l.samples()[0].scansioni, 5);
        assert_eq!(l.scan(0, 4).unwrap().mz.len(), 10);
    }
    let outside = buono.replacen("<indexOffset>", "<indexOffset>9999999", 1);
    assert_eq!(apri_mzxml(outside.into_bytes()).samples()[0].scansioni, 5);
}

#[test]
fn a_huge_peaks_count_is_an_error_without_allocating() {
    let b = text(FLAT).replacen("peaksCount=\"30\"", "peaksCount=\"4000000000\"", 1);
    assert_eq!(mzxml_error(b.into_bytes()), "error.memoryBudget");
}

#[test]
fn more_peaks_than_declared_are_cut_or_an_error_if_compressed() {
    let simple = text(Opz {
        zlib: false,
        ..FLAT
    })
    .replacen("peaksCount=\"30\"", "peaksCount=\"4\"", 1);
    assert_eq!(
        apri_mzxml(simple.into_bytes()).scan(0, 0).unwrap().mz.len(),
        4
    );
    let compressed = text(FLAT).replacen("peaksCount=\"30\"", "peaksCount=\"4\"", 1);
    assert_eq!(mzxml_error(compressed.into_bytes()), "error.memoryBudget");
}

#[test]
fn a_zlib_inflating_past_the_declared_size_is_an_error() {
    let b = text(FLAT);
    let da = b.find("compressedLen=\"").unwrap();
    let a = da + b[da..].find('>').unwrap() + 1;
    let fine = a + b[a..].find("</peaks>").unwrap();
    let bomba = b64(&zlib(&vec![0u8; 8 << 20]));
    let b = format!("{}{}{}", &b[..a], bomba, &b[fine..]);
    assert_eq!(mzxml_error(b.into_bytes()), "error.memoryBudget");
}

#[test]
fn broken_base64_unknown_precision_and_truncated_data_are_keyed_errors() {
    let b = text(Opz {
        zlib: false,
        index: false,
        ..FLAT
    });
    let da = b.find("compressedLen=\"0\">").unwrap() + 18;
    let rotta = format!("{}!!!{}", &b[..da], &b[da + 3..]);
    assert_eq!(mzxml_error(rotta.into_bytes()), "err.file.base64");
    let strana = b.replacen("precision=\"64\"", "precision=\"16\"", 1);
    assert_eq!(mzxml_error(strana.into_bytes()), "err.file.xml");
    let order = b.replacen("contentType=\"m/z-int\"", "contentType=\"int-m/z\"", 1);
    assert_eq!(mzxml_error(order.into_bytes()), "err.file.xml");
    // truncated in the middle of the peaks: an error or a short scan, never a panic
    let cut = b[..da + 100].to_string();
    let r = mzxml_error(cut.into_bytes());
    assert!(r == "ok" || r.starts_with("err."), "{r}");
}

#[test]
fn deep_nesting_is_a_keyed_error() {
    let mut b = String::from(TESTATA);
    for k in 0..400 {
        b += &format!(
            "<scan num=\"{}\" msLevel=\"1\" retentionTime=\"PT1S\">",
            k + 1
        );
    }
    b += &"</scan>".repeat(400);
    b += "</msRun></mzXML>";
    assert_eq!(mzxml_error(b.into_bytes()), "err.dam.unreadable");
}

#[test]
fn without_scans_or_with_another_xml_does_not_open() {
    let empty = format!("{}</msRun></mzXML>", HEAD_WITHOUT_SCAN());
    assert_eq!(mzxml_error(empty.into_bytes()), "err.file.xml");
    // named .mzXML but not one: a cut or damaged copy, said with its name and size; an empty one is told apart
    let r = mzlab_lettori::apri(file_set("x.mzXML", b"<altro/>".to_vec()), &Limits::sito());
    let e = r.err().unwrap();
    assert_eq!((e.key, e.params.len()), ("err.file.notThisFormat", 3));
    let r = mzlab_lettori::apri(file_set("x.mzXML", Vec::new()), &Limits::sito());
    assert_eq!(r.err().map(|e| e.key), Some("err.file.empty"));
}

#[allow(non_snake_case)]
fn HEAD_WITHOUT_SCAN() -> &'static str {
    TESTATA
}

#[test]
fn no_file_length_panics_the_reader() {
    let b = mzxml(&run(), Opz { a64: false, ..FLAT });
    for n in (0..b.len()).step_by(97) {
        let _ = mzxml_error(b[..n].to_vec());
    }
}

/// A run of `n` MS1 scans whose `scanCount` says `dichiarate` (`None`: no attribute).
fn long(n: u32, dichiarate: Option<u32>, o: Opz) -> Vec<u8> {
    let v: Vec<Sc> = (1..=n)
        .map(|k| sc(k, 1, 6.0 + k as f64, 30, None))
        .collect();
    let count = dichiarate.map_or(String::new(), |d| format!("scanCount=\"{d}\" "));
    let t = String::from_utf8(mzxml(&v, o)).unwrap();
    t.replacen("scanCount=\"5\" ", &count, 1).into_bytes()
}

/// The scans a damaged copy still gives: each is the same as in the whole file, and with `mancanti` they make up the run.
fn intact_part(whole: &[u8], broken: Vec<u8>, n: usize) -> usize {
    let mut everything = apri_mzxml(whole.to_vec());
    let mut l = apri_mzxml(broken);
    let rows = l.index(0).unwrap().to_vec();
    assert!(rows.len() < n, "a damaged copy reads fewer scans");
    for (k, r) in rows.iter().enumerate() {
        let j = everything
            .index(0)
            .unwrap()
            .iter()
            .position(|v| v.id == r.id)
            .unwrap();
        let (a, b) = (l.scan(0, k).unwrap(), everything.scan(0, j).unwrap());
        assert_eq!((a.mz, a.intensity), (b.mz, b.intensity), "scan {}", r.id);
    }
    let m = l.missing(0);
    assert!(m > 0, "the scans a damaged copy lacks are counted");
    rows.len() + m
}

#[test]
fn a_cut_or_zeroed_copy_says_how_many_scans_are_missing() {
    let n = 40;
    for o in [
        FLAT,
        Opz {
            index: false,
            zlib: false,
            ..FLAT
        },
    ] {
        let b = long(n, Some(n), o);
        assert_eq!(
            apri_mzxml(b.clone()).missing(0),
            0,
            "the whole file lacks nothing"
        );
        for f in [0.5, 0.1] {
            let cut = b[..(b.len() as f64 * f) as usize].to_vec();
            assert_eq!(intact_part(&b, cut, n as usize), n as usize, "cut at {f}");
        }
        // a block of zeros: `scanCount` (or the index after the run) still says how many
        let mut z = b.clone();
        let l = z.len();
        z[l / 3..l / 2].fill(0);
        assert_eq!(intact_part(&b, z, n as usize), n as usize, "zeros");
    }
    // nothing declared: the scan the cut falls in is left out and counted
    let b = long(
        n,
        None,
        Opz {
            index: false,
            ..FLAT
        },
    );
    let da = String::from_utf8_lossy(&b).rfind("<peaks").unwrap();
    assert_eq!(
        intact_part(&b, b[..da + 40].to_vec(), n as usize),
        n as usize
    );
}

// ---- real files (private data repository) ----

/// The pairs (mzXML, mzML of the same raw file) of `dati/formati/aperti/mzXML/`.
fn pairs() -> Vec<(PathBuf, PathBuf)> {
    let Some(d) = dati_percorsi::percorso("dati/formati/aperti/mzXML") else {
        eprintln!("SKIP aperti_mzxml: no data repository (MZLAB_DATI)");
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
        let p = e.path();
        if p.extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("mzxml"))
        {
            let m = ["mzML", "mzml"]
                .iter()
                .map(|x| p.with_extension(x))
                .find(|m| m.exists());
            out.extend(m.map(|m| (p.clone(), m)));
        }
    }
    out
}

fn mzml_spettri(m: &PathBuf) -> Vec<mzlab_core::mzml::Spectrum> {
    let mut v = Vec::new();
    let f = std::fs::File::open(m).unwrap();
    mzlab_core::mzml::parse(std::io::BufReader::new(f), |it| {
        if let mzlab_core::mzml::Item::Spectrum(s) = it {
            v.push(s);
        }
    })
    .unwrap();
    v
}

/// The mzXML of msconvert and its mzML: the same scans, m/z within 1e-6, intensities within 1e-6 (relative), the same RT, level, precursor and polarity.
#[test]
fn every_real_mzxml_says_what_its_mzml_says() {
    for (x, m) in pairs() {
        let name = x.file_name().unwrap().to_string_lossy().into_owned();
        let mut l = apri_mzxml(std::fs::read(&x).unwrap());
        let reference = mzml_spettri(&m);
        assert_eq!(l.samples()[0].scansioni, reference.len(), "{name}");
        for (k, r) in reference.iter().enumerate() {
            let s = l.scan(0, k).unwrap();
            assert_eq!(
                (s.level, s.polarity),
                (r.level, r.polarity),
                "{name} scan {k}"
            );
            assert!(
                (s.rt - r.rt).abs() < 1e-4,
                "{name} scan {k} rt {} {}",
                s.rt,
                r.rt
            );
            assert_eq!(
                s.precursor.is_some(),
                r.precursor.is_some(),
                "{name} scan {k}"
            );
            if let (Some(a), Some(b)) = (s.precursor, r.precursor) {
                assert!((a - b).abs() < 1e-4, "{name} scan {k} precursor");
            }
            assert_eq!(s.mz.len(), r.mz.len(), "{name} scan {k}");
            for i in 0..s.mz.len() {
                assert!(
                    (s.mz[i] - r.mz[i]).abs() <= 1e-6 * r.mz[i].max(1.0),
                    "{name} scan {k} mz {i}"
                );
                let (a, b) = (s.intensity_f64(i), r.intensity_f64(i));
                assert!(
                    (a - b).abs() <= 1e-6 * b.abs().max(1.0),
                    "{name} scan {k} int {i}"
                );
            }
        }
    }
}

/// Every real mzXML of the data repository (no mzML beside it): scans counted, peaks sorted and finite, the header read.
#[test]
fn real_mzxmls_open_and_the_counts_match() {
    let Some(d) = dati_percorsi::percorso("dati/formati/aperti/mzXML") else {
        eprintln!("SKIP aperti_mzxml: no data repository (MZLAB_DATI)");
        return;
    };
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
        let p = e.path();
        if !p
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("mzxml"))
        {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let mut l = apri_mzxml(std::fs::read(&p).unwrap());
        let n = l.samples()[0].scansioni;
        assert!(n > 0, "{name}");
        let attese = [("PXD070375", 1096), ("PXD005665", 5344)];
        if let Some((_, v)) = attese.iter().find(|(k, _)| name.starts_with(k)) {
            assert_eq!(n, *v, "{name}");
        }
        let tic: Vec<f64> = l.index(0).unwrap().iter().map(|v| v.tic).collect();
        for k in (0..n).step_by((n / 20).max(1)) {
            let s = l.scan(0, k).unwrap();
            assert!(s.mz.windows(2).all(|w| w[0] <= w[1]), "{name} scan {k}");
            assert!(
                (0..s.mz.len()).all(|i| s.intensity_f64(i).is_finite() && s.mz[i] > 0.0),
                "{name} scan {k}"
            );
            assert!(tic[k].is_finite() && tic[k] >= 0.0, "{name} scan {k} tic");
        }
        assert!(l.method(0).unwrap().contains("\"mzxml\""), "{name}");
    }
}
