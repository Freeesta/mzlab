//! Alignment "per family" on a synthetic series built here (invented ions with an RT drift per file, adducts, isotopologues, in-source fragments,
//! a fragment shared by two compounds, a family whose base ion is missing in some files): the families end in one row, the shared fragment does not
//! join two compounds, the table has the same bytes in two runs and matches the frozen answer in `tests/golden/feat/famalign_sintetico.json`
//! (rewrite it with `MZLAB_GOLDEN_WRITE=1`).

use std::path::Path;

use mzlab_feat::align::{align, AlignParams};
use mzlab_feat::famalign::{align_families, FamAlignParams};
use mzlab_feat::table::{
    Aligned, FileFeatures, CELL_ABSENT, CELL_DETECTED, FLAG_GOOD, ROLE_ADDUCT, ROLE_ISF,
    ROLE_ISOTOPE, ROLE_M,
};
use sha2::{Digest, Sha256};

const FILES: usize = 6;
const COMPOUNDS: usize = 120;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// An ion of a file before sorting: (m/z, RT, area, role, family key, charge).
type Ion = (f64, f32, f64, u8, usize, i8);

/// Ions of compound `c` in file `k`: base, isotopologue, Na adduct and a fragment (all with the RT of the compound plus the drift of the file).
fn compound(c: usize, k: usize, rng: &mut Lcg, mz0: f64, rt0: f64, out: &mut Vec<Ion>) {
    let rt = (rt0 + 0.04 * k as f64 + rng.next() * 0.01) as f32;
    let a = 1e7 * (0.5 + rng.next());
    let jitter = |m: f64, r: &mut Lcg| m * (1.0 + (r.next() - 0.5) * 2e-6);
    let base_missing = c.is_multiple_of(10) && k % 2 == 1;
    if !base_missing {
        out.push((jitter(mz0, rng), rt, a, ROLE_M, c, 1));
        out.push((jitter(mz0 + 1.003355, rng), rt, a * 0.2, ROLE_ISOTOPE, c, 1));
    }
    let na_role = if base_missing { ROLE_M } else { ROLE_ADDUCT };
    out.push((jitter(mz0 + 21.98194, rng), rt, a * 0.3, na_role, c, 1));
    if c.is_multiple_of(3) && !base_missing {
        out.push((jitter(mz0 - 18.0106, rng), rt, a * 0.1, ROLE_ISF, c, 1));
    }
}

/// Builds the table of file `k`: ions sorted by m/z, `family_id` = row of the root + 1.
fn file(k: usize, seed: u64) -> FileFeatures {
    let mut rng = Lcg(seed + k as u64 * 977);
    let mut ions: Vec<Ion> = Vec::new();
    for c in 0..COMPOUNDS {
        let mut r0 = Lcg(1000 + c as u64);
        let (mz0, rt0) = (200.0 + r0.next() * 400.0, 1.0 + r0.next() * 18.0);
        if rng.next() < 0.9 || c.is_multiple_of(10) {
            compound(c, k, &mut rng, mz0, rt0, &mut ions);
        }
    }
    // a fragment of two compounds (same m/z, same RT) that each file assigns to the heavier one of its own family
    for (c, shift) in [(0usize, 0.0f32), (1, 0.0)] {
        let rt = 7.0 + 0.04 * k as f32 + shift;
        ions.push((150.12345, rt, 1e6, ROLE_ISF, 1000 + c, 1));
    }
    for z in 0..2 {
        let mz0 = 400.0 + z as f64 * 50.0;
        ions.push((mz0, 7.0 + 0.04 * k as f32, 5e6, ROLE_M, 1000 + z, 1));
    }
    ions.sort_by(|a, b| a.0.total_cmp(&b.0));
    let root_of = |key: usize| ions.iter().position(|i| i.4 == key && i.3 == ROLE_M);
    let mut f = FileFeatures::default();
    for (i, ion) in ions.iter().enumerate() {
        f.mz.push(ion.0);
        f.rt.push(ion.1);
        f.left.push(ion.1 - 0.05);
        f.right.push(ion.1 + 0.05);
        f.area.push(ion.2);
        f.height.push((ion.2 / 40.0) as f32);
        f.flags.push(FLAG_GOOD);
        f.role.push(ion.3);
        f.charge.push(ion.5);
        let root = if ion.3 == ROLE_M {
            Some(i)
        } else {
            root_of(ion.4)
        };
        f.family_id.push(root.map_or(0, |r| r as u32 + 1));
    }
    f
}

fn series() -> Vec<FileFeatures> {
    (0..FILES).map(|k| file(k, 42)).collect()
}

fn sha(al: &Aligned) -> String {
    Sha256::digest(al.to_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn row_of(al: &Aligned, mz: f64) -> Vec<usize> {
    al.rows_near(mz, 5.0)
}

#[test]
fn families_end_in_one_row() {
    let files = series();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let fam = align_families(&refs, &FamAlignParams::default());
    let ion = align(&refs, &AlignParams::default());
    assert!(
        fam.len() * 100 <= ion.len() * 60,
        "{} rows against {} ions",
        fam.len(),
        ion.len()
    );
    let (mut together, mut total) = (0, 0);
    for c in (0..COMPOUNDS).filter(|c| !c.is_multiple_of(10)) {
        let mut r0 = Lcg(1000 + c as u64);
        let mz0 = 200.0 + r0.next() * 400.0;
        let base = row_of(&fam, mz0);
        for d in [1.003355, 21.98194] {
            let other = row_of(&fam, mz0 + d);
            if base.is_empty() || other.is_empty() {
                continue;
            }
            total += 1;
            together += base.iter().any(|b| other.contains(b)) as usize;
        }
    }
    assert!(
        total > 150 && together * 100 >= total * 98,
        "{together} of {total}"
    );
}

#[test]
fn shared_fragment_does_not_join_two_compounds() {
    let files = series();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let fam = align_families(&refs, &FamAlignParams::default());
    let a = row_of(&fam, 400.0);
    let b = row_of(&fam, 450.0);
    assert!(!a.is_empty() && !b.is_empty());
    assert!(
        a.iter().all(|r| !b.contains(r)),
        "the two bases share a row"
    );
}

#[test]
fn states_follow_the_peaks_and_two_runs_agree() {
    let files = series();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let fam = align_families(&refs, &FamAlignParams::default());
    for r in 0..fam.len() {
        for k in 0..FILES {
            let c = fam.cell(r, k);
            let present = !fam.apex_rt[c].is_nan();
            assert_eq!(
                fam.state[c],
                if present { CELL_DETECTED } else { CELL_ABSENT }
            );
            assert!(present || fam.area[c] == 0.0);
        }
        assert!(!fam.members(r).is_empty());
    }
    let again = align_families(&refs, &FamAlignParams::default());
    assert_eq!(fam.to_bytes(), again.to_bytes());
}

#[test]
fn matches_the_frozen_answer() {
    let files = series();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let fam = align_families(&refs, &FamAlignParams::default());
    let now =
        serde_json::json!({"rows": fam.len(), "members": fam.member_mz.len(), "sha256": sha(&fam)});
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/feat/famalign_sintetico.json");
    if std::env::var("MZLAB_GOLDEN_WRITE").is_ok() {
        std::fs::write(&p, serde_json::to_string_pretty(&now).unwrap() + "\n").unwrap();
    }
    let want: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    assert_eq!(now, want);
}
