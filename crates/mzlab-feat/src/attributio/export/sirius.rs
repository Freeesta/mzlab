//! Export of the MS/MS data of a campaign for SIRIUS: the native `.ms` text and an MGF it can import, one compound per feature with at least one MS2.
//! Written from the public description of the formats; nothing is read back from SIRIUS. The compound is named after the feature id, never a file or sample.

use crate::attributio::campaign::Campaign;
use crate::attributio::error::Result;
use crate::attributio::export::ctx::{gnps_provenance, GnpsProv};
use crate::attributio::io::Io;
use crate::attributio::readers::ms2::feature_mgf;
use crate::attributio::readers::mzmine::bracket_adduct;
use crate::attributio::sources::{load_sources, PolSources};
use std::collections::{BTreeMap, HashMap};
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

/// The adduct of a feature: the one of its library hit (mzMine, then GNPS2).
fn adduct_of(p: &PolSources, gnps: &HashMap<(String, i64), GnpsProv>, id: i64) -> Option<String> {
    p.mzmine
        .iter()
        .find(|h| h.id == id)
        .and_then(|h| h.adduct.clone())
        .or_else(|| {
            let g = gnps.get(&(p.pol.clone(), id))?;
            bracket_adduct(g.adduct.as_deref(), &p.pol)
        })
}

/// Features the export could not write, by reason.
#[derive(Default)]
struct LeftOut {
    no_adduct: Vec<String>,
    unsupported: Vec<String>,
}

/// The features of one polarity that have an MS2, an adduct SIRIUS names and a precursor m/z.
fn collect_pol(
    p: &PolSources,
    spectra: &BTreeMap<i64, Spectra>,
    gnps: &HashMap<(String, i64), GnpsProv>,
    left: &mut LeftOut,
    out: &mut Vec<SiriusFeature>,
) {
    let (Some(idc), Some(mzc)) = (p.table.col("id"), p.table.col("mz")) else {
        return;
    };
    let rtc = p.table.col("rt");
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
        let Some(adduct) = adduct_of(p, gnps, id) else {
            left.no_adduct.push(name);
            continue;
        };
        let Some((ion, charge)) = ionization(&adduct) else {
            left.unsupported.push(format!("{name} {adduct}"));
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

/// The features of the campaign with an MS2, an adduct SIRIUS knows and a precursor m/z. A feature without an adduct, or with one SIRIUS does not name,
/// is left out with a warning.
pub fn collect(c: &Campaign, io: &Io, warnings: &mut Vec<String>) -> Result<Vec<SiriusFeature>> {
    let src = load_sources(c, io)?;
    let gnps = gnps_provenance(c, io)?;
    let mut out = Vec::new();
    let mut left = LeftOut::default();
    for p in &src.pols {
        if let Some(text) = feature_mgf(c, io, &p.pol).and_then(|path| io.text(&path)) {
            collect_pol(p, &read_spectra(&text), &gnps, &mut left, &mut out);
        }
    }
    left_out(warnings, "no adduct is known", &left.no_adduct);
    left_out(
        warnings,
        "SIRIUS has no equivalent for the adduct",
        &left.unsupported,
    );
    Ok(out)
}

#[cfg(test)]
#[path = "sirius_tests.rs"]
mod tests;
