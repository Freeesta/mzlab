//! The targeted pass on a small synthetic series (`tools/feat_mirato.py`, weak products): it finds products the detection misses, two runs give the same
//! bytes and the same registry, a row with no detected cell and no MS2 is capped at level 4, the chance of a coincidence is a probability and the
//! registry knows how many hypotheses were tried. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::elements::{fmt, parse_formula};
use mzlab_core::idxread::Index;
use mzlab_feat::hypo::{generate, HypoParams, Imported, Inputs, Outcome, Rules};
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::table::{FileFeatures, CELL_TARGETED};
use mzlab_feat::target::{run, TargetParams};
use mzlab_feat::trace::Params;
use serde_json::Value;

const PROTON: f64 = 1.007_276;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The series in a fresh directory, or `None` when python3/numpy are not there.
fn series() -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!("mzlab_mirato_{}", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/feat_mirato.py"))
        .args([
            "--seme",
            "2",
            "--profilo",
            "piccolo",
            "--scala",
            "0.05",
            "--out",
        ])
        .arg(&out)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
    }
    ok.then_some(out)
}

#[test]
fn targeted_pass_is_deterministic_capped_and_counted() {
    let Some(dir) = series() else { return };
    let truth: Value =
        serde_json::from_slice(&std::fs::read(dir.join("verita.json")).unwrap()).unwrap();
    let mut f = parse_formula(truth["parent"]["ion"].as_str().unwrap()).unwrap();
    f.iter_mut().filter(|e| e.0 == "H").for_each(|e| e.1 -= 1);
    let parent = fmt(&f);
    let parent_rt = truth["parent"]["rt_obs"].as_f64();
    let imp: Vec<Imported> = truth["ioni"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| ["tp1", "tp2", "isomero"].contains(&i["ruolo"].as_str().unwrap()))
        .map(|i| Imported {
            name: i["id"].as_str().unwrap().to_string(),
            formula: String::new(),
            mass: Some(i["mz"].as_f64().unwrap() - PROTON),
        })
        .collect();
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "mzidx"))
        .collect();
    paths.sort();
    let (mut ix, mut feats) = (Vec::new(), Vec::<FileFeatures>::new());
    for p in &paths {
        let f = File::open(p).unwrap();
        let size = f.metadata().unwrap().len();
        let mut i = Index::open(f, size).unwrap();
        let pr = Params {
            polarity: Some(1),
            ..Default::default()
        };
        feats.push(analyse(&mut i, &pr, &Steps::default()).unwrap().0);
        ix.push(i);
    }
    let rules =
        Rules::parse(&std::fs::read(root().join("TP_Mine/data/regole_tp.json")).unwrap()).unwrap();
    let inp = Inputs {
        parent: &parent,
        suspects: None,
        families: &[],
        imported: &imp,
        fragments: &[],
    };
    let reg0 = generate(&rules, &inp, &HypoParams::default()).unwrap();
    let tp = TargetParams {
        parent_rt,
        ..Default::default()
    };
    let refs: Vec<&FileFeatures> = feats.iter().collect();
    let (mut a, mut b) = (reg0.clone(), reg0.clone());
    let ra = run(&mut ix, &refs, &mut a, &tp).unwrap();
    let rb = run(&mut ix, &refs, &mut b, &tp).unwrap();
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(ra.to_bytes(), rb.to_bytes());
    assert_eq!(a.to_json(), b.to_json());
    assert_eq!(ra.tried as usize, reg0.len());
    assert_eq!(ra.found as usize, ra.rows.len());
    assert!(ra.found > 0, "nothing found");
    assert!(
        ra.rows.windows(2).all(|w| w[0].hyp < w[1].hyp),
        "rows in the order of the registry"
    );
    let found_in_reg = a
        .outcomes
        .iter()
        .filter(|o| matches!(o, Outcome::Found(_)))
        .count();
    assert_eq!(found_in_reg, ra.rows.len());
    let mut targeted = 0;
    for (k, r) in ra.rows.iter().enumerate() {
        assert!(r.coincidence > 0.0 && r.coincidence <= 1.0);
        assert_eq!(r.cells.len(), ra.n_files);
        if r.n_detected == 0 && r.ms2_total == 0 {
            assert_eq!(r.level_cap(), Some(4));
        } else {
            assert_eq!(r.level_cap(), None);
        }
        let ev = ra.evidence(&a, k);
        assert_eq!((ev.tried, ev.found), (ra.tried, ra.found));
        targeted += r.cells.iter().filter(|c| c.state == CELL_TARGETED).count();
    }
    assert!(targeted > 0, "the weak products must enter as targeted");
    let al = ra.to_aligned();
    assert_eq!(al.mz.len(), ra.rows.len());
    assert_eq!(al.state.len(), ra.rows.len() * ra.n_files);
}
