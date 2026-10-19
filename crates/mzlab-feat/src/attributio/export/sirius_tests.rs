//! Unit tests of the SIRIUS export (`sirius.rs`).

use super::*;

fn sample() -> Vec<SiriusFeature> {
    vec![
        SiriusFeature {
            name: "feature_pos_7".into(),
            mz: 285.0763,
            ionization: "[M+H]+",
            charge: 1,
            rt_s: Some(154.2),
            ms1: vec![(285.0763, 1000.5), (286.0797, 120.25)],
            ms2: vec![
                Ms2 {
                    collision: Some(20.0),
                    peaks: vec![(100.1, 5.0), (150.25, 80.0)],
                },
                Ms2 {
                    collision: Some(40.0),
                    peaks: vec![(91.05, 33.0)],
                },
            ],
        },
        SiriusFeature {
            name: "feature_neg_9".into(),
            mz: 213.0123456,
            ionization: "[M-H]-",
            charge: -1,
            rt_s: None,
            ms1: vec![],
            ms2: vec![Ms2 {
                collision: None,
                peaks: vec![(79.9568, 12.0)],
            }],
        },
    ]
}

/// Reads a `.ms` back into (name, mass, ionization, charge, rt, ms1, [(collision, peaks)]).
#[allow(clippy::type_complexity)]
fn read_ms(
    t: &str,
) -> Vec<(
    String,
    f64,
    String,
    i32,
    Option<f64>,
    Peaks,
    Vec<(Option<f64>, Peaks)>,
)> {
    let mut v = Vec::new();
    let mut mode = 0;
    for l in t.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(n) = l.strip_prefix(">compound ") {
            v.push((n.to_string(), 0.0, String::new(), 0, None, vec![], vec![]));
        } else if let Some(x) = l.strip_prefix(">parentmass ") {
            v.last_mut().unwrap().1 = x.parse().unwrap();
        } else if let Some(x) = l.strip_prefix(">ionization ") {
            v.last_mut().unwrap().2 = x.to_string();
        } else if let Some(x) = l.strip_prefix(">charge ") {
            v.last_mut().unwrap().3 = x.parse().unwrap();
        } else if let Some(x) = l.strip_prefix(">rt ") {
            v.last_mut().unwrap().4 = Some(x.parse().unwrap());
        } else if l == ">ms1" {
            mode = 1;
        } else if l == ">ms2" {
            mode = 2;
            v.last_mut().unwrap().6.push((None, vec![]));
        } else if let Some(x) = l.strip_prefix(">collision ") {
            mode = 2;
            v.last_mut()
                .unwrap()
                .6
                .push((Some(x.parse().unwrap()), vec![]));
        } else {
            let mut p = l.split_whitespace().map(|s| s.parse::<f64>().unwrap());
            let pk = (p.next().unwrap(), p.next().unwrap());
            let last = v.last_mut().unwrap();
            if mode == 1 {
                last.5.push(pk);
            } else {
                last.6.last_mut().unwrap().1.push(pk);
            }
        }
    }
    v
}

#[test]
fn ms_roundtrip() {
    let f = sample();
    let back = read_ms(&write_ms(&f));
    assert_eq!(back.len(), 2);
    for (b, f) in back.iter().zip(&f) {
        assert_eq!(b.0, f.name);
        assert!((b.1 - f.mz).abs() < 1e-9);
        assert_eq!((b.2.as_str(), b.3), (f.ionization, f.charge));
        assert_eq!(b.4.is_some(), f.rt_s.is_some());
        if let (Some(a), Some(e)) = (b.4, f.rt_s) {
            assert!((a - e).abs() < 1e-9);
        }
        assert_eq!(b.5.len(), f.ms1.len());
        assert_eq!(b.6.len(), f.ms2.len());
        for (s, e) in b.6.iter().zip(&f.ms2) {
            assert_eq!(s.0, e.collision);
            assert_eq!(s.1.len(), e.peaks.len());
            for (x, y) in s.1.iter().zip(&e.peaks) {
                assert!((x.0 - y.0).abs() < 1e-9 && (x.1 - y.1).abs() < 1e-9);
            }
        }
    }
    assert_eq!(back[1].6[0].0, None);
}

#[test]
fn mgf_roundtrip() {
    let f = sample();
    let text = write_mgf(&f);
    // 1 MS1 + 2 MS2 for the first feature, 1 MS2 for the second.
    assert_eq!(text.matches("BEGIN IONS").count(), 4);
    assert_eq!(text.matches("END IONS").count(), 4);
    assert!(text.contains("CHARGE=1+\n") && text.contains("CHARGE=1-\n"));
    let first = text.split("END IONS").next().unwrap();
    assert!(first.contains("MSLEVEL=1") && first.contains("PEPMASS=285.0763"));
    assert!(first.contains("RTINSECONDS=154.2"));
    let blocks: Vec<&str> = text.split("END IONS").collect();
    assert!(blocks[1].contains("COLLISION_ENERGY=20") && blocks[2].contains("COLLISION_ENERGY=40"));
    assert!(!blocks[3].contains("COLLISION_ENERGY") && !blocks[3].contains("RTINSECONDS"));
    let mut n = 0;
    for b in &blocks[..4] {
        for l in b
            .lines()
            .filter(|l| l.starts_with(|c: char| c.is_ascii_digit()) && !l.contains('='))
        {
            let mut p = l.split_whitespace().map(|s| s.parse::<f64>().unwrap());
            let (m, i) = (p.next().unwrap(), p.next().unwrap());
            let all: Vec<(f64, f64)> = f
                .iter()
                .flat_map(|x| {
                    x.ms1
                        .iter()
                        .chain(x.ms2.iter().flat_map(|s| s.peaks.iter()))
                })
                .copied()
                .collect();
            assert!(all
                .iter()
                .any(|(a, b)| (a - m).abs() < 1e-9 && (b - i).abs() < 1e-9));
            n += 1;
        }
    }
    assert_eq!(n, 2 + 2 + 1 + 1);
}

#[test]
fn adducts() {
    assert_eq!(ionization("[M + Na]+"), Some(("[M+Na]+", 1)));
    assert_eq!(ionization("[M+Cl]-"), Some(("[M+Cl]-", -1)));
    assert_eq!(ionization("[M+HCOO]-"), None);
    assert_eq!(ionization("[M+2H]2+"), None);
}

#[test]
fn reads_every_block_of_a_feature() {
    let t = "BEGIN IONS\nFEATURE_ID=5\nMSLEVEL=1\n100.0 10\nEND IONS\n\
BEGIN IONS\nFEATURE_ID=5\nMSLEVEL=2\nCOLLISION_ENERGY=30\n50.0 1\n60 2\nEND IONS\n\
BEGIN IONS\nFEATURE_ID=5\nMSLEVEL=2\n70.0 3\nEND IONS\n\
BEGIN IONS\nFEATURE_ID=6\nMSLEVEL=1\n100.0 10\nEND IONS\n";
    let s = read_spectra(t);
    assert_eq!(s[&5].ms2.len(), 2);
    assert_eq!(s[&5].ms2[0].collision, Some(30.0));
    assert_eq!(s[&5].ms2[1].collision, None);
    assert_eq!(s[&5].ms1.len(), 1);
    // A feature with MS1 only has no MS2 and is skipped by `collect`.
    assert!(s[&6].ms2.is_empty());
}

#[test]
fn no_file_or_sample_names() {
    let f = sample();
    for t in [write_ms(&f), write_mgf(&f)] {
        for bad in [
            ".mzML",
            ".mzml",
            ".raw",
            "sample",
            "FILENAME",
            "SOURCE_SCAN",
        ] {
            assert!(!t.contains(bad), "{bad}");
        }
    }
}
