//! Tests of `spill`: a table comes back by ranges of m/z and the weak ions are the same in memory and through a spill.

use super::*;

/// Weak tables of `files` files: clusters a few ppm wide, each present in a run of files.
fn tables(files: usize) -> Vec<FileFeatures> {
    let mut out: Vec<FileFeatures> = (0..files).map(|_| FileFeatures::default()).collect();
    for c in 0..600usize {
        let (first, len) = (1 + c % 3, 2 + c % 3);
        for (k, f) in out.iter_mut().enumerate().skip(first).take(len) {
            let mz = 200.0 + c as f64 * 0.21 + k as f64 * 1e-6;
            f.mz.push(mz);
            f.rt.push(2.0 + (c % 17) as f32 * 0.3);
            f.left.push(1.9);
            f.right.push(2.1);
            f.area.push(1e5 + c as f64);
            f.height.push(1e4);
            f.flags.push(16);
        }
    }
    out
}

#[test]
fn a_table_comes_back_by_ranges_of_mz() {
    let f = &tables(3)[1];
    let b = encode(f);
    let all = decode_range(&b, f64::NEG_INFINITY, f64::INFINITY).unwrap();
    assert_eq!(
        (all.mz.clone(), all.rt.clone(), all.flags.clone()),
        (f.mz.clone(), f.rt.clone(), f.flags.clone())
    );
    let (lo, hi) = (f.mz[40], f.mz[90]);
    let part = decode_range(&b, lo, hi).unwrap();
    assert_eq!(part.mz, f.mz[40..90]);
    assert_eq!(part.area, f.area[40..90]);
    assert!(decode_range(&b[..b.len() - 1], lo, hi).is_err());
}

/// A spill that lives outside the module, like the array of the page.
struct Outside(Vec<Vec<u8>>);

impl Spill for Outside {
    fn put(&mut self, k: usize, bytes: Vec<u8>) -> Result<()> {
        self.0.put(k, bytes)
    }
    fn get(&mut self, k: usize) -> Result<Vec<u8>> {
        Spill::get(&mut self.0, k)
    }
    fn external(&self) -> bool {
        true
    }
}

#[test]
fn the_weak_ions_are_the_same_in_memory_and_through_a_spill() {
    let r = Recipe {
        weak_files: 2,
        ..Recipe::default()
    };
    let mut memory = Weak::new(None);
    for t in tables(6) {
        memory.add(t, None).unwrap();
    }
    let want = memory.recurring(&r, None).unwrap();
    assert!(want.len() > 100, "{} recurring rows", want.len());
    let budget = MemBudget::mb(0.05);
    let mut store = Outside(Vec::new());
    let mut spilled = Weak::new(Some(&mut store));
    for t in tables(6) {
        spilled.add(t, Some(&budget)).unwrap();
    }
    assert!(spilled.spilled, "the tables go to the spill");
    let got = spilled.recurring(&r, Some(&budget)).unwrap();
    assert_eq!(store.0.len(), 6);
    assert_eq!(
        (got.mz, got.rt, got.area, got.state),
        (want.mz, want.rt, want.area, want.state)
    );
    // a spill inside the module is counted on the budget: refused in order
    let mut inside: Vec<Vec<u8>> = Vec::new();
    let mut held = Weak::new(Some(&mut inside));
    let e = tables(6)
        .into_iter()
        .try_for_each(|t| held.add(t, Some(&budget)))
        .unwrap_err();
    assert_eq!(e.key, "error.memoryBudget");
    // with no spill the phase is refused in order
    let mut none = Weak::new(None);
    let e = tables(6)
        .into_iter()
        .try_for_each(|t| none.add(t, Some(&budget)))
        .unwrap_err();
    assert_eq!(e.key, "error.memoryBudget");
}
