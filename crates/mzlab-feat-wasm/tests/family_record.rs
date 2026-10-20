//! `run::family_record` (behind the export `family_record`) against the native path of `examples/famiglie.rs` on the hard synthetic series of
//! `tools/valida/sintetici_degdma.py`: same record byte for byte, same bytes on a second run, whatever the reader, and an orderly refusal when the
//! budget holds nothing. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).
#![cfg(feature = "detect")]

use std::path::Path;

use mzlab_core::budget::MemBudget;
use mzlab_core::idxread::Index;
use mzlab_feat::family::{analyse_families, FamParams};
use mzlab_feat::famrecord;
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::trace::Params;
use mzlab_feat_wasm::{family, run};

fn make() -> Option<(std::path::PathBuf, Vec<u8>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("mzlab_family_record_{}", std::process::id()));
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
        return None;
    }
    let bytes = std::fs::read(out.join("degdma.mzidx")).unwrap();
    Some((out, bytes))
}

#[test]
fn the_record_equals_the_native_one_and_is_deterministic() {
    let Some((dir, bytes)) = make() else { return };
    let open = || Index::open(bytes.clone(), bytes.len() as u64).unwrap();
    let recipe = |ix: &mut Index<Vec<u8>>| run::recipe_for("", ix).unwrap();

    let mut ix = open();
    let r = recipe(&mut ix);
    let got = family::record(&mut ix, "degdma", &r, &MemBudget::mb(192.0)).unwrap();

    // the native path of `examples/famiglie.rs`, with the numbers of the same recipe
    let mut ix = open();
    let (mut f, links) = analyse(&mut ix, &r.detect, &r.steps).unwrap();
    let fam = analyse_families(&mut ix, &mut f, &links, &r.detect, &r.fam).unwrap();
    let want = famrecord::record("degdma", &f, &fam);
    assert!(got.contains("\"schema\":\"famiglia/1\""));
    assert!(!fam.arcs.is_empty(), "the series has arcs to prove");
    assert_eq!(got, want, "wasm path and native path");

    // exactly what `examples/famiglie.rs` runs: default numbers, no recipe
    let mut ix = open();
    let (p, steps) = (Params::default(), Steps::default());
    let (mut f, links) = analyse(&mut ix, &p, &steps).unwrap();
    let fam = analyse_families(&mut ix, &mut f, &links, &p, &FamParams::default()).unwrap();
    let mut ix = open();
    let r0 = run::recipe_from_json("").unwrap();
    assert_eq!(
        family::record(&mut ix, "degdma", &r0, &MemBudget::mb(192.0)).unwrap(),
        famrecord::record("degdma", &f, &fam),
        "the default recipe is the example"
    );

    let mut ix = open();
    let r = recipe(&mut ix);
    let again = family::record(&mut ix, "degdma", &r, &MemBudget::mb(192.0)).unwrap();
    assert_eq!(got, again, "second run");

    let mut ix = open();
    let err = family::record(&mut ix, "degdma", &r, &MemBudget::mb(0.001)).unwrap_err();
    assert_eq!(err.key, "error.memoryBudget");
    let _ = std::fs::remove_dir_all(&dir);
}
