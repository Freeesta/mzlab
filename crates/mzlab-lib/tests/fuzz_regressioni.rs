//! The entry points of the cargo-fuzz targets (`fuzz/ingressi.rs`) in the ordinary tests: the seeds and the stored crashes (`fuzz/regressioni/<target>/*`) must return,
//! never panic. The fuzz run itself is not part of the mandatory CI (`.github/workflows/fuzz.yml`). Lives here because this crate sees both `mzlab-core` and `mzlab-lib`.

#[path = "../../../fuzz/ingressi.rs"]
#[allow(dead_code)]
mod ingressi;

use std::path::Path;

const MZML: &[u8] = include_bytes!("../../../tests/golden/ref/msntree_rotta/msn.mzML");
const MZIDX: &[u8] = include_bytes!("../../../tests/golden/ref/msntree_rotta/msn.mzidx");

type Entry = fn(&[u8]);

const ENTRIES: [(&str, Entry); 7] = [
    ("ole", ingressi::ole),
    ("xmltree_methodinfo", ingressi::xmltree_methodinfo),
    ("mzmlhead", ingressi::mzmlhead),
    ("idxread", ingressi::idxread),
    ("raw", ingressi::raw),
    ("wiff", ingressi::wiff),
    ("mzlib_msp_mgf", ingressi::mzlib_msp_mgf),
];

#[test]
fn every_entry_point_survives_the_seeds_and_degenerate_inputs() {
    let msp = b"\0Name: A\nPrecursorMZ: 100.5\nNum Peaks: 2\n50 10\n60 20\n\n";
    for (name, f) in ENTRIES {
        for seed in [MZML, MZIDX, msp.as_slice(), &[], &[0], &[0xFF; 64]] {
            f(seed);
            // a truncation at every tenth of the seed
            for k in 1..10 {
                f(&seed[..seed.len() * k / 10]);
            }
        }
        eprintln!("{name}: ok");
    }
}

#[test]
fn the_stored_crashes_no_longer_crash() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/regressioni");
    for (name, f) in ENTRIES {
        let Ok(dir) = std::fs::read_dir(root.join(name)) else {
            continue;
        };
        for e in dir.flatten() {
            if let Ok(b) = std::fs::read(e.path()) {
                f(&b);
            }
        }
    }
}
