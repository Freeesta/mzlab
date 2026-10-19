//! The Rosina test bench (docs/studi/ROSINA.md F): compares a reader with the ground truth `verita/1` of an external oracle, levels F0-F5, and reports per file `L`, `P` or `NV`.
//! Included with `#[path = "comune/verita.rs"] mod verita;` by `verita_banco.rs` (the bench on built files) and by `verita_<famiglia>.rs` (the real corpus, skipped without data).
#![allow(dead_code)]

use mzlab_core::lettori::{Lettore, VoceScansione};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[path = "verita_livelli.rs"]
mod livelli_f;
#[allow(unused_imports)]
pub use livelli_f::{normalizza, percorso_json};

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
    let croms = l.cromatogrammi(c).unwrap_or_default();
    livelli.push(("F1", livelli_f::f1_conteggi(&indice, &croms, v)));
    livelli.push(("F2", livelli_f::f2_intestazioni(l, &indice, v, c, tol)));
    livelli.push(("F3", livelli_f::f3_punti(l, v, c, tol)));
    livelli.push(("F4", livelli_f::f4_cromatogrammi(l, &croms, v, c, tol)));
    livelli.push(("F5", livelli_f::metadati(l, v, c)));
    Rapporto {
        file: nome.into(),
        livelli,
    }
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
