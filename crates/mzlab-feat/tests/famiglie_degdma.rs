//! The hard family of `tools/valida/sintetici_degdma.py` (weak [M+H]+, adducts and a base-peak in-source fragment stronger than the molecule, the same m/z coming
//! from a later product): the rules must never join the product's fragment to the molecule, and two runs give the same bytes. How many members of the molecule
//! are found is a measure (`docs/agenti/mzfeat-prove.md`), not an assertion. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::Path;

use mzlab_core::idxread::Index;
use mzlab_feat::family::{analyse_families, FamParams, Orient};
use mzlab_feat::famrecord::record;
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::table::*;
use mzlab_feat::trace::Params;
use serde_json::Value;

fn row(f: &FileFeatures, mz: f64, rt: f64) -> usize {
    (0..f.len())
        .filter(|&i| (f.mz[i] - mz).abs() <= mz * 5e-6 && (f64::from(f.rt[i]) - rt).abs() <= 0.1)
        .max_by(|&a, &b| f.height[a].total_cmp(&f.height[b]))
        .unwrap_or_else(|| panic!("no row at {mz}@{rt}"))
}

#[test]
fn the_fragment_of_a_later_product_is_not_joined_to_the_molecule() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("mzlab_feat_degdma_{}", std::process::id()));
    let ok = std::process::Command::new("python3")
        .arg(root.join("tools/feat_sintetici.py"))
        .arg(&out)
        .args(["--solo", "degdma"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        return;
    }
    let truth: Value =
        serde_json::from_slice(&std::fs::read(out.join("degdma/verita.json")).unwrap()).unwrap();
    let ions = truth["ioni"].as_array().unwrap();
    let find = |id: &str| {
        ions.iter()
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("{id}"))
    };
    let (mol, twin) = (find("SIN-019"), find("SIN-020_ISFF1"));
    let at = |r: &Value| (r["mz"].as_f64().unwrap(), r["rt_obs"][0].as_f64().unwrap());
    let run = || {
        let file = std::fs::File::open(out.join("degdma.mzidx")).unwrap();
        let size = file.metadata().unwrap().len();
        let mut ix = Index::open(file, size).unwrap();
        let p = Params::default();
        let (mut f, links) = analyse(&mut ix, &p, &Steps::default()).unwrap();
        let fam = analyse_families(&mut ix, &mut f, &links, &p, &FamParams::default()).unwrap();
        (f, fam)
    };
    let (f, fam) = run();
    let m = row(&f, at(mol).0, at(mol).1);
    let t = row(&f, at(twin).0, at(twin).1);
    assert_ne!(
        f.family_id[m], f.family_id[t],
        "the same m/z at another RT is another compound"
    );
    assert_ne!(
        f.role[t], ROLE_ISF,
        "its parent is not the molecule: no MS2 proof, no co-elution"
    );
    // every member of the molecule's family (isotopologues apart) is one of the ions of the truth
    for i in (0..f.len()).filter(|&i| f.family_id[i] == f.family_id[m] && f.role[i] != ROLE_ISOTOPE)
    {
        let ok = ions.iter().any(|r| {
            r["id"].as_str().unwrap().starts_with("SIN-019")
                && (f.mz[i] - r["mz"].as_f64().unwrap()).abs() <= f.mz[i] * 5e-6
        });
        assert!(
            ok,
            "row {:.4}@{:.2} is no ion of the molecule",
            f.mz[i], f.rt[i]
        );
    }
    let (g, gam) = run();
    assert_eq!(
        record("degdma", &f, &fam),
        record("degdma", &g, &gam),
        "second run"
    );
    let _ = std::fs::remove_dir_all(&out);
}

fn series(solo: &str) -> Option<std::path::PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("mzlab_feat_orient_{solo}_{}", std::process::id()));
    let ok = std::process::Command::new("python3")
        .arg(root.join("tools/feat_sintetici.py"))
        .arg(&out)
        .args(["--solo", solo])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        return None;
    }
    Some(out)
}

fn families(mzidx: &Path, orient: Orient) -> (FileFeatures, mzlab_feat::family::Families) {
    let file = std::fs::File::open(mzidx).unwrap();
    let size = file.metadata().unwrap().len();
    let mut ix = Index::open(file, size).unwrap();
    let p = Params::default();
    let (mut f, links) = analyse(&mut ix, &p, &Steps::default()).unwrap();
    let fam = analyse_families(
        &mut ix,
        &mut f,
        &links,
        &p,
        &FamParams {
            orient,
            ..FamParams::default()
        },
    )
    .unwrap();
    (f, fam)
}

#[test]
fn the_default_leaves_the_text_of_the_fingerprints_alone() {
    let text = format!("{:?}", FamParams::default());
    assert!(!text.contains("orient"), "{text}");
    let on = format!(
        "{:?}",
        FamParams {
            orient: Orient::Evidence,
            ..FamParams::default()
        }
    );
    assert!(on.contains("orient: Evidence"), "{on}");
}

#[test]
fn with_evidence_the_strong_adducts_and_the_base_peak_fragment_join_the_molecule() {
    let Some(out) = series("degdma") else { return };
    let truth: Value =
        serde_json::from_slice(&std::fs::read(out.join("degdma/verita.json")).unwrap()).unwrap();
    let ions = truth["ioni"].as_array().unwrap();
    let at = |id: &str| {
        let r = ions
            .iter()
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        (r["mz"].as_f64().unwrap(), r["rt_obs"][0].as_f64().unwrap())
    };
    let (f, fam) = families(&out.join("degdma.mzidx"), Orient::Evidence);
    let row_of = |id: &str| row(&f, at(id).0, at(id).1);
    let m = row_of("SIN-019");
    for id in ["SIN-019_Na", "SIN-019_K", "SIN-019_NH4", "SIN-019_ISF1"] {
        assert_eq!(
            f.family_id[row_of(id)],
            f.family_id[m],
            "{id} is in the family of the molecule"
        );
    }
    assert_eq!(f.role[row_of("SIN-019_Na")], ROLE_ADDUCT);
    assert_eq!(f.role[row_of("SIN-019_K")], ROLE_ADDUCT);
    assert_eq!(f.role[row_of("SIN-019_ISF1")], ROLE_ISF);
    let twin = row_of("SIN-020_ISFF1");
    assert_ne!(
        f.family_id[twin], f.family_id[m],
        "the same m/z at another RT stays alone"
    );
    assert_ne!(f.role[twin], ROLE_ISF);
    // the arcs say that the proof, not the factor, put them there
    let rec: Value = serde_json::from_str(&record("degdma", &f, &fam)).unwrap();
    let used = rec["families"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|x| x["arcs"].as_array().unwrap())
        .filter(|a| a["admitted_by"] == "evidence")
        .count();
    assert!(used >= 2, "{used} arcs admitted by evidence");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn evidence_adds_no_false_link_on_the_identity_series() {
    let Some(out) = series("identita") else {
        return;
    };
    let (a, _) = families(&out.join("identita.mzidx"), Orient::Factor);
    let (b, _) = families(&out.join("identita.mzidx"), Orient::Evidence);
    assert_eq!(a.family_id, b.family_id);
    assert_eq!(a.role, b.role);
    let _ = std::fs::remove_dir_all(&out);
}
