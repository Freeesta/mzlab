//! The low-resolution Dig of a negative-mode series (ESI-, parent as [M-H]-): the products are looked for as [M-H]- ions (neutral mass - 1.00728), the parent
//! is its own [M-H]- peak and the answer says so. Synthetic data only: a bentazone-like parent (C10H12N2O3S) that falls and two products that rise.

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::legacy::lr::{self, summary::summary};
use mzlab_feat_wasm::run;
use serde_json::Value;

mod comune;
use comune::{noise, open, Lcg, SHIFT, TIMES};

const NEUTRAL: &str = "C10H12N2O3S";
/// [M-H]- of the parent, of its product -OS+H2 and of its product -H2OS.
const PARENT_MZ: f64 = 239.0496;
const OS_H2: f64 = 193.0996;
const H2OS: f64 = 189.0670;

fn pool(t: Option<f64>) -> Vec<(f64, f64, f64, f64)> {
    let mut out = vec![(311.10, 3.3, 0.07, 4.0e5)];
    let Some(k) = t else { return out };
    out.push((PARENT_MZ, 9.0, 0.07, 3.0e6 * (-0.05 * k).exp()));
    out.push((OS_H2, 7.4, 0.07, 1.2e6 * (1.0 - (-0.06 * k).exp())));
    out.push((H2OS, 8.3, 0.07, 6.0e5 * (1.0 - (-0.03 * k).exp())));
    out
}

/// A negative-mode full-scan file: Q1 scans every 3 s from 0.5 to 14 min, noise and the ions of `pool`.
fn index_of(t: Option<f64>, seed: u64) -> Vec<u8> {
    let (mut rng, name) = (Lcg(seed), format!("neg{seed}"));
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
        let nz = noise(&mut rng, &prev);
        let mut peaks: Vec<(f64, f32)> = nz
            .iter()
            .map(|&m| (m, (3e3 * (0.8 * rng.normal()).exp()) as f32))
            .collect();
        prev = nz;
        for (m, c, s, a) in pool(t) {
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
            polarity: -1,
            profile: false,
            filter: "- p ESI Q1MS [100.000-400.000]".into(),
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

/// The Dig as the page runs it: detection, alignment, hypotheses searched in `polarity` (what `finder-lr.js` passes from the adduct), then the Dig.
fn dig(adduct: &str, polarity: &str) -> (lr::LrDig, Value) {
    let (mut files, mut idxs) = (Vec::new(), Vec::new());
    for (i, t) in TIMES.iter().enumerate() {
        files.push(format!(
            r#"{{"name": "neg-t{t}.mzML", "time": {t:.1}, "type": "sample", "kind": "full"}}"#
        ));
        idxs.push(index_of(Some(*t), 11 + i as u64));
    }
    files.push(
        r#"{"name": "neg-blank.mzML", "time": null, "type": "blank", "kind": "full"}"#.into(),
    );
    idxs.push(index_of(None, 99));
    let files = format!("[{}]", files.join(","));
    let parent = format!(r#"{{"name": "Parent", "neutral": "{NEUTRAL}", "adduct": "{adduct}"}}"#);
    let plan: Value = serde_json::from_str(&lr::plan::plan_json(&files, &parent).unwrap()).unwrap();
    let r = run::recipe_for(polarity, &mut open(&idxs[0])).unwrap();
    let list: Value = serde_json::from_str(&files).unwrap();
    let stored: Vec<Vec<u8>> = idxs
        .iter()
        .zip(list.as_array().unwrap())
        .map(|(b, f)| {
            run::detect(
                &mut open(b),
                f["name"].as_str().unwrap(),
                &run::recipe_for("", &mut open(b)).unwrap(),
            )
            .unwrap()
        })
        .collect();
    let (feats, origin) = run::read_files(&stored).unwrap();
    let table = run::align(&feats, &origin, &r, |k| Ok(open(&idxs[k]))).unwrap();
    let rules = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../TP_Mine/data/regole_tp_2.json"
    ))
    .unwrap();
    let mut ix: Vec<Index<Vec<u8>>> = idxs.iter().map(|b| open(b)).collect();
    let refs: Vec<_> = feats.iter().collect();
    let h = run::hypotheses(
        &mut ix,
        &refs,
        &rules,
        "",
        plan["formula"].as_str().unwrap(),
        None,
        &r,
    )
    .unwrap();
    let d = lr::dig(
        &files,
        &parent,
        "{}",
        &table,
        &h.registry.to_json(),
        &mut |k| Ok(open(&idxs[k])),
    )
    .unwrap();
    let s: Value = serde_json::from_str(&summary(&d, "{}")).unwrap();
    (d, s)
}

fn found(d: &lr::LrDig, mz: f64) -> bool {
    d.entries.iter().any(|e| {
        e.id != 0
            && !e.unexpected
            && (e.mz - mz).abs() < 0.02
            && matches!(e.label, "forte" | "possibile")
    })
}

#[test]
fn the_products_of_an_m_h_parent_are_m_h_ions() {
    let (d, s) = dig("[M-H]-", r#"{"polarity": -1}"#);
    assert!(d.parent_ok, "the parent is its [M-H]- peak at {PARENT_MZ}");
    assert_eq!(s["parent"]["adduct"], "[M-H]-");
    assert!((s["parent"]["mz"].as_f64().unwrap() - PARENT_MZ).abs() < 0.005);
    assert!(
        found(&d, OS_H2),
        "-OS+H2 at {OS_H2}: {:?}",
        d.entries
            .iter()
            .map(|e| (e.mz, e.label))
            .collect::<Vec<_>>()
    );
    assert!(found(&d, H2OS), "-H2OS at {H2OS}");
    // every hypothesis is an [M-H]- ion of a formula: its calculated m/z is neutral - 1.00728
    let f = d
        .entries
        .iter()
        .find(|e| e.id != 0 && (e.mz - OS_H2).abs() < 0.02)
        .unwrap();
    assert!(
        (f.neutral_mass.unwrap() - 1.00728 - f.mz).abs() < 0.001,
        "neutral {:?} ion {}",
        f.neutral_mass,
        f.mz
    );
}

#[test]
fn a_parent_whose_ion_is_not_in_the_files_names_the_ion_it_looked_for() {
    // the same negative files, the parent given as [M+H]+: 241.06 is not a peak, and the message says which ion was searched
    let (d, _) = dig("[M+H]+", "");
    assert!(!d.parent_ok);
    let weak: Vec<_> = d
        .entries
        .iter()
        .filter(|e| e.id != 0 && !e.unexpected && e.score.is_some())
        .collect();
    assert!(weak.iter().all(|e| e
        .criteria
        .iter()
        .any(|c| c.name == lr::PARENT_OK && c.status == "fail" && c.text.contains("[M+H]+"))));
}
