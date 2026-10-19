//! The comparison levels F1-F5 of the Rosina bench (docs/studi/ROSINA.md F.2): one function per level, called by `confronta` in `verita.rs`.
#![allow(dead_code)]

use super::{Esito, Tol};
use mzlab_core::lettori::{Lettore, VoceCromatogramma, VoceScansione};
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

pub fn livello(esito_difetti: Vec<String>, manca: Vec<String>) -> Esito {
    if !esito_difetti.is_empty() {
        Esito::Difetto(esito_difetti)
    } else if !manca.is_empty() {
        Esito::Manca(manca)
    } else {
        Esito::Ok
    }
}

/// F1: scans per level and chromatograms per type, and the ids when both name them the same way.
pub fn f1_conteggi(indice: &[VoceScansione], croms: &[VoceCromatogramma], v: &Value) -> Esito {
    let sc = v["scansioni"].as_array().cloned().unwrap_or_default();
    let (mut d1, mut m1) = (Vec::new(), Vec::new());
    if indice.len() != sc.len() {
        d1.push(format!("scans {} vs {}", indice.len(), sc.len()));
    }
    for (k, n) in v["conteggi"]["spettri"].as_object().into_iter().flatten() {
        let Some(lv) = k.strip_prefix("ms").and_then(|x| x.parse::<u8>().ok()) else {
            continue;
        };
        let ours = indice.iter().filter(|s| s.livello == lv).count() as u64;
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
    match (indice.first(), sc.first()) {
        (Some(a), Some(b)) if Some(a.id.as_str()) == b["id"].as_str() => {
            for (s, t) in indice.iter().zip(&sc) {
                if Some(s.id.as_str()) != t["id"].as_str() {
                    d1.push(format!("id {} vs {}", s.id, t["id"]));
                    break;
                }
            }
        }
        (Some(_), Some(_)) => m1.push("ids (named differently)".into()),
        _ => {}
    }
    livello(d1, m1)
}

/// F2 for one scan: RT, level, polarity, precursor (differences in `d`, items the reader cannot give in `m`).
fn f2_scansione(
    k: usize,
    s: &VoceScansione,
    t: &Value,
    tol: Tol,
    d: &mut Vec<String>,
    m: &mut Vec<String>,
) {
    if let Some(rt) = num(&t["rt_min"]) {
        if (s.rt - rt).abs() > tol.rt_min {
            d.push(format!("scan {k}: RT {} vs {rt}", s.rt));
        }
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
fn f2_campi(
    l: &mut dyn Lettore,
    c: usize,
    k: usize,
    t: &Value,
    d: &mut Vec<String>,
    m: &mut Vec<String>,
) {
    if !(t["ce"].is_number() || t["isolamento"].is_object()) {
        return;
    }
    let campi = l.intestazione(c, k).unwrap_or_default();
    if let Some(ce) = num(&t["ce"]) {
        match campi
            .iter()
            .find(|f| f.cv.as_deref() == Some("MS:1000045"))
            .and_then(|f| f.valore.parse::<f64>().ok())
        {
            Some(x) if (x - ce).abs() <= 1e-4 => {}
            other => d.push(format!("scan {k}: collision energy {other:?} vs {ce}")),
        }
    }
    if t["isolamento"].is_object() && !campi.iter().any(|f| f.cv.as_deref() == Some("MS:1000827")) {
        m.push("isolation window".into());
    }
}

/// F2: the header of every scan.
pub fn f2_intestazioni(
    l: &mut dyn Lettore,
    indice: &[VoceScansione],
    v: &Value,
    c: usize,
    tol: Tol,
) -> Esito {
    let sc = v["scansioni"].as_array().cloned().unwrap_or_default();
    let (mut d2, mut m2) = (Vec::new(), Vec::new());
    for (k, (s, t)) in indice.iter().zip(&sc).enumerate() {
        f2_scansione(k, s, t, tol, &mut d2, &mut m2);
        f2_campi(l, c, k, t, &mut d2, &mut m2);
    }
    m2.sort();
    m2.dedup();
    livello(d2, m2)
}

/// F3: the points of the chosen scans.
pub fn f3_punti(l: &mut dyn Lettore, v: &Value, c: usize, tol: Tol) -> Esito {
    let (mut d3, mut m3) = (Vec::new(), Vec::new());
    let punti = v["punti"].as_object().cloned().unwrap_or_default();
    if punti.is_empty() {
        m3.push("no points in the truth".into());
    }
    for (chiave, p) in &punti {
        let Ok(k) = chiave.parse::<usize>() else {
            continue;
        };
        let Ok(s) = l.scansione(c, k) else {
            d3.push(format!("scan {k}: cannot be read"));
            continue;
        };
        let serie = |nome: &str| -> Vec<f64> {
            p[nome]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(num)
                .collect()
        };
        let (mz, it) = (serie("mz"), serie("intensita"));
        if s.mz.len() != mz.len() {
            d3.push(format!("scan {k}: {} points vs {}", s.mz.len(), mz.len()));
            continue;
        }
        let massimo = it.iter().cloned().fold(0.0f64, f64::max).max(1e-300);
        for (i, (&a, &b)) in s.mz.iter().zip(&mz).enumerate() {
            let y = s.intensity_f64(i);
            if !tol.mz_ok(a, b) {
                d3.push(format!("scan {k}[{i}]: m/z {a} vs {b}"));
                break;
            }
            if (y - it[i]).abs() > tol.intensita_rel * massimo {
                d3.push(format!("scan {k}[{i}]: intensity {y} vs {}", it[i]));
                break;
            }
        }
    }
    livello(d3, m3)
}

/// F4 for one chromatogram of the truth `t` against the reader's `ch`.
fn f4_traccia(
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
    let somma: f64 = ch.intensity.iter().sum();
    let massimo = ch.intensity.iter().cloned().fold(0.0f64, f64::max);
    for (nome, ours, theirs) in [
        ("sum", somma, num(&t["somma"])),
        ("maximum", massimo, num(&t["massimo"])),
    ] {
        if let Some(b) = theirs {
            if !rel_ok(ours, b, tol.cromatogramma_rel) {
                d.push(format!("chromatogram {id}: {nome} {ours} vs {b}"));
            }
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
pub fn f4_cromatogrammi(
    l: &mut dyn Lettore,
    croms: &[VoceCromatogramma],
    v: &Value,
    c: usize,
    tol: Tol,
) -> Esito {
    let (mut d4, mut m4) = (Vec::new(), Vec::new());
    let vc = v["cromatogrammi"].as_array().cloned().unwrap_or_default();
    if vc.is_empty() {
        m4.push("no chromatograms in the truth".into());
    }
    for (j, t) in vc.iter().enumerate() {
        let tipo = t["tipo"].as_str().unwrap_or("altro");
        let id = t["id"].as_str().unwrap_or("");
        let ordine = vc[..j]
            .iter()
            .filter(|x| x["tipo"].as_str() == Some(tipo))
            .count();
        let voce = croms.iter().find(|x| x.id == id).or_else(|| {
            croms
                .iter()
                .filter(|x| kind_tipo(x.tipo) == tipo)
                .nth(ordine)
        });
        let Some(voce) = voce else {
            d4.push(format!("chromatogram {id} ({tipo}): not in the reader"));
            continue;
        };
        match l.cromatogramma(c, &voce.id) {
            Ok(ch) => f4_traccia(id, t, &ch, tol, &mut d4),
            Err(_) => d4.push(format!("chromatogram {id}: cannot be read")),
        }
    }
    livello(d4, m4)
}

/// The value at a path like `strumento.serie` or `ms.periodi[0].nome` of a JSON document.
pub fn percorso_json<'a>(doc: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = doc;
    for seg in path.split('.') {
        let (nome, idx) = match seg.split_once('[') {
            Some((n, r)) => (n, r.trim_end_matches(']').parse::<usize>().ok()),
            None => (seg, None),
        };
        if !nome.is_empty() {
            cur = cur.get(nome)?;
        }
        if let Some(i) = idx {
            cur = cur.get(i)?;
        }
    }
    Some(cur)
}

/// Text normalised for the comparison of F5: trimmed, runs of spaces folded.
pub fn normalizza(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn testo(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(normalizza(s)),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Two timestamps are equal to the second, whatever the zone suffix and the fraction.
fn istante(s: &str) -> String {
    s.trim()
        .trim_end_matches('Z')
        .split('.')
        .next()
        .unwrap_or("")
        .to_string()
}

pub fn metadati(l: &mut dyn Lettore, v: &Value, c: usize) -> Esito {
    let campi = v["metadati"].as_object().cloned().unwrap_or_default();
    if campi.is_empty() {
        return Esito::Manca(vec!["no fields in the truth".into()]);
    }
    let metodo: Value = match l.metodo(c).ok().and_then(|m| serde_json::from_str(&m).ok()) {
        Some(m) => m,
        None => return Esito::Difetto(vec!["metodo/1 cannot be read".into()]),
    };
    let (mut difetti, mut manca) = (Vec::new(), Vec::new());
    for (nome, f) in campi {
        let Some(path) = f["metodo"].as_str() else {
            manca.push(format!("{nome} (not mapped)"));
            continue;
        };
        let Some(a) = percorso_json(&metodo, path).and_then(testo) else {
            difetti.push(format!("{nome}: {path} is absent"));
            continue;
        };
        let Some(b) = testo(&f["valore"]) else {
            continue;
        };
        let uguale = if path.contains("file_creato") || nome.contains("TimeStamp") {
            istante(&a) == istante(&b)
        } else {
            a == b
        };
        if !uguale {
            difetti.push(format!("{nome}: {path} = {a:?} vs {b:?}"));
        }
    }
    livello(difetti, manca)
}
