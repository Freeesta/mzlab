//! The `nominal` preset of `features.detect` on a synthetic nominal-resolution series (a parent with two low-mass products a few thousand counts per second high, the
//! centroid scattering by 0.4 Da): `flow run --set det.preset=nominal`, the desktop route and the browser runner's layers write the same bytes; the preset keeps the products
//! the default drops; the record names the preset. The `.wasm` leg of the same parity is `tests/test_feat_nominal_wasm.py`.

use std::path::PathBuf;

use mzlab_core::json::Value;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};

use crate::flow_run_tests::{flow_run, p};
use crate::tests::layered_table;

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"n0.mzML","type":"raw"},{"name":"n1.mzML","type":"raw"},{"name":"n2.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:n0.mzML","file:n1.mzML","file:n2.mzML"]}},
{"id":"det","step":"features.detect@2","in":{"index":["idx.index"]}},
{"id":"ali","step":"features.align@1","in":{"peaks":["det.peaks"]}},
{"id":"gap","step":"features.gapfill@1","in":{"table":"ali.table","index":["idx.index"]}}]}"#;

/// Parent, products (low mass, low height), the same ions in every file.
const IONS: [(f64, f64, f64); 4] = [
    (301.10, 6.0, 3.0e6),
    (105.03, 6.0, 9.0e3),
    (91.05, 6.0, 5.0e3),
    (77.04, 6.0, 3.5e3),
];

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

pub(crate) fn mzml(seed: u64) -> Vec<u8> {
    let mut rng = Lcg(seed);
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: format!("n{seed}"),
            ..Default::default()
        },
    )
    .unwrap();
    let n = 270;
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.5 + 0.05 * i as f64;
        let mut peaks: Vec<(f64, f32)> = (0..40)
            .map(|_| {
                (
                    60.0 + 340.0 * rng.next(),
                    (1.5e3 * (0.8 * rng.normal()).exp()) as f32,
                )
            })
            .collect();
        for &(m, c, a) in &IONS {
            let v = a * (-0.5 * ((rt - c) / 0.07).powi(2)).exp() * (1.0 + 0.04 * rng.normal());
            if v > 50.0 {
                peaks.push((m + 0.3 + 0.4 * rng.normal(), v as f32));
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
            filter: "+ p ESI Q1MS [50.000-400.000]".into(),
            precursor: None,
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn fixture(tag: &str) -> (PathBuf, Vec<(String, Vec<u8>)>) {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_nom_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let files: Vec<(String, Vec<u8>)> =
        (0..3u64).map(|k| (format!("n{k}.mzML"), mzml(5))).collect();
    for (n, b) in &files {
        std::fs::write(dir.join("data").join(n), b).unwrap();
    }
    std::fs::write(dir.join("plan.json"), PLAN).unwrap();
    (dir, files)
}

/// The preset's values as the layer of the browser runner.
fn layer() -> String {
    let reg = mzlab_feat::steps::registry();
    let step = mzlab_core::step::find(&reg, "features.detect").unwrap();
    let preset = step.presets.iter().find(|p| p.id == "nominal").unwrap();
    let params = Value::Obj(
        preset
            .params
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect(),
    );
    format!(
        r#"[{{"step":"features.detect","params":{}}}]"#,
        mzlab_core::jsonb::write(&params)
    )
}

/// Rows of the detections of every file within 0.5 Da of the nominal `mz` (calibration offset 0.3 Da).
fn rows_near(dir: &std::path::Path, mz: f64) -> usize {
    let det: Vec<Vec<u8>> = (0..3)
        .map(|k| std::fs::read(dir.join(format!("det-n{k}.mzfeatf"))).unwrap())
        .collect();
    let (tables, _) = mzlab_ops::run::feat::run::read_files(&det).unwrap();
    tables
        .iter()
        .map(|t| {
            (0..t.len())
                .filter(|&i| (t.mz[i] - (mz + 0.3)).abs() <= 0.5)
                .count()
        })
        .sum()
}

#[test]
fn the_nominal_preset_gives_the_same_bytes_in_flow_run_the_app_and_the_browser_runner() {
    let (dir, files) = fixture("parity");
    let run = |tag: &str, set: &[&str]| {
        let out = dir.join(tag);
        let (plan, data) = (dir.join("plan.json"), dir.join("data"));
        let mut a = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
        a.extend_from_slice(set);
        let o = flow_run(&a);
        assert_eq!(o.code, 0, "{tag}: {}", o.stderr);
        out
    };
    let nominal = run("nominal", &["--set", "det.preset=nominal"]);
    let default = run("default", &[]);
    let table = |d: &PathBuf| std::fs::read(d.join("ali.mzfeat")).unwrap();
    for as_text in [false, true] {
        assert!(
            table(&nominal) == layered_table(&files, &layer(), as_text),
            "browser runner differs (as text: {as_text})"
        );
    }
    assert_ne!(
        table(&nominal),
        table(&default),
        "the preset changes the table"
    );
    // the desktop route runs the plan the CLI wrote (preset included)
    let desk = dir.join("desk");
    std::fs::create_dir_all(&desk).unwrap();
    let as_run = std::fs::read(nominal.join("plan.json")).unwrap();
    crate::flow::run_plan(&as_run, &dir.join("data"), &desk, 1, None).unwrap();
    assert_eq!(
        std::fs::read(desk.join("gap.mzfeat")).unwrap(),
        std::fs::read(nominal.join("gap.mzfeat")).unwrap(),
        "desktop route"
    );
    // the record names the choice
    let prov = std::fs::read_to_string(nominal.join("prov.json")).unwrap();
    assert!(prov.contains("\"preset\":\"nominal\""), "{prov}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_nominal_preset_keeps_the_low_mass_products_and_one_row_per_ion() {
    let (dir, _) = fixture("rows");
    let run = |tag: &str, set: &[&str]| {
        let out = dir.join(tag);
        let (plan, data) = (dir.join("plan.json"), dir.join("data"));
        let mut a = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
        a.extend_from_slice(set);
        assert_eq!(flow_run(&a).code, 0);
        out
    };
    let default = run("default", &[]);
    let nominal = run("nominal", &["--set", "det.preset=nominal"]);
    for mz in [91.05, 77.04] {
        assert_eq!(rows_near(&default, mz), 0, "the default drops {mz}");
        assert_eq!(
            rows_near(&nominal, mz),
            3,
            "the preset keeps {mz} in each of the three files"
        );
    }
    assert_eq!(
        rows_near(&nominal, 301.10),
        3,
        "the parent is one row per file"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
