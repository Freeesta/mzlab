//! `run::align_to_spilling` handed the facts of the files (as `series_js::series_recipe` reads them, from the indexes it already parsed) writes the same
//! bytes as `run::align_to`, which opens every index again for them. The synthetic series is the one of `tools/bench/oro_mzmine.py --genera` (python3
//! with numpy; skipped without).
#![cfg(feature = "detect")]

use std::path::Path;
use std::process::Command;

use mzlab_core::budget::MemBudget;
use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::facts::file_facts;
use mzlab_feat_wasm::run;

fn index_bytes(path: &Path) -> Vec<u8> {
    let mzml = std::fs::read(path).unwrap();
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "x.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

fn series() -> Option<Vec<Vec<u8>>> {
    let dir = std::env::temp_dir().join(format!("mzlab_align_facts_{}", std::process::id()));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/bench/oro_mzmine.py"))
        .arg("--genera")
        .arg(&dir)
        .current_dir(&root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("SKIP align_facts: python3/numpy not available");
        return None;
    }
    let v = (1..=4)
        .map(|i| index_bytes(&dir.join(format!("oro_{i}.mzML"))))
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    Some(v)
}

#[test]
fn facts_handed_in_give_the_bytes_of_facts_read_again() {
    let Some(ix) = series() else { return };
    let open = |k: usize| Index::open(ix[k].clone(), ix[k].len() as u64);
    let r = run::recipe_for("{}", &mut open(0).unwrap()).unwrap();
    let mut stored: Vec<Vec<u8>> = (0..ix.len())
        .map(|k| run::detect(&mut open(k).unwrap(), &format!("oro_{k}"), &r).unwrap())
        .collect();
    let budget = MemBudget::from_request(f64::NAN);
    let mut again = Vec::new();
    run::align_to(&mut stored, &budget, &r, open, &mut again).unwrap();
    let facts = (0..ix.len())
        .map(|k| file_facts(&open(k).unwrap()))
        .collect();
    let mut handed = Vec::new();
    run::align_to_spilling(
        &mut stored,
        &budget,
        &r,
        Some(facts),
        open,
        &mut handed,
        None,
        &mut |_, _| {},
    )
    .unwrap();
    assert!(!again.is_empty());
    assert_eq!(again, handed);
}
