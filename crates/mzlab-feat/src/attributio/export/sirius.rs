//! Export of the MS/MS data of a campaign for SIRIUS: the native `.ms` text and an MGF it can import, one compound per feature with at least one MS2.
//! Written from the public description of the formats; nothing is read back from SIRIUS. The compound is named after the feature id, never a file or sample.

use crate::attributio::campaign::Campaign;
use crate::attributio::error::Result;
use crate::attributio::export::ctx::gnps_provenance;
use crate::attributio::io::Io;
use crate::attributio::readers::ms2::feature_mgf;
use crate::attributio::readers::mzmine::bracket_adduct;
use crate::attributio::sources::load_sources;
use std::collections::BTreeMap;
use std::fmt::Write;

/// A centroid peak list as `(m/z, intensity)`.
pub type Peaks = Vec<(f64, f64)>;

/// One MS2 spectrum of a feature.
#[derive(Clone, Debug, PartialEq)]
pub struct Ms2 {
    /// Collision energy as the instrument reported it, when known.
    pub collision: Option<f64>,
    pub peaks: Peaks,
}

/// A feature ready to be written; `ionization` and `charge` come from `ionization()`.
#[derive(Clone, Debug, PartialEq)]
pub struct SiriusFeature {
    pub name: String,
    pub mz: f64,
    pub ionization: &'static str,
    pub charge: i32,
    /// Retention time in seconds.
    pub rt_s: Option<f64>,
    pub ms1: Peaks,
    pub ms2: Vec<Ms2>,
}

/// The adducts SIRIUS names the same way, with their charge; anything else has no equivalent and is not guessed.
pub fn ionization(adduct: &str) -> Option<(&'static str, i32)> {
    let a: String = adduct.split_whitespace().collect();
    [
        ("[M+H]+", 1),
        ("[M+Na]+", 1),
        ("[M+NH4]+", 1),
        ("[M-H]-", -1),
        ("[M+Cl]-", -1),
    ]
    .into_iter()
    .find(|(n, _)| *n == a)
}

fn peaks_text(o: &mut String, p: &Peaks) {
    for (m, i) in p {
        let _ = writeln!(o, "{m} {i}");
    }
}

/// The `.ms` text: a block per feature, MS1 first, then a `>collision <energy>` (or `>ms2`) block per MS2.
pub fn write_ms(features: &[SiriusFeature]) -> String {
    let mut o = String::new();
    for f in features {
        let _ = writeln!(o, ">compound {}", f.name);
        let _ = writeln!(o, ">parentmass {}", f.mz);
        let _ = writeln!(o, ">ionization {}", f.ionization);
        let _ = writeln!(o, ">charge {}", f.charge);
        if let Some(rt) = f.rt_s {
            let _ = writeln!(o, ">rt {rt}");
        }
        if !f.ms1.is_empty() {
            o.push_str("\n>ms1\n");
            peaks_text(&mut o, &f.ms1);
        }
        for s in &f.ms2 {
            match s.collision {
                Some(e) => {
                    let _ = writeln!(o, "\n>collision {e}");
                }
                None => o.push_str("\n>ms2\n"),
            }
            peaks_text(&mut o, &s.peaks);
        }
        o.push('\n');
    }
    o
}

fn mgf_block(o: &mut String, f: &SiriusFeature, level: u8, ce: Option<f64>, peaks: &Peaks) {
    o.push_str("BEGIN IONS\n");
    let _ = writeln!(o, "TITLE={} MS{level}", f.name);
    let _ = writeln!(o, "FEATURE_ID={}", f.name);
    let _ = writeln!(o, "PEPMASS={}", f.mz);
    let _ = writeln!(
        o,
        "CHARGE={}{}",
        f.charge.abs(),
        if f.charge > 0 { '+' } else { '-' }
    );
    if let Some(rt) = f.rt_s {
        let _ = writeln!(o, "RTINSECONDS={rt}");
    }
    let _ = writeln!(o, "MSLEVEL={level}");
    let _ = writeln!(o, "ADDUCT={}", f.ionization);
    if let Some(e) = ce {
        let _ = writeln!(o, "COLLISION_ENERGY={e}");
    }
    peaks_text(o, peaks);
    o.push_str("END IONS\n\n");
}

/// The MGF text: a block for the MS1 (when there is one) and one per MS2.
pub fn write_mgf(features: &[SiriusFeature]) -> String {
    let mut o = String::new();
    for f in features {
        if !f.ms1.is_empty() {
            mgf_block(&mut o, f, 1, None, &f.ms1);
        }
        for s in &f.ms2 {
            mgf_block(&mut o, f, 2, s.collision, &s.peaks);
        }
    }
    o
}

#[derive(Default)]
struct Spectra {
    ms1: Peaks,
    ms2: Vec<Ms2>,
}

/// Every block of an MGF grouped by FEATURE_ID: all the MS2 blocks, and the MS1 block with most peaks.
fn read_spectra(text: &str) -> BTreeMap<i64, Spectra> {
    let mut out: BTreeMap<i64, Spectra> = BTreeMap::new();
    let (mut hdr, mut peaks): (BTreeMap<String, String>, Peaks) = Default::default();
    for line in text.lines().map(str::trim) {
        if line == "BEGIN IONS" {
            (hdr, peaks) = Default::default();
        } else if line == "END IONS" {
            let fid = hdr
                .get("FEATURE_ID")
                .and_then(|s| s.trim().parse::<f64>().ok())
                .map(|v| v as i64);
            let level = hdr.get("MSLEVEL").map_or("2", |s| s.trim());
            if let (Some(fid), false) = (fid, peaks.is_empty()) {
                let e = out.entry(fid).or_default();
                let p = std::mem::take(&mut peaks);
                if level == "1" {
                    if p.len() > e.ms1.len() {
                        e.ms1 = p;
                    }
                } else {
                    let collision = hdr
                        .get("COLLISION_ENERGY")
                        .and_then(|s| s.split_whitespace().next())
                        .and_then(|s| s.parse().ok());
                    e.ms2.push(Ms2 {
                        collision,
                        peaks: p,
                    });
                }
            }
        } else if line.contains('=') && !line.starts_with(|c: char| c.is_ascii_digit()) {
            if let Some((k, v)) = line.split_once('=') {
                hdr.insert(k.to_uppercase(), v.to_string());
            }
        } else {
            let mut p = line.split_whitespace().map(|s| s.parse::<f64>().ok());
            if let (Some(Some(m)), Some(Some(i))) = (p.next(), p.next()) {
                peaks.push((m, i));
            }
        }
    }
    out
}

fn left_out(warnings: &mut Vec<String>, why: &str, ids: &[String]) {
    if !ids.is_empty() {
        let shown: Vec<&str> = ids.iter().take(10).map(String::as_str).collect();
        let more = if ids.len() > 10 { ", ..." } else { "" };
        warnings.push(format!(
            "SIRIUS export: {} feature(s) left out, {why} ({}{more})",
            ids.len(),
            shown.join(", ")
        ));
    }
}

/// The features of the campaign with an MS2, an adduct SIRIUS knows and a precursor m/z. The adduct is the one of the library hit of the feature
/// (mzMine, then GNPS2); a feature without one, or with one SIRIUS does not name, is left out with a warning.
pub fn collect(c: &Campaign, io: &Io, warnings: &mut Vec<String>) -> Result<Vec<SiriusFeature>> {
    let src = load_sources(c, io)?;
    let gnps = gnps_provenance(c, io)?;
    let mut out = Vec::new();
    let (mut no_adduct, mut unsupported) = (Vec::new(), Vec::new());
    for p in &src.pols {
        let Some(text) = feature_mgf(c, io, &p.pol).and_then(|path| io.text(&path)) else {
            continue;
        };
        let spectra = read_spectra(&text);
        let (idc, mzc, rtc) = (p.table.col("id"), p.table.col("mz"), p.table.col("rt"));
        let (Some(idc), Some(mzc)) = (idc, mzc) else {
            continue;
        };
        for r in 0..p.table.len() {
            let Some(id) = p.table.int(r, idc) else {
                continue;
            };
            let Some(s) = spectra.get(&id).filter(|s| !s.ms2.is_empty()) else {
                continue;
            };
            let Some(mz) = p.table.num(r, mzc) else {
                continue;
            };
            let name = format!("feature_{}_{id}", p.pol.to_lowercase());
            let adduct = p
                .mzmine
                .iter()
                .find(|h| h.id == id)
                .and_then(|h| h.adduct.clone())
                .or_else(|| {
                    let g = gnps.get(&(p.pol.clone(), id))?;
                    bracket_adduct(g.adduct.as_deref(), &p.pol)
                });
            let Some(adduct) = adduct else {
                no_adduct.push(name);
                continue;
            };
            let Some((ion, charge)) = ionization(&adduct) else {
                unsupported.push(format!("{name} {adduct}"));
                continue;
            };
            out.push(SiriusFeature {
                name,
                mz,
                ionization: ion,
                charge,
                // The feature table keeps the RT in minutes.
                rt_s: rtc.and_then(|k| p.table.num(r, k)).map(|m| m * 60.0),
                ms1: s.ms1.clone(),
                ms2: s.ms2.clone(),
            });
        }
    }
    left_out(warnings, "no adduct is known", &no_adduct);
    left_out(
        warnings,
        "SIRIUS has no equivalent for the adduct",
        &unsupported,
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
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
        assert!(
            blocks[1].contains("COLLISION_ENERGY=20") && blocks[2].contains("COLLISION_ENERGY=40")
        );
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
}
