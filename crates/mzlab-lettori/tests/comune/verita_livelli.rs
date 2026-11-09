//! The comparison levels F1-F5 of the Rosina bench (docs/studi/ROSINA.md F.2): one function per level, called by `confronta` in `verita.rs`.
#![allow(dead_code)]

use super::{Outcome, Tol};
use mzlab_core::lettori::{ChromatogramEntry, Reader, ScanEntry};
use mzlab_core::mzml::ChromKind;
use serde_json::Value;

pub fn kind_tipo(k: ChromKind) -> &'static str {
    match k {
        ChromKind::Tic => "tic",
        ChromKind::Bpc => "bpc",
        ChromKind::Srm => "srm",
        ChromKind::Sic => "sim",
        ChromKind::Pda => "pda",
        ChromKind::Pressure => "pressione",
        ChromKind::Analog => "analogico",
        ChromKind::Other => "altro",
    }
}

pub fn num(v: &Value) -> Option<f64> {
    v.as_f64()
}

pub fn rel_ok(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs()).max(1e-300)
}

pub fn level(defects_outcome: Vec<String>, absent: Vec<String>) -> Outcome {
    if !defects_outcome.is_empty() {
        Outcome::Defect(defects_outcome)
    } else if !absent.is_empty() {
        Outcome::Missing(absent)
    } else {
        Outcome::Ok
    }
}

/// F1: scans per level and chromatograms per type, and the ids when both name them the same way.
pub fn f1_counts(index: &[ScanEntry], croms: &[ChromatogramEntry], v: &Value) -> Outcome {
    let sc = v["scansioni"].as_array().cloned().unwrap_or_default();
    let (mut d1, mut m1) = (Vec::new(), Vec::new());
    if index.len() != sc.len() {
        d1.push(format!("scans {} vs {}", index.len(), sc.len()));
    }
    for (k, n) in v["conteggi"]["spettri"].as_object().into_iter().flatten() {
        let Some(lv) = k.strip_prefix("ms").and_then(|x| x.parse::<u8>().ok()) else {
            continue;
        };
        let ours = index.iter().filter(|s| s.livello == lv).count() as u64;
        if Some(ours) != n.as_u64() {
            d1.push(format!("{k}: {ours} vs {n}"));
        }
    }
    for (k, n) in v["conteggi"]["cromatogrammi"]
        .as_object()
        .into_iter()
        .flatten()
    {
        let ours = croms.iter().filter(|x| kind_tipo(x.tipo) == k).count() as u64;
        if Some(ours) != n.as_u64() {
            d1.push(format!("chromatograms {k}: {ours} vs {n}"));
        }
    }
    match (index.first(), sc.first()) {
        (Some(a), Some(b)) if Some(a.id.as_str()) == b["id"].as_str() => {
            for (s, t) in index.iter().zip(&sc) {
                if Some(s.id.as_str()) != t["id"].as_str() {
                    d1.push(format!("id {} vs {}", s.id, t["id"]));
                    break;
                }
            }
        }
        (Some(_), Some(_)) => m1.push("ids (named differently)".into()),
        _ => {}
    }
    level(d1, m1)
}

/// F2 for one scan: RT, level, polarity, precursor (differences in `d`, items the reader cannot give in `m`).
fn f2_scansione(
    k: usize,
    s: &ScanEntry,
    t: &Value,
    tol: Tol,
    d: &mut Vec<String>,
    m: &mut Vec<String>,
) {
    if let Some(rt) = num(&t["rt_min"])
        && (s.rt - rt).abs() > tol.rt_min
    {
        d.push(format!("scan {k}: RT {} vs {rt}", s.rt));
    }
    if t["livello"].as_u64() != Some(u64::from(s.livello)) {
        d.push(format!("scan {k}: level {} vs {}", s.livello, t["livello"]));
    }
    let pol = t["polarita"].as_i64().unwrap_or(0);
    if pol != 0 && s.polarita != 0 && pol != i64::from(s.polarita) {
        d.push(format!("scan {k}: polarity {} vs {pol}", s.polarita));
    } else if pol != 0 && s.polarita == 0 {
        m.push("polarity (reader says none)".into());
    }
    if let Some(p) = num(&t["precursore_mz"]) {
        match s.precursore {
            Some(q) if (q - p).abs() <= tol.mz_da => {}
            other => d.push(format!("scan {k}: precursor {other:?} vs {p}")),
        }
    } else if s.precursore.is_some() && t["livello"].as_u64() == Some(1) {
        d.push(format!("scan {k}: precursor on an MS1 scan"));
    }
}

/// F2 for one scan: collision energy and isolation window, read from the scan header fields.
fn f2_fields(
    l: &mut dyn Reader,
    c: usize,
    k: usize,
    t: &Value,
    d: &mut Vec<String>,
    m: &mut Vec<String>,
) {
    if !(t["ce"].is_number() || t["isolamento"].is_object()) {
        return;
    }
    let fields = l.scan_header(c, k).unwrap_or_default();
    if let Some(ce) = num(&t["ce"]) {
        match fields
            .iter()
            .find(|f| f.cv.as_deref() == Some("MS:1000045"))
            .and_then(|f| f.valore.parse::<f64>().ok())
        {
            Some(x) if (x - ce).abs() <= 1e-4 => {}
            other => d.push(format!("scan {k}: collision energy {other:?} vs {ce}")),
        }
    }
    if t["isolamento"].is_object() && !fields.iter().any(|f| f.cv.as_deref() == Some("MS:1000827"))
    {
        m.push("isolation window".into());
    }
}

/// F2: the header of every scan.
pub fn f2_headers(
    l: &mut dyn Reader,
    index: &[ScanEntry],
    v: &Value,
    c: usize,
    tol: Tol,
) -> Outcome {
    let sc = v["scansioni"].as_array().cloned().unwrap_or_default();
    let (mut d2, mut m2) = (Vec::new(), Vec::new());
    for (k, (s, t)) in index.iter().zip(&sc).enumerate() {
        f2_scansione(k, s, t, tol, &mut d2, &mut m2);
        f2_fields(l, c, k, t, &mut d2, &mut m2);
    }
    m2.sort();
    m2.dedup();
    level(d2, m2)
}

/// F3: the points of the chosen scans.
pub fn f3_points(l: &mut dyn Reader, v: &Value, c: usize, tol: Tol) -> Outcome {
    let (mut d3, mut m3) = (Vec::new(), Vec::new());
    let points = v["punti"].as_object().cloned().unwrap_or_default();
    if points.is_empty() {
        m3.push("no points in the truth".into());
    }
    for (key, p) in &points {
        let Ok(k) = key.parse::<usize>() else {
            continue;
        };
        let Ok(s) = l.scan(c, k) else {
            d3.push(format!("scan {k}: cannot be read"));
            continue;
        };
        let series = |name: &str| -> Vec<f64> {
            p[name]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(num)
                .collect()
        };
        let (mz, it) = (series("mz"), series("intensita"));
        if s.mz.len() != mz.len() {
            d3.push(format!("scan {k}: {} points vs {}", s.mz.len(), mz.len()));
            continue;
        }
        let maximum = it.iter().copied().fold(0.0f64, f64::max).max(1e-300);
        for (i, (&a, &b)) in s.mz.iter().zip(&mz).enumerate() {
            let y = s.intensity_f64(i);
            if !tol.mz_ok(a, b) {
                d3.push(format!("scan {k}[{i}]: m/z {a} vs {b}"));
                break;
            }
            if (y - it[i]).abs() > tol.intensita_rel * maximum {
                d3.push(format!("scan {k}[{i}]: intensity {y} vs {}", it[i]));
                break;
            }
        }
    }
    level(d3, m3)
}

/// F4 for one chromatogram of the truth `t` against the reader's `ch`.
fn f4_trace(
    id: &str,
    t: &Value,
    ch: &mzlab_core::mzml::Chromatogram,
    tol: Tol,
    d: &mut Vec<String>,
) {
    if Some(ch.intensity.len() as u64) != t["n"].as_u64() {
        d.push(format!(
            "chromatogram {id}: {} points vs {}",
            ch.intensity.len(),
            t["n"]
        ));
        return;
    }
    let summed: f64 = ch.intensity.iter().sum();
    let maximum = ch.intensity.iter().copied().fold(0.0f64, f64::max);
    for (name, ours, theirs) in [
        ("sum", summed, num(&t["somma"])),
        ("maximum", maximum, num(&t["massimo"])),
    ] {
        if let Some(b) = theirs
            && !rel_ok(ours, b, tol.cromatogramma_rel)
        {
            d.push(format!("chromatogram {id}: {name} {ours} vs {b}"));
        }
    }
    let n = ch.intensity.len();
    for p in t["campione"].as_array().into_iter().flatten() {
        let (Some(tt), Some(yy)) = (num(&p[0]), num(&p[1])) else {
            continue;
        };
        match ch.time.iter().position(|&x| (x - tt).abs() <= tol.rt_min) {
            Some(i) if i < n && rel_ok(ch.intensity[i], yy, tol.cromatogramma_rel) => {}
            other => d.push(format!(
                "chromatogram {id}: point at {tt} min {other:?} vs {yy}"
            )),
        }
    }
}

/// F4: the chromatograms, by id, else by type and order.
pub fn f4_chromatograms(
    l: &mut dyn Reader,
    croms: &[ChromatogramEntry],
    v: &Value,
    c: usize,
    tol: Tol,
) -> Outcome {
    let (mut d4, mut m4) = (Vec::new(), Vec::new());
    let vc = v["cromatogrammi"].as_array().cloned().unwrap_or_default();
    if vc.is_empty() {
        m4.push("no chromatograms in the truth".into());
    }
    for (j, t) in vc.iter().enumerate() {
        let tipo = t["tipo"].as_str().unwrap_or("altro");
        let id = t["id"].as_str().unwrap_or("");
        let order = vc[..j]
            .iter()
            .filter(|x| x["tipo"].as_str() == Some(tipo))
            .count();
        let entry = croms.iter().find(|x| x.id == id).or_else(|| {
            croms
                .iter()
                .filter(|x| kind_tipo(x.tipo) == tipo)
                .nth(order)
        });
        let Some(voce) = entry else {
            d4.push(format!("chromatogram {id} ({tipo}): not in the reader"));
            continue;
        };
        match l.chromatogram(c, &voce.id) {
            Ok(ch) => f4_trace(id, t, &ch, tol, &mut d4),
            Err(_) => d4.push(format!("chromatogram {id}: cannot be read")),
        }
    }
    level(d4, m4)
}

/// The value at a path like `strumento.serie` or `ms.periodi[0].nome` of a JSON document.
pub fn json_path<'a>(doc: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = doc;
    for seg in path.split('.') {
        let (name, idx) = match seg.split_once('[') {
            Some((n, r)) => (n, r.trim_end_matches(']').parse::<usize>().ok()),
            None => (seg, None),
        };
        if !name.is_empty() {
            cur = cur.get(name)?;
        }
        if let Some(i) = idx {
            cur = cur.get(i)?;
        }
    }
    Some(cur)
}

/// Text normalised for the comparison of F5: trimmed, runs of spaces folded.
pub fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(normalize(s)),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Two timestamps are equal to the second, whatever the zone suffix and the fraction.
fn instant(s: &str) -> String {
    s.trim()
        .trim_end_matches('Z')
        .split('.')
        .next()
        .unwrap_or("")
        .to_string()
}

pub fn metadati(l: &mut dyn Reader, v: &Value, c: usize) -> Outcome {
    let fields = v["metadati"].as_object().cloned().unwrap_or_default();
    if fields.is_empty() {
        return Outcome::Missing(vec!["no fields in the truth".into()]);
    }
    let method: Value = match l.method(c).ok().and_then(|m| serde_json::from_str(&m).ok()) {
        Some(m) => m,
        None => return Outcome::Defect(vec!["metodo/1 cannot be read".into()]),
    };
    let (mut difetti, mut absent) = (Vec::new(), Vec::new());
    for (name, f) in fields {
        let Some(path) = f["metodo"].as_str() else {
            absent.push(format!("{name} (not mapped)"));
            continue;
        };
        let Some(a) = json_path(&method, path).and_then(text) else {
            difetti.push(format!("{name}: {path} is absent"));
            continue;
        };
        let Some(b) = text(&f["valore"]) else {
            continue;
        };
        let uguale = if path.contains("file_creato") || name.contains("TimeStamp") {
            instant(&a) == instant(&b)
        } else {
            a == b
        };
        if !uguale {
            difetti.push(format!("{name}: {path} = {a:?} vs {b:?}"));
        }
    }
    level(difetti, absent)
}
