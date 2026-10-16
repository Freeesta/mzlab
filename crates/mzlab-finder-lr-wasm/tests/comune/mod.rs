//! The synthetic low-resolution series of `TP_Mine/tests/lr_synth.py` written in Rust and the Dig on it (the pipeline runs natively as the pool of
//! `feat-worker.js` runs it), shared by the tests of the low-resolution Dig.
#![allow(dead_code)]

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::legacy::lr;
use mzlab_feat_wasm::run;
use serde_json::Value;
use std::path::PathBuf;

pub const SHIFT: f64 = 0.30;
pub const TIMES: [f64; 7] = [0.0, 5.0, 10.0, 15.0, 30.0, 45.0, 60.0];
pub const PARENT: &str = r#"{"name": "Parent", "neutral": "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"}"#;

pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn normal(&mut self) -> f64 {
        let (a, b) = (self.next().max(1e-12), self.next());
        (-2.0 * a.ln()).sqrt() * (2.0 * std::f64::consts::PI * b).cos()
    }
}

/// (m/z, RT, sigma, amplitude) of `lr_synth._pool`.
pub fn pool(t: Option<f64>) -> Vec<(f64, f64, f64, f64)> {
    let mut out = vec![(311.10, 3.3, 0.07, 4.0e5), (157.09, 3.0, 0.08, 1.5e5)];
    let Some(k) = t else { return out };
    out.push((241.0641, 9.0, 0.07, 3.0e6 * (-0.05 * k).exp()));
    out.push((257.0590, 7.4, 0.07, 1.2e6 * (1.0 - (-0.06 * k).exp())));
    out.push((257.0590, 8.3, 0.07, 6.0e5 * (1.0 - (-0.03 * k).exp())));
    let g = 8.0e5 * (1.0 - (-0.05 * k).exp());
    out.push((333.20, 6.8, 0.07, g));
    out.push((334.20, 6.8, 0.07, 0.11 * g));
    out.push((
        355.10,
        5.2,
        0.07,
        9.0e5 * ((-0.01 * k).exp() - (-0.15 * k).exp()),
    ));
    if k > 0.0 {
        out.push((289.10, 4.4, 0.07, 5.0e5));
    }
    out
}

/// `lr_synth._noise`: 40 m/z between 100 and 400, none within 1 Da of a noise peak of the scan before (noise never forms a row).
pub fn noise(rng: &mut Lcg, prev: &[f64]) -> Vec<f64> {
    (0..40)
        .map(|_| {
            let mut m = 100.0 + 300.0 * rng.next();
            for _ in 0..50 {
                if !prev.iter().any(|p| (p - m).abs() < 1.0) {
                    break;
                }
                m = 100.0 + 300.0 * rng.next();
            }
            m
        })
        .collect()
}

/// The `.mzidx` of one file: Q1 scans every 3 s from 0.5 to 14 min, the noise of `noise` and the ions of `pool`.
pub fn index_of(t: Option<f64>, seed: u64) -> Vec<u8> {
    index_with(t, seed, &|_| Vec::new())
}

/// `index_of` with more ions, `(m/z, RT, sigma, amplitude)` by sampling time; they have a generator of their own, so the other ions are the same.
pub fn index_with(
    t: Option<f64>,
    seed: u64,
    extra: &dyn Fn(Option<f64>) -> Vec<(f64, f64, f64, f64)>,
) -> Vec<u8> {
    let mut rng = Lcg(seed);
    let mut rng2 = Lcg(seed + 5000);
    let more = extra(t);
    let comps = pool(t);
    let name = format!("lr{seed}");
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: name.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 270;
    w.open_spectra(n).unwrap();
    let mut prev: Vec<f64> = Vec::new();
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let noise = noise(&mut rng, &prev);
        let mut peaks: Vec<(f64, f32)> = noise
            .iter()
            .map(|&m| (m, (3e3 * (0.8 * rng.normal()).exp()) as f32))
            .collect();
        prev = noise;
        for &(m, c, s, a) in &comps {
            let v = a * (-0.5 * ((rt - c) / s).powi(2)).exp() * (1.0 + 0.04 * rng.normal());
            if v > 100.0 {
                peaks.push((m + SHIFT + 0.03 * rng.normal(), v as f32));
            }
        }
        for &(m, c, s, a) in &more {
            let v = a * (-0.5 * ((rt - c) / s).powi(2)).exp() * (1.0 + 0.04 * rng2.normal());
            if v > 100.0 {
                peaks.push((m + SHIFT + 0.03 * rng2.normal(), v as f32));
            }
        }
        peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
        w.spectrum(&Spectrum {
            index: i,
            id: format!("sample=1 period=1 cycle={} experiment=1", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile: false,
            filter: "+ p ESI Q1MS [100.000-400.000]".into(),
            precursor: None,
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, &format!("{name}.mzML")).unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

pub fn open(idx: &[u8]) -> Index<Vec<u8>> {
    Index::open(idx.to_vec(), idx.len() as u64).unwrap()
}

/// The files as the page passes them (the kind of `classify`) and the indexes of the columns.
pub fn series() -> (String, Vec<Vec<u8>>) {
    series_with(&|_| Vec::new())
}

/// `series` with more ions in every file (`index_with`).
pub fn series_with(
    extra: &dyn Fn(Option<f64>) -> Vec<(f64, f64, f64, f64)>,
) -> (String, Vec<Vec<u8>>) {
    let mut files = Vec::new();
    let mut idxs = Vec::new();
    for (i, t) in TIMES.iter().enumerate() {
        files.push(format!(r#"{{"name": "lr_FullMass-t{t}.mzML", "time": {t:.1}, "type": "sample", "kind": "full"}}"#));
        idxs.push(index_with(Some(*t), 11 + i as u64, extra));
    }
    files
        .push(r#"{"name": "lr_blank.mzML", "time": null, "type": "blank", "kind": "full"}"#.into());
    idxs.push(index_with(None, 99, extra));
    (format!("[{}]", files.join(",")), idxs)
}

/// The pipeline as the pool runs it: detection, alignment, hypotheses of the parent; then the Dig.
pub fn dig(files: &str, idxs: &[Vec<u8>]) -> lr::LrDig {
    dig_for(files, PARENT, idxs)
}

pub fn dig_for(files: &str, parent: &str, idxs: &[Vec<u8>]) -> lr::LrDig {
    dig_set(files, parent, "{}", idxs)
}

/// `dig_for` with the settings of the Dig (`plan::settings`).
pub fn dig_set(files: &str, parent: &str, settings: &str, idxs: &[Vec<u8>]) -> lr::LrDig {
    let plan: Value = serde_json::from_str(&lr::plan::plan_json(files, parent).unwrap()).unwrap();
    assert_eq!(plan["rust"], true);
    let r = run::recipe_for("", &mut open(&idxs[0])).unwrap();
    // as detect_file_to of the pool: the recipe of every file from its own index, the file name as the name
    let names: Vec<String> = serde_json::from_str::<Value>(files)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap().to_string())
        .collect();
    let stored: Vec<Vec<u8>> = idxs
        .iter()
        .zip(&names)
        .map(|(b, n)| {
            run::detect(&mut open(b), n, &run::recipe_for("", &mut open(b)).unwrap()).unwrap()
        })
        .collect();
    let (feats, origin) = run::read_files(&stored).unwrap();
    let table = run::align(&feats, &origin, &r, |k| Ok(open(&idxs[k]))).unwrap();
    let rules = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../TP_Mine/data/regole_tp.json"
    ))
    .unwrap();
    let mut ix: Vec<Index<Vec<u8>>> = idxs.iter().map(|b| open(b)).collect();
    let refs: Vec<_> = feats.iter().collect();
    let formula = plan["formula"].as_str().unwrap();
    let h = run::hypotheses(&mut ix, &refs, &rules, "", formula, None, &r).unwrap();
    lr::dig(
        files,
        parent,
        settings,
        &table,
        &h.registry.to_json(),
        &mut |k| Ok(open(&idxs[k])),
    )
    .unwrap()
}

/// `d` with the MS2 matches of the series: `idxs` are the columns, `ms2` the MS2 files (in the order of the plan).
pub fn linked(mut d: lr::LrDig, idxs: &[Vec<u8>], ms2: &[&Vec<u8>]) -> lr::LrDig {
    let all: Vec<&Vec<u8>> = idxs.iter().chain(ms2.iter().copied()).collect();
    lr::ms2link::ms2_link(&mut d, &[], &mut |k| Ok(open(all[k]))).unwrap();
    d
}

/// The private data repository (with the truth of the MS2 tests), if there is one: `MZLAB_DATI` or the folder next to this repository.
pub fn root() -> Option<PathBuf> {
    let p = match std::env::var("MZLAB_DATI") {
        Ok(v) => PathBuf::from(v),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../mzlab-dati"),
    };
    p.join("scratchpad/tabui_ms2_lr_truth.json")
        .exists()
        .then_some(p)
}

pub fn index_of_file(path: &std::path::Path) -> Vec<u8> {
    let mzml = std::fs::read(path).expect("mzML");
    let mut src = mzml.clone();
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let p = prepare(&mut src, mzml.len() as u64, &name).unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}
