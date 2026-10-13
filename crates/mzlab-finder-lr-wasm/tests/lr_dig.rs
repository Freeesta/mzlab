//! The low-resolution Dig on the synthetic series of `TP_Mine/tests/lr_synth.py` (same ions, kinetics and +0.30 Da shift, written here in
//! Rust): the pipeline runs natively as the pool of `feat-worker.js` runs it (detection, alignment, hypotheses), then the adapter. Checked
//! against `tests/golden/finder_lr_contratto.json` (`tools/finder_lr_riferimento.py`): the keys and JSON types of the Python answer, and its
//! rows that pass the criteria found again within ±0.35 Da and ±0.25 min.

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::legacy::lr::{self, detail::detail, summary::summary};
use mzlab_feat_wasm::run;
use serde_json::Value;

const SHIFT: f64 = 0.30;
const TIMES: [f64; 7] = [0.0, 5.0, 10.0, 15.0, 30.0, 45.0, 60.0];
const PARENT: &str = r#"{"name": "Parent", "neutral": "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"}"#;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let (a, b) = (self.next().max(1e-12), self.next());
        (-2.0 * a.ln()).sqrt() * (2.0 * std::f64::consts::PI * b).cos()
    }
}

/// (m/z, RT, sigma, amplitude) of `lr_synth._pool`.
fn pool(t: Option<f64>) -> Vec<(f64, f64, f64, f64)> {
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
fn noise(rng: &mut Lcg, prev: &[f64]) -> Vec<f64> {
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
fn index_of(t: Option<f64>, seed: u64) -> Vec<u8> {
    let mut rng = Lcg(seed);
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

fn open(idx: &[u8]) -> Index<Vec<u8>> {
    Index::open(idx.to_vec(), idx.len() as u64).unwrap()
}

/// The files as the page passes them (the kind of `classify`) and the indexes of the columns.
fn series() -> (String, Vec<Vec<u8>>) {
    let mut files = Vec::new();
    let mut idxs = Vec::new();
    for (i, t) in TIMES.iter().enumerate() {
        files.push(format!(r#"{{"name": "lr_FullMass-t{t}.mzML", "time": {t:.1}, "type": "sample", "kind": "full"}}"#));
        idxs.push(index_of(Some(*t), 11 + i as u64));
    }
    files
        .push(r#"{"name": "lr_blank.mzML", "time": null, "type": "blank", "kind": "full"}"#.into());
    idxs.push(index_of(None, 99));
    (format!("[{}]", files.join(",")), idxs)
}

/// The pipeline as the pool runs it: detection, alignment, hypotheses of the parent; then the Dig.
fn dig(files: &str, idxs: &[Vec<u8>]) -> lr::LrDig {
    let plan: Value = serde_json::from_str(&lr::plan::plan_json(files, PARENT).unwrap()).unwrap();
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
    lr::dig(files, PARENT, "{}", &table, &h.registry.to_json()).unwrap()
}

fn golden() -> Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/golden/finder_lr_contratto.json"
    );
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Every key of the contract is there with one of its types; nothing else.
fn same_shape(got: &Value, want: &Value, path: &str) {
    let (Value::Object(g), Value::Object(w)) = (got, want) else {
        panic!("{path}: not an object")
    };
    let keys = |m: &serde_json::Map<String, Value>| {
        m.keys().cloned().collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(keys(g), keys(w), "{path}: keys");
    for (k, types) in w {
        let t = kind(&g[k]);
        let ok = types.as_array().unwrap().iter().any(|x| x == t);
        assert!(ok, "{path}.{k}: {t} not in {types}");
    }
}

#[test]
fn contract_and_rows_of_python() {
    let (files, idxs) = series();
    let d = dig(&files, &idxs);
    let s: Value = serde_json::from_str(&summary(&d, r#"{"total": 1}"#)).unwrap();
    let g = golden();
    same_shape(&s, &g["summary"], "summary");
    for row in s["rows"].as_array().unwrap() {
        same_shape(row, &g["row"], "row");
        for k in row["kinetics"].as_array().unwrap() {
            same_shape(k, &g["kinetics"], "kinetics");
        }
    }
    let rows = s["rows"].as_array().unwrap();
    assert_eq!(rows[0]["label"], "progenitore");
    assert_eq!(rows[0]["id"], 0);
    let found = |m: f64, rt: f64| {
        rows.iter().any(|r| {
            matches!(
                r["label"].as_str(),
                Some("progenitore" | "forte" | "possibile")
            ) && (r["mz"].as_f64().unwrap() - m).abs() <= 0.35
                && r["ref_rt"].as_f64().is_some_and(|x| (x - rt).abs() <= 0.25)
        })
    };
    let want = g["python_rows"].as_array().unwrap();
    let hit = want
        .iter()
        .filter(|w| found(w["mz"].as_f64().unwrap(), w["ref_rt"].as_f64().unwrap()))
        .count();
    assert!(
        hit as f64 >= 0.95 * want.len() as f64,
        "{hit} of {} rows of the Python found: {rows:#?}",
        want.len()
    );
    for r in rows
        .iter()
        .filter(|r| matches!(r["label"].as_str(), Some("forte" | "possibile")))
    {
        let id = r["id"].as_i64().unwrap();
        let det: Value =
            serde_json::from_str(&detail(&d, id, |k| Ok(open(&idxs[k]))).unwrap()).unwrap();
        same_shape(&det, &g["detail"], "detail");
        assert!(
            det["criteria"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["status"] == "pass"),
            "{id} without proofs"
        );
        for t in det["traces"].as_array().unwrap() {
            same_shape(t, &g["trace"], "trace");
        }
    }
}

#[test]
fn isotope_and_offset() {
    let (files, idxs) = series();
    let d = dig(&files, &idxs);
    let o = d
        .offset
        .as_ref()
        .expect("the offset measured on the parent");
    assert!(
        o.applied && (o.offset - SHIFT).abs() < 0.1,
        "offset {}",
        o.offset
    );
    let strong = |e: &&lr::Entry| matches!(e.label, "forte" | "possibile");
    assert!(
        !d.entries
            .iter()
            .filter(strong)
            .any(|e| (e.mz_x - (334.20 + SHIFT)).abs() <= 0.35),
        "the 13C peak of 333.20 is not an ion of its own"
    );
    assert!(
        !d.entries
            .iter()
            .filter(strong)
            .any(|e| (e.mz_x - (311.10 + SHIFT)).abs() <= 0.35),
        "an ion of the blank"
    );
}

#[test]
fn deterministic() {
    let (files, idxs) = series();
    let (a, b) = (dig(&files, &idxs), dig(&files, &idxs));
    assert_eq!(summary(&a, "{}"), summary(&b, "{}"));
}
