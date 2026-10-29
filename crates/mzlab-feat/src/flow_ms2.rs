//! The text of `spectra.ms2@1`: the MS2 scans of mzML files (DDA) as an MGF `spectra` file, with the precursor, charge, RT and the precursor intensity of every scan; with
//! `dia_pseudo` the pseudo spectra of the features of a table (`dia`). Takes names and bytes and returns the text: `flow run` (`mzlab-cli`) and the browser (`mzlab-feat-wasm`)
//! run this one function. The MGF feeds `spectra.match` and `formula.ms2`.

use crate::dia::{pseudo_ms2, DiaParams, DiaScan, Prec};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;
use mzlab_core::mzml::{self, Spectrum};

/// A file the step reads: its name and its bytes.
pub type Named<'a> = (&'a str, &'a [u8]);

/// The feature table as the CSV "full": header and rows; the area columns are `datafile:NAME:area`.
pub struct Table {
    pub id: usize,
    pub mz: usize,
    pub rt: usize,
    pub areas: Vec<(String, usize)>,
    pub rows: Vec<Vec<String>>,
}

pub fn read_table(bytes: &[u8]) -> Result<Table> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap_or("").split(',').collect();
    let col = |n: &str| {
        head.iter()
            .position(|h| *h == n)
            .ok_or_else(|| Error::new("err.flow.formula.table").with("column", n))
    };
    let areas = head
        .iter()
        .enumerate()
        .filter_map(|(k, h)| {
            h.strip_prefix("datafile:")?
                .strip_suffix(":area")
                .map(|n| (n.to_string(), k))
        })
        .collect();
    Ok(Table {
        id: col("id")?,
        mz: col("mz")?,
        rt: col("rt")?,
        areas,
        rows: lines
            .map(|l| l.split(',').map(String::from).collect())
            .collect(),
    })
}

/// The parameters of the step that change the text; a missing one has the default of the descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct Ms2Params {
    pub min_intensity: f64,
    pub max_isolation: Option<f64>,
    pub tol_da: f64,
    pub rt_tol: f64,
    pub dia_pseudo: bool,
    pub dia_min_corr: f64,
    pub dia_min_points: f64,
    pub dia_frag_ppm: f64,
}

impl Ms2Params {
    /// From the resolved parameters of the step (`Job::resolved.params`, or the parameters the page merged with the defaults).
    pub fn from_params(params: &[(String, Value)]) -> Ms2Params {
        let get = |name: &str| params.iter().find(|(k, _)| k == name).map(|(_, v)| v);
        let num = |name: &str| get(name).and_then(Value::f64);
        Ms2Params {
            min_intensity: num("min_intensity").unwrap_or(0.0),
            max_isolation: num("max_isolation"),
            tol_da: num("tol_da").unwrap_or(0.02),
            rt_tol: num("rt_tol").unwrap_or(0.2),
            dia_pseudo: get("dia_pseudo").and_then(Value::bool) == Some(true),
            dia_min_corr: num("dia_min_corr").unwrap_or(0.9),
            dia_min_points: num("dia_min_points").unwrap_or(5.0),
            dia_frag_ppm: num("dia_frag_ppm").unwrap_or(10.0),
        }
    }
}

impl Ms2Params {
    /// From the object the page sends: the parameters of the step merged with the defaults of its descriptor.
    pub fn from_json(text: &str) -> Result<Ms2Params> {
        match mzlab_core::json::parse(text.as_bytes())? {
            Value::Obj(o) => Ok(Ms2Params::from_params(&o)),
            _ => Ok(Ms2Params::from_params(&[])),
        }
    }
}

fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |p| p.0)
}

/// What the text of a spectrum says that `mzml::Spectrum` does not keep.
#[derive(Default, Clone, Copy)]
struct Extra {
    charge: Option<i64>,
    width: Option<f64>,
    /// Isolation window `(low, high)` in Da.
    iso: Option<(f64, f64)>,
    prec_int: Option<f64>,
}

/// The value of the cvParam `acc` inside a `<spectrum>` block.
fn cv_value(block: &str, acc: &str) -> Option<f64> {
    let at = block.find(&format!("accession=\"{acc}\""))?;
    let start = block[..at].rfind('<')?;
    let tag = &block[start..start + block[start..].find('>')?];
    let v = tag.split_once("value=\"")?.1;
    v.split('"').next()?.parse().ok()
}

/// Extras of every spectrum in file order (`<spectrum ` opens one; `<spectrumList` does not match).
fn extras(text: &str) -> Vec<Extra> {
    text.split("<spectrum ")
        .skip(1)
        .map(|b| {
            let b = b.split("</spectrumList").next().unwrap_or(b);
            let width = cv_value(b, "MS:1000828").zip(cv_value(b, "MS:1000829"));
            let iso = cv_value(b, "MS:1000827")
                .zip(width)
                .map(|(t, (lo, hi))| (t - lo, t + hi));
            Extra {
                charge: cv_value(b, "MS:1000041").map(|c| c as i64),
                width: width.map(|(lo, hi)| lo + hi),
                iso,
                prec_int: cv_value(b, "MS:1000042"),
            }
        })
        .collect()
}

fn record(
    out: &mut String,
    (title, id): (&str, &str),
    s: &Spectrum,
    e: Extra,
    peaks: &[(f64, f64)],
) {
    let (sign, mode) = match s.polarity {
        1 => ('+', "Positive"),
        -1 => ('-', "Negative"),
        _ => ('+', ""),
    };
    out.push_str(&format!(
        "BEGIN IONS\nTITLE={title}\nSPECTRUMID={id}\nPEPMASS={:.5}",
        s.precursor.unwrap_or(0.0)
    ));
    if let Some(i) = e.prec_int {
        out.push_str(&format!(" {i:.1}"));
    }
    out.push_str(&format!(
        "\nRTINSECONDS={:.3}\nSCANS={}\nMSLEVEL=2\n",
        s.rt * 60.0,
        s.index + 1
    ));
    if id != title {
        out.push_str(&format!("FEATURE_ID={id}\n"));
    }
    if let Some(c) = e.charge.filter(|c| *c > 0) {
        out.push_str(&format!("CHARGE={c}{sign}\n"));
    }
    if !mode.is_empty() {
        out.push_str(&format!("IONMODE={mode}\n"));
    }
    for (m, i) in peaks {
        out.push_str(&format!("{m:.5} {i:.1}\n"));
    }
    out.push_str("END IONS\n");
}

/// The id of the feature of the table that a scan belongs to: present in this file (area above zero), RT and precursor within the tolerances, the closest
/// in m/z (the first row on a tie).
fn feature_of(t: &Table, file: &str, s: &Spectrum, tol: (f64, f64)) -> Option<String> {
    let prec = s.precursor?;
    let area = t.areas.iter().find(|a| a.0 == file).map(|a| a.1);
    let num = |row: &Vec<String>, k: usize| row.get(k).and_then(|v| v.parse::<f64>().ok());
    let near = t.rows.iter().filter(|r| {
        let (mz, rt) = (num(r, t.mz), num(r, t.rt));
        mz.zip(rt)
            .is_some_and(|(mz, rt)| (mz - prec).abs() <= tol.0 && (rt - s.rt).abs() <= tol.1)
            && area.is_none_or(|k| num(r, k).is_some_and(|a| a > 0.0))
    });
    let best = near.min_by(|a, b| {
        let d = |r: &Vec<String>| (num(r, t.mz).unwrap_or(0.0) - prec).abs();
        d(a).total_cmp(&d(b))
    });
    best.and_then(|r| r.get(t.id)).cloned()
}

/// Runs the step of `job` on the mzML files and returns the MGF text: files in the given order, scans in file order, `TITLE` `<file>.<scan>`.
/// `SPECTRUMID` (the query id of `spectra.match`) is the same, or with a feature table the id of the feature of the scan, and the scans of no feature go.
pub fn ms2_text(p: &Ms2Params, files: &[Named], features: Option<&[u8]>) -> Result<String> {
    let floor = p.min_intensity;
    let widest = p.max_isolation;
    let tol = (p.tol_da, p.rt_tol);
    let table = features.map(read_table).transpose()?;
    if p.dia_pseudo {
        let table =
            table.ok_or_else(|| Error::new("err.flow.port.missing").with("port", "features"))?;
        return dia_text(p, files, &table, tol.1);
    }
    let mut out = String::new();
    for (name, bytes) in files {
        let run = mzml::read(*bytes)?;
        let text = String::from_utf8_lossy(bytes);
        let extra = extras(&text);
        let extra = if extra.len() == run.spectra.len() {
            extra
        } else {
            vec![Extra::default(); run.spectra.len()]
        };
        for (s, e) in run.spectra.iter().zip(extra) {
            if s.level != 2 || s.precursor.is_none() {
                continue;
            }
            if widest.zip(e.width).is_some_and(|(max, w)| w > max) {
                continue;
            }
            let title = format!("{}.{}", stem(name), s.index + 1);
            let id = match &table {
                Some(t) => match feature_of(t, stem(name), s, tol) {
                    Some(id) => id,
                    None => continue,
                },
                None => title.clone(),
            };
            let peaks: Vec<(f64, f64)> = (0..s.mz.len())
                .map(|i| (s.mz[i], s.intensity_f64(i)))
                .filter(|p| p.1 >= floor && p.1 > 0.0)
                .collect();
            if !peaks.is_empty() {
                record(&mut out, (&title, &id), s, e, &peaks);
            }
        }
    }
    if out.is_empty() && files.is_empty() {
        return Err(Error::new("err.flow.port.missing").with("port", "source"));
    }
    Ok(out)
}

/// The features of the table present in the file `file` (area above zero) as precursors with the RT range `rt ± rt_tol`, and their m/z.
fn precursors(t: &Table, file: &str, rt_tol: f64) -> (Vec<Prec>, Vec<f64>) {
    let num = |row: &Vec<String>, k: usize| row.get(k).and_then(|v| v.parse::<f64>().ok());
    let area = t.areas.iter().find(|a| a.0 == file).map(|a| a.1);
    let (mut precs, mut mzs) = (Vec::new(), Vec::new());
    for r in &t.rows {
        let (Some(mz), Some(rt), Some(id)) = (num(r, t.mz), num(r, t.rt), r.get(t.id)) else {
            continue;
        };
        if area.is_some_and(|k| !num(r, k).is_some_and(|a| a > 0.0)) {
            continue;
        }
        precs.push(Prec {
            id: id.clone(),
            mz,
            rt_lo_s: (rt - rt_tol) * 60.0,
            rt_hi_s: (rt + rt_tol) * 60.0,
        });
        mzs.push(mz);
    }
    (precs, mzs)
}

/// With `dia_pseudo`: one record per feature with a pseudo spectrum (`dia::pseudo_ms2`), `TITLE` `<file>.<id>.pseudo`, `PEPMASS` the m/z of the feature,
/// `RTINSECONDS` the apex. A feature takes part for a file where its area is above zero.
fn dia_text(m: &Ms2Params, files: &[Named], t: &Table, rt_tol: f64) -> Result<String> {
    let p = DiaParams {
        min_corr: m.dia_min_corr,
        min_points: m.dia_min_points as usize,
        frag_ppm: m.dia_frag_ppm,
        ..DiaParams::default()
    };
    let mut out = String::new();
    for (name, bytes) in files {
        let run = mzml::read(*bytes)?;
        let text = String::from_utf8_lossy(bytes);
        let extra = extras(&text);
        let extra = if extra.len() == run.spectra.len() {
            extra
        } else {
            vec![Extra::default(); run.spectra.len()]
        };
        let ints: Vec<Vec<f64>> = run
            .spectra
            .iter()
            .map(|s| (0..s.mz.len()).map(|i| s.intensity_f64(i)).collect())
            .collect();
        let scans: Vec<DiaScan> = run
            .spectra
            .iter()
            .zip(&extra)
            .zip(&ints)
            .filter(|((s, _), _)| s.level == 1 || s.level == 2)
            .map(|((s, e), int)| DiaScan {
                rt_s: s.rt * 60.0,
                level: s.level,
                iso: if s.level == 2 { e.iso } else { None },
                mz: &s.mz,
                int,
            })
            .collect();
        let (precs, mzs) = precursors(t, stem(name), rt_tol);
        for ps in pseudo_ms2(&scans, &precs, &p) {
            let Some(k) = precs.iter().position(|q| q.id == ps.id) else {
                continue;
            };
            let at = run
                .spectra
                .iter()
                .find(|s| s.level == 2 && s.rt * 60.0 == ps.apex_rt_s);
            let head = Spectrum {
                index: at.map_or(0, |s| s.index),
                level: 2,
                rt: ps.apex_rt_s / 60.0,
                polarity: at.map_or(0, |s| s.polarity),
                precursor: Some(mzs[k]),
                ..Spectrum::default()
            };
            let peaks: Vec<(f64, f64)> = ps.peaks.iter().map(|q| (q.0, q.1)).collect();
            let title = format!("{}.{}.pseudo", stem(name), ps.id);
            record(&mut out, (&title, &ps.id), &head, Extra::default(), &peaks);
        }
    }
    if out.is_empty() && files.is_empty() {
        return Err(Error::new("err.flow.port.missing").with("port", "source"));
    }
    Ok(out)
}
