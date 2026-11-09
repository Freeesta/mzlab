//! The Rosina test bench (docs/studi/ROSINA.md F): compares a reader with the ground truth `verita/1` of an external oracle, levels F0-F5, and reports per file `L`, `P` or `NV`.
//! Included with `#[path = "common/truth.rs"] mod verita;` by `truth_bench.rs` (the bench on built files) and by `verita_<famiglia>.rs` (the real corpus, skipped without data).
#![allow(dead_code)]

use mzlab_core::readers::{Reader, ScanEntry};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[path = "truth_levels.rs"]
mod levels_f;
#[allow(unused_imports)]
pub use levels_f::{json_path, normalize};

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
pub enum Outcome {
    Ok,
    Missing(Vec<String>),
    Defect(Vec<String>),
}

pub const LEVELS: [&str; 6] = ["F0", "F1", "F2", "F3", "F4", "F5"];

#[derive(Debug)]
pub struct Ratio {
    pub file: String,
    pub levels: Vec<(&'static str, Outcome)>,
}

impl Ratio {
    pub fn defects(&self) -> Vec<String> {
        self.levels
            .iter()
            .filter_map(|(l, e)| match e {
                Outcome::Defect(d) => {
                    Some(d.iter().map(|x| format!("{l}: {x}")).collect::<Vec<_>>())
                }
                _ => None,
            })
            .flatten()
            .collect()
    }
    /// The levels that could not be compared (with what is missing).
    pub fn missing(&self) -> Vec<String> {
        self.levels
            .iter()
            .filter_map(|(l, e)| match e {
                Outcome::Missing(m) => Some(format!("{l} ({})", m.join(", "))),
                _ => None,
            })
            .collect()
    }
    /// `L`: F0-F5 compared and equal; `P`: equal where compared, some level missing; `NV`: only F0 (the truth has nothing to compare); a defect is `DIFETTO`.
    pub fn outcome(&self) -> &'static str {
        if !self.defects().is_empty() {
            return "DIFETTO";
        }
        let compared = self
            .levels
            .iter()
            .filter(|(_, e)| *e == Outcome::Ok)
            .count();
        if compared == self.levels.len() {
            "L"
        } else if compared <= 1 {
            "NV"
        } else {
            "P"
        }
    }
    pub fn row(&self) -> String {
        let m = self.missing();
        format!("{}\t{}\t{}", self.file, self.outcome(), m.join("; "))
    }
}

pub fn read(p: &Path) -> Result<Value, String> {
    let t = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let v: Value = serde_json::from_str(&t).map_err(|e| format!("{}: {e}", p.display()))?;
    if v["schema"] != "verita/1" {
        return Err(format!("{}: schema is not verita/1", p.display()));
    }
    Ok(v)
}

/// Every `*.verita.json` under `dir`, sorted.
pub fn truths_in(dir: &Path) -> Vec<PathBuf> {
    fn round_trip(d: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                round_trip(&p, out);
            } else if p.to_string_lossy().ends_with(".verita.json") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    round_trip(dir, &mut out);
    out.sort();
    out
}

/// F1 to F4 over sample `c` of an open reader; F0 is the caller's (it opened the file); F5 reads the `metodo/1` document.
pub fn compare(l: &mut dyn Reader, v: &Value, c: usize, tol: Tol, name: &str) -> Ratio {
    let mut livelli: Vec<(&'static str, Outcome)> = vec![("F0", Outcome::Ok)];
    let index: Vec<ScanEntry> = match l.index(c) {
        Ok(i) => i.to_vec(),
        Err(e) => {
            livelli.push(("F1", Outcome::Defect(vec![format!("index: {}", e.key)])));
            return Ratio {
                file: name.into(),
                levels: livelli,
            };
        }
    };
    let croms = l.chromatograms(c).unwrap_or_default();
    livelli.push(("F1", levels_f::f1_counts(&index, &croms, v)));
    livelli.push(("F2", levels_f::f2_headers(l, &index, v, c, tol)));
    livelli.push(("F3", levels_f::f3_points(l, v, c, tol)));
    livelli.push(("F4", levels_f::f4_chromatograms(l, &croms, v, c, tol)));
    livelli.push(("F5", levels_f::metadata(l, v, c)));
    Ratio {
        file: name.into(),
        levels: livelli,
    }
}

/// Writes the report of the run (`file<TAB>L|P|NV<TAB>levels missing`) when `MZLAB_ROSINA_RAPPORTO` names a file; the nightly workflow of the private repository collects it.
pub fn write_report(rows: &[Ratio]) {
    for r in rows {
        eprintln!("ROSINA {}", r.row());
    }
    if let Some(p) = std::env::var_os("MZLAB_ROSINA_RAPPORTO") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            for r in rows {
                let _ = writeln!(f, "{}", r.row());
            }
        }
    }
}

/// Panics with every defect of the reports (a `P` or `NV` is not a failure: it is the honest level of the file).
pub fn require(rows: &[Ratio]) {
    let all: Vec<String> = rows
        .iter()
        .flat_map(|r| {
            r.defects()
                .into_iter()
                .map(move |d| format!("{}: {d}", r.file))
        })
        .collect();
    assert!(all.is_empty(), "{}", all.join("\n"));
}

/// The native file (or folder) a truth belongs to: the path of the truth under `scratchpad/rosina/verita/`, without `.verita.json`, under `dati/formati/`.
fn set_of(native: &Path) -> Option<mzlab_core::readers::FileSet> {
    use mzlab_core::readers::{FileSet, Member};
    fn member(p: &Path, name: String) -> Option<Member> {
        let f = std::fs::File::open(p).ok()?;
        Some(Member {
            nome: name,
            dim: f.metadata().ok()?.len(),
            dati: Box::new(f),
            sha1: None,
        })
    }
    fn round_trip(d: &Path, prefix: &str, out: &mut Vec<Member>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            let name = format!(
                "{prefix}/{}",
                p.file_name().unwrap_or_default().to_string_lossy()
            );
            if p.is_dir() {
                round_trip(&p, &name, out);
            } else if let Some(m) = member(&p, name) {
                out.push(m);
            }
        }
    }
    let base = native.file_name()?.to_string_lossy().into_owned();
    let mut members = Vec::new();
    if native.is_dir() {
        round_trip(native, &base, &mut members);
    } else {
        members.push(member(native, base.clone())?);
        // a `.wiff` travels with its `.wiff.scan`
        let siblings = native.parent()?.read_dir().ok()?;
        for e in siblings.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n != base && n.starts_with(&format!("{base}.")) {
                members.extend(member(&e.path(), n));
            }
        }
    }
    Some(FileSet { membri: members })
}

/// The tolerances a truth asks for: `"hr"`, `"lr"`, or an object that overrides single fields of the family default (`mz_ppm` null = by Da, `mz_da`, `rt_min`, `intensita_rel`, `cromatogramma_rel`),
/// e.g. 1 ppm for a file with a calibration per scan (docs/studi/ROSINA.md F.2).
pub fn tolerance_of(v: &Value, base: Tol) -> Tol {
    match &v["tolleranza"] {
        Value::String(s) if s == "hr" => Tol::HR,
        Value::String(s) if s == "lr" => Tol::LR,
        Value::Object(o) => {
            let mut t = base;
            if let Some(x) = o.get("mz_ppm") {
                t.mz_ppm = x.as_f64();
            }
            for (k, field) in [
                ("mz_da", &mut t.mz_da),
                ("rt_min", &mut t.rt_min),
                ("intensita_rel", &mut t.intensita_rel),
                ("cromatogramma_rel", &mut t.cromatogramma_rel),
            ] {
                if let Some(x) = o.get(k).and_then(Value::as_f64) {
                    *field = x;
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
    family: &str,
    filter: fn(&str) -> bool,
    tol: Tol,
    file_path: fn(&str) -> Option<PathBuf>,
) {
    let Some(dir) = file_path("scratchpad/rosina/verita") else {
        eprintln!("SKIP verita_{family}: no data repository");
        return;
    };
    let mut reports = Vec::new();
    for vp in truths_in(&dir) {
        let rel = vp
            .strip_prefix(&dir)
            .unwrap_or(&vp)
            .to_string_lossy()
            .trim_end_matches(".verita.json")
            .to_string();
        if !filter(&rel) {
            continue;
        }
        let Some(native) = file_path(&format!("dati/formati/{rel}")) else {
            eprintln!("SKIP {rel}: the native file is not in this checkout");
            continue;
        };
        let v = read(&vp).unwrap_or_else(|e| panic!("{e}"));
        let tol = tolerance_of(&v, tol);
        let c = v["file"]["campione"].as_u64().unwrap_or(0) as usize;
        let fs = set_of(&native).unwrap_or_else(|| panic!("{rel}: cannot list the files"));
        let l = match mzlab_lettori::apri(fs, &mzlab_core::readers::Limits::desktop()) {
            Ok(mut l) => compare(&mut *l, &v, c, tol, &rel),
            Err(e) => Ratio {
                file: rel.clone(),
                levels: vec![(
                    "F0",
                    Outcome::Defect(vec![format!("does not open: {}", e.key)]),
                )],
            },
        };
        reports.push(l);
    }
    if reports.is_empty() {
        eprintln!("SKIP verita_{family}: no truth for this family yet");
        return;
    }
    write_report(&reports);
    require(&reports);
}
