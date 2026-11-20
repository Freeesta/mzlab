//! The parts of an edit patch in progress: its limits, the atoms it holds (`Slot`), the sets of ids it keeps and the lookup of an id.

use mzlab_core::json::Value;
use mzlab_mol::Atom;

/// Operations in one patch, bytes of a patch given as text, and bonds a drawing may reach: a patch is a short list of edits, and the
/// work of an apply is bounded by these and by `MAX_ATOMS`.
pub(super) const MAX_OPS: usize = 1000;
pub(crate) const MAX_PATCH: usize = 64 << 10;
pub(super) const MAX_BONDS: usize = 2500;

pub(super) struct Slot {
    pub(super) id: u32,
    pub(super) atom: Atom,
    pub(super) xy: Option<[f64; 2]>,
    pub(super) label: Option<String>,
}

/// A set of atom ids (at most a few thousand): a list without repeats.
#[derive(Default)]
pub(super) struct Ids(pub(super) Vec<u32>);

impl Ids {
    pub(super) fn add(&mut self, x: u32) {
        if !self.0.contains(&x) {
            self.0.push(x);
        }
    }

    pub(super) fn has(&self, x: u32) -> bool {
        self.0.contains(&x)
    }
}

#[derive(Default)]
pub(super) struct Sets {
    /// Atoms whose bonds or bond orders changed: their stereo goes and their hydrogens are counted again.
    pub(super) shape: Ids,
    /// Atoms whose element, charge or radical changed: their hydrogens are counted again.
    pub(super) atom: Ids,
    /// Atoms whose hydrogens the patch states.
    pub(super) h_given: Ids,
}

/// The place of each atom id in the slots: the ids and places packed in one sorted list, searched by bisection.
pub(super) struct Index(Vec<u64>);

/// Ascending order of `v`, by a plain merge sort (the one sort of this module: the library sorts would each add a copy to the `.wasm`).
fn sort64(v: &mut Vec<u64>) {
    let n = v.len();
    let mut buf = v.clone();
    let mut w = 1;
    while w < n {
        for i in (0..n).step_by(2 * w) {
            let (m, e) = ((i + w).min(n), (i + 2 * w).min(n));
            let (mut a, mut b) = (i, m);
            for k in i..e {
                if a < m && (b >= e || v[a] <= v[b]) {
                    buf[k] = v[a];
                    a += 1;
                } else {
                    buf[k] = v[b];
                    b += 1;
                }
            }
        }
        std::mem::swap(v, &mut buf);
        w *= 2;
    }
}

impl Index {
    pub(super) fn of(slots: &[Slot]) -> Index {
        let mut v: Vec<u64> = slots
            .iter()
            .enumerate()
            .map(|(i, s)| u64::from(s.id) << 32 | i as u64)
            .collect();
        sort64(&mut v);
        Index(v)
    }

    pub(super) fn get(&self, id: u32) -> Option<u32> {
        let i = self.0.partition_point(|&k| ((k >> 32) as u32) < id);
        let k = *self.0.get(i)?;
        ((k >> 32) as u32 == id).then_some(k as u32)
    }
}

/// The size of a patch given as a list, counted as its compact JSON text with a number as 8 bytes: the limit `MAX_PATCH` is the one of the
/// text form.
pub(super) fn weight(v: &Value) -> usize {
    match v {
        Value::Str(s) => s.len() + 2,
        Value::Arr(a) => 2 + a.iter().map(|x| weight(x) + 1).sum::<usize>(),
        Value::Obj(e) => {
            2 + e
                .iter()
                .map(|(k, x)| k.len() + 4 + weight(x))
                .sum::<usize>()
        }
        _ => 8,
    }
}
