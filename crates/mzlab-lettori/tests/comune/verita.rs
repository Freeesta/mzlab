//! The Rosina test bench (docs/studi/ROSINA.md F): compares a reader with the ground truth `verita/1` of an external oracle, levels F0-F5, and reports per file `L`, `P` or `NV`.
//! Included with `#[path = "comune/verita.rs"] mod verita;` by `verita_banco.rs` (the bench on built files) and by `verita_<famiglia>.rs` (the real corpus, skipped without data).
#![allow(dead_code)]

use mzlab_core::lettori::{Lettore, VoceScansione};
use mzlab_core::mzml::ChromKind;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Tolerances of F.2 (docs/studi/ROSINA.md): m/z by ppm (high resolution) or by Da (unit resolution), times in minutes, intensities relative.
#[derive(Clone, Copy)]
pub struct Tol {
    pub mz_ppm: Option<f64>,
    pub mz_da: f64,
    pub rt_min: f64,
    pub intensita_rel: f64,
    pub cromatogramma_rel: f64,
}

impl Tol {
    pub const HR: Tol = Tol {
        mz_ppm: Some(0.5),
        mz_da: 1e-4,
        rt_min: 1e-4,
        intensita_rel: 1e-5,
        cromatogramma_rel: 1e-6,
    };
    pub const LR: Tol = Tol {
        mz_ppm: None,
        mz_da: 1e-3,
        rt_min: 1e-4,
        intensita_rel: 1e-5,
        cromatogramma_rel: 1e-6,
    };
    fn mz_ok(&self, a: f64, b: f64) -> bool {
        match self.mz_ppm {
            Some(p) => (a - b).abs() <= b.abs() * p * 1e-6,
            None => (a - b).abs() <= self.mz_da,
        }
    }
}

/// What one level found: everything compared and equal, items the truth or the reader cannot give (named), or differences.
#[derive(Debug, Clone, PartialEq)]
pub enum Esito {
    Ok,
    Manca(Vec<String>),
    Difetto(Vec<String>),
}

pub const LIVELLI: [&str; 6] = ["F0", "F1", "F2", "F3", "F4", "F5"];

#[derive(Debug)]
pub struct Rapporto {
    pub file: String,
    pub livelli: Vec<(&'static str, Esito)>,
}

impl Rapporto {
    pub fn difetti(&self) -> Vec<String> {
        self.livelli
            .iter()
            .filter_map(|(l, e)| match e {
                Esito::Difetto(d) => {
                    Some(d.iter().map(|x| format!("{l}: {x}")).collect::<Vec<_>>())
                }
                _ => None,
            })
            .flatten()
            .collect()
    }
    /// The levels that could not be compared (with what is missing).
    pub fn mancanti(&self) -> Vec<String> {
        self.livelli
            .iter()
            .filter_map(|(l, e)| match e {
                Esito::Manca(m) => Some(format!("{l} ({})", m.join(", "))),
                _ => None,
            })
            .collect()
    }
    /// `L`: F0-F5 compared and equal; `P`: equal where compared, some level missing; `NV`: only F0 (the truth has nothing to compare); a defect is `DIFETTO`.
    pub fn esito(&self) -> &'static str {
        if !self.difetti().is_empty() {
            return "DIFETTO";
        }
        let confrontati = self.livelli.iter().filter(|(_, e)| *e == Esito::Ok).count();
        if confrontati == self.livelli.len() {
            "L"
        } else if confrontati <= 1 {
            "NV"
        } else {
            "P"
        }
    }
    pub fn riga(&self) -> String {
        let m = self.mancanti();
        format!("{}\t{}\t{}", self.file, self.esito(), m.join("; "))
    }
}

pub fn leggi(p: &Path) -> Result<Value, String> {
    let t = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let v: Value = serde_json::from_str(&t).map_err(|e| format!("{}: {e}", p.display()))?;
    if v["schema"] != "verita/1" {
        return Err(format!("{}: schema is not verita/1", p.display()));
    }
    Ok(v)
}

/// Every `*.verita.json` under `dir`, sorted.
pub fn verita_in(dir: &Path) -> Vec<PathBuf> {
    fn giro(d: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                giro(&p, out);
            } else if p.to_string_lossy().ends_with(".verita.json") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    giro(dir, &mut out);
    out.sort();
    out
}

fn kind_tipo(k: ChromKind) -> &'static str {
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

fn num(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn rel_ok(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs()).max(1e-300)
}

fn livello(esito_difetti: Vec<String>, manca: Vec<String>) -> Esito {
    if !esito_difetti.is_empty() {
        Esito::Difetto(esito_difetti)
    } else if !manca.is_empty() {
        Esito::Manca(manca)
    } else {
        Esito::Ok
    }
}

/// F1 to F4 over sample `c` of an open reader; F0 is the caller's (it opened the file); F5 reads the `metodo/1` document.
pub fn confronta(l: &mut dyn Lettore, v: &Value, c: usize, tol: Tol, nome: &str) -> Rapporto {
    let mut livelli: Vec<(&'static str, Esito)> = vec![("F0", Esito::Ok)];
    let indice: Vec<VoceScansione> = match l.indice(c) {
        Ok(i) => i.to_vec(),
        Err(e) => {
            livelli.push(("F1", Esito::Difetto(vec![format!("index: {}", e.key)])));
            return Rapporto {
                file: nome.into(),
                livelli,
            };
        }
    };
    let sc = v["scansioni"].as_array().cloned().unwrap_or_default();

    // F1: counts per level and per chromatogram type, and the ids when both name them the same way
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
    let croms = l.cromatogrammi(c).unwrap_or_default();
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
    livelli.push(("F1", livello(d1, m1)));

    // F2: the header of every scan
    let (mut d2, mut m2) = (Vec::new(), Vec::new());
    let (mut ce_visto, mut isol_visto) = (false, false);
    for (k, (s, t)) in indice.iter().zip(&sc).enumerate() {
        if let Some(rt) = num(&t["rt_min"]) {
            if (s.rt - rt).abs() > tol.rt_min {
                d2.push(format!("scan {k}: RT {} vs {rt}", s.rt));
            }
        }
        if t["livello"].as_u64() != Some(u64::from(s.livello)) {
            d2.push(format!("scan {k}: level {} vs {}", s.livello, t["livello"]));
        }
        let pol = t["polarita"].as_i64().unwrap_or(0);
        if pol != 0 && s.polarita != 0 && pol != i64::from(s.polarita) {
            d2.push(format!("scan {k}: polarity {} vs {pol}", s.polarita));
        } else if pol != 0 && s.polarita == 0 {
            m2.push("polarity (reader says none)".into());
        }
        if let Some(p) = num(&t["precursore_mz"]) {
            match s.precursore {
                Some(q) if (q - p).abs() <= tol.mz_da => {}
                other => d2.push(format!("scan {k}: precursor {other:?} vs {p}")),
            }
        } else if s.precursore.is_some() && t["livello"].as_u64() == Some(1) {
            d2.push(format!("scan {k}: precursor on an MS1 scan"));
        }
        if t["ce"].is_number() || t["isolamento"].is_object() {
            let campi = l.intestazione(c, k).unwrap_or_default();
            if let Some(ce) = num(&t["ce"]) {
                ce_visto = true;
                match campi
                    .iter()
                    .find(|f| f.cv.as_deref() == Some("MS:1000045"))
                    .and_then(|f| f.valore.parse::<f64>().ok())
                {
                    Some(x) if (x - ce).abs() <= 1e-4 => {}
                    other => d2.push(format!("scan {k}: collision energy {other:?} vs {ce}")),
                }
            }
            if t["isolamento"].is_object() {
                isol_visto = true;
                if !campi.iter().any(|f| f.cv.as_deref() == Some("MS:1000827")) {
                    m2.push("isolation window".into());
                }
            }
        }
    }
    let _ = (ce_visto, isol_visto);
    m2.sort();
    m2.dedup();
    livelli.push(("F2", livello(d2, m2)));

    // F3: the points of the chosen scans
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
        let mz: Vec<f64> = p["mz"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(num)
            .collect();
        let it: Vec<f64> = p["intensita"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(num)
            .collect();
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
    livelli.push(("F3", livello(d3, m3)));

    // F4: the chromatograms, by id, else by type and order
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
        let Ok(ch) = l.cromatogramma(c, &voce.id) else {
            d4.push(format!("chromatogram {id}: cannot be read"));
            continue;
        };
        if Some(ch.intensity.len() as u64) != t["n"].as_u64() {
            d4.push(format!(
                "chromatogram {id}: {} points vs {}",
                ch.intensity.len(),
                t["n"]
            ));
            continue;
        }
        let somma: f64 = ch.intensity.iter().sum();
        let massimo = ch.intensity.iter().cloned().fold(0.0f64, f64::max);
        for (nome, ours, theirs) in [
            ("sum", somma, num(&t["somma"])),
            ("maximum", massimo, num(&t["massimo"])),
        ] {
            if let Some(b) = theirs {
                if !rel_ok(ours, b, tol.cromatogramma_rel) {
                    d4.push(format!("chromatogram {id}: {nome} {ours} vs {b}"));
                }
            }
        }
        let n = ch.intensity.len();
        for p in t["campione"].as_array().into_iter().flatten() {
            let (Some(tt), Some(yy)) = (num(&p[0]), num(&p[1])) else {
                continue;
            };
            let i = ch.time.iter().position(|&x| (x - tt).abs() <= tol.rt_min);
            match i {
                Some(i) if i < n && rel_ok(ch.intensity[i], yy, tol.cromatogramma_rel) => {}
                other => d4.push(format!(
                    "chromatogram {id}: point at {tt} min {other:?} vs {yy}"
                )),
            }
        }
    }
    livelli.push(("F4", livello(d4, m4)));

    // F5: the fields of the truth that name a path of metodo/1
    livelli.push(("F5", metadati(l, v, c)));
    Rapporto {
        file: nome.into(),
        livelli,
    }
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

fn metadati(l: &mut dyn Lettore, v: &Value, c: usize) -> Esito {
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

/// Writes the report of the run (`file<TAB>L|P|NV<TAB>levels missing`) when `MZLAB_ROSINA_RAPPORTO` names a file; the nightly workflow of the private repository collects it.
pub fn scrivi_rapporto(righe: &[Rapporto]) {
    for r in righe {
        eprintln!("ROSINA {}", r.riga());
    }
    if let Some(p) = std::env::var_os("MZLAB_ROSINA_RAPPORTO") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            for r in righe {
                let _ = writeln!(f, "{}", r.riga());
            }
        }
    }
}

/// Panics with every defect of the reports (a `P` or `NV` is not a failure: it is the honest level of the file).
pub fn esigi(righe: &[Rapporto]) {
    let tutti: Vec<String> = righe
        .iter()
        .flat_map(|r| {
            r.difetti()
                .into_iter()
                .map(move |d| format!("{}: {d}", r.file))
        })
        .collect();
    assert!(tutti.is_empty(), "{}", tutti.join("\n"));
}

/// The native file (or folder) a truth belongs to: the path of the truth under `scratchpad/rosina/verita/`, without `.verita.json`, under `dati/formati/`.
fn insieme_di(nativo: &Path) -> Option<mzlab_core::lettori::FileSet> {
    use mzlab_core::lettori::{FileSet, Membro};
    fn membro(p: &Path, nome: String) -> Option<Membro> {
        let f = std::fs::File::open(p).ok()?;
        Some(Membro {
            nome,
            dim: f.metadata().ok()?.len(),
            dati: Box::new(f),
            sha1: None,
        })
    }
    fn giro(d: &Path, prefisso: &str, out: &mut Vec<Membro>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        let mut voci: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        voci.sort();
        for p in voci {
            let nome = format!(
                "{prefisso}/{}",
                p.file_name().unwrap_or_default().to_string_lossy()
            );
            if p.is_dir() {
                giro(&p, &nome, out);
            } else if let Some(m) = membro(&p, nome) {
                out.push(m);
            }
        }
    }
    let base = nativo.file_name()?.to_string_lossy().into_owned();
    let mut membri = Vec::new();
    if nativo.is_dir() {
        giro(nativo, &base, &mut membri);
    } else {
        membri.push(membro(nativo, base.clone())?);
        // a `.wiff` travels with its `.wiff.scan`
        let fratelli = nativo.parent()?.read_dir().ok()?;
        for e in fratelli.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n != base && n.starts_with(&format!("{base}.")) {
                membri.extend(membro(&e.path(), n));
            }
        }
    }
    Some(FileSet { membri })
}

/// The tolerances a truth asks for: `"hr"`, `"lr"`, or an object that overrides single fields of the family default (`mz_ppm` null = by Da, `mz_da`, `rt_min`, `intensita_rel`, `cromatogramma_rel`),
/// e.g. 1 ppm for a file with a calibration per scan (docs/studi/ROSINA.md F.2).
pub fn tolleranza_di(v: &Value, base: Tol) -> Tol {
    match &v["tolleranza"] {
        Value::String(s) if s == "hr" => Tol::HR,
        Value::String(s) if s == "lr" => Tol::LR,
        Value::Object(o) => {
            let mut t = base;
            if let Some(x) = o.get("mz_ppm") {
                t.mz_ppm = x.as_f64();
            }
            for (k, campo) in [
                ("mz_da", &mut t.mz_da),
                ("rt_min", &mut t.rt_min),
                ("intensita_rel", &mut t.intensita_rel),
                ("cromatogramma_rel", &mut t.cromatogramma_rel),
            ] {
                if let Some(x) = o.get(k).and_then(Value::as_f64) {
                    *campo = x;
                }
            }
            t
        }
        _ => base,
    }
}

/// The real corpus: every truth under `scratchpad/rosina/verita/` that `filtro` accepts (it gets the path relative to that folder, without `.verita.json`) is compared with its native
/// file under `dati/formati/`. Skipped without the private data repository (`MZLAB_DATI`); `tol` is the default of the family, a truth may override it (`tolleranza_di`).
pub fn corpus(
    famiglia: &str,
    filtro: fn(&str) -> bool,
    tol: Tol,
    percorso: fn(&str) -> Option<PathBuf>,
) {
    let Some(dir) = percorso("scratchpad/rosina/verita") else {
        eprintln!("SKIP verita_{famiglia}: no data repository");
        return;
    };
    let mut rapporti = Vec::new();
    for vp in verita_in(&dir) {
        let rel = vp
            .strip_prefix(&dir)
            .unwrap_or(&vp)
            .to_string_lossy()
            .trim_end_matches(".verita.json")
            .to_string();
        if !filtro(&rel) {
            continue;
        }
        let Some(nativo) = percorso(&format!("dati/formati/{rel}")) else {
            eprintln!("SKIP {rel}: the native file is not in this checkout");
            continue;
        };
        let v = leggi(&vp).unwrap_or_else(|e| panic!("{e}"));
        let tol = tolleranza_di(&v, tol);
        let c = v["file"]["campione"].as_u64().unwrap_or(0) as usize;
        let fs = insieme_di(&nativo).unwrap_or_else(|| panic!("{rel}: cannot list the files"));
        let l = match mzlab_lettori::apri(fs, &mzlab_core::lettori::Limiti::desktop()) {
            Ok(mut l) => confronta(&mut *l, &v, c, tol, &rel),
            Err(e) => Rapporto {
                file: rel.clone(),
                livelli: vec![(
                    "F0",
                    Esito::Difetto(vec![format!("does not open: {}", e.key)]),
                )],
            },
        };
        rapporti.push(l);
    }
    if rapporti.is_empty() {
        eprintln!("SKIP verita_{famiglia}: no truth for this family yet");
        return;
    }
    scrivi_rapporto(&rapporti);
    esigi(&rapporti);
}
