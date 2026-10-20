//! The weak tier of a series out of the linear memory: the weak table of each file is encoded and handed to a `Spill` (in the browser an array of the
//! worker's JavaScript heap, outside the memory of the module), and read back by ranges of m/z to be aligned in slabs. Used by `series::finish`.

use crate::align::align;
use crate::recipe::Recipe;
use crate::slabs::{cuts, LEAN_ROW, WORK_ROW};
use crate::table::{Aligned, FileFeatures};
use mzlab_core::budget::MemBudget;
use mzlab_core::error::{Error, Result};

/// Where the weak tables of the files wait while the others are made: the page keeps them (JavaScript) and gives a copy when asked.
pub trait Spill {
    /// Keeps `bytes` as the table of file `k` (replacing what was there).
    fn put(&mut self, k: usize, bytes: Vec<u8>) -> Result<()>;
    /// A copy of what `put` kept for file `k`.
    fn get(&mut self, k: usize) -> Result<Vec<u8>>;
    /// True if the bytes live outside the memory of the module (they are not counted on the budget).
    fn external(&self) -> bool {
        false
    }
}

impl Spill for Vec<Vec<u8>> {
    fn put(&mut self, k: usize, bytes: Vec<u8>) -> Result<()> {
        if self.len() <= k {
            self.resize(k + 1, Vec::new());
        }
        self[k] = bytes;
        Ok(())
    }
    fn get(&mut self, k: usize) -> Result<Vec<u8>> {
        self.as_slice().get(k).cloned().ok_or_else(corrupt)
    }
}

fn corrupt() -> Error {
    crate::store::bad()
}

/// Bytes a weak row takes in memory (`push_lean` columns), and in the encoding below.
pub(crate) const WEAK_ROW: usize = 29;
const HEAD: usize = 17;
const ROW: usize = 8 + 4 + 4 + 4 + 8 + 4 + 1;

/// The lean table of a file in a fixed layout sorted by m/z (a stable sort, as the alignment does): `n` (u64), `dt` (f64), `q3` present (u8), then the
/// columns m/z, RT, left, right, area, height, flags and, if present, q3.
pub(crate) fn encode(f: &FileFeatures) -> Vec<u8> {
    let n = f.len();
    let mut order: Vec<u32> = (0..n as u32).collect();
    if !f.mz.windows(2).all(|w| w[0] <= w[1]) {
        crate::sortc::by(&mut order, |&a, &b| {
            f.mz[a as usize].total_cmp(&f.mz[b as usize])
        });
    }
    let q3 = !f.q3.is_empty();
    let mut b = Vec::with_capacity(HEAD + n * (ROW + 8 * q3 as usize));
    b.extend_from_slice(&(n as u64).to_le_bytes());
    b.extend_from_slice(&f.dt.to_le_bytes());
    b.push(q3 as u8);
    macro_rules! put {
        ($($c:ident),*) => { $( for &i in &order { b.extend_from_slice(&f.$c[i as usize].to_le_bytes()); } )* };
    }
    put!(mz, rt, left, right, area, height);
    for &i in &order {
        b.push(f.flags[i as usize]);
    }
    if q3 {
        put!(q3);
    }
    b
}

/// The rows of an encoded table with m/z in `[lo, hi)`.
pub(crate) fn decode_range(b: &[u8], lo: f64, hi: f64) -> Result<FileFeatures> {
    if b.len() < HEAD {
        return Err(corrupt());
    }
    let n = u64::from_le_bytes(b[0..8].try_into().unwrap()) as usize;
    let q3 = b[16] != 0;
    if b.len() != HEAD + n * (ROW + 8 * q3 as usize) {
        return Err(corrupt());
    }
    let col = |at: usize, w: usize| HEAD + at * n + w;
    let mz_at =
        |i: usize| f64::from_le_bytes(b[HEAD + i * 8..HEAD + i * 8 + 8].try_into().unwrap());
    let from = partition(n, |i| mz_at(i) < lo);
    let to = partition(n, |i| mz_at(i) < hi);
    let mut f = FileFeatures {
        dt: f64::from_le_bytes(b[8..16].try_into().unwrap()),
        ..FileFeatures::default()
    };
    // the columns follow one another: f64 m/z, then f32 RT, left, right, f64 area, f32 height, u8 flags, f64 q3
    let offs = [0, 8, 12, 16, 20, 28, 32, 33];
    let read8 =
        |o: usize, i: usize| f64::from_le_bytes(b[col(o, i * 8)..][..8].try_into().unwrap());
    let read4 =
        |o: usize, i: usize| f32::from_le_bytes(b[col(o, i * 4)..][..4].try_into().unwrap());
    for i in from..to {
        f.mz.push(read8(offs[0], i));
        f.rt.push(read4(offs[1], i));
        f.left.push(read4(offs[2], i));
        f.right.push(read4(offs[3], i));
        f.area.push(read8(offs[4], i));
        f.height.push(read4(offs[5], i));
        f.flags.push(b[col(offs[6], i)]);
        if q3 {
            f.q3.push(read8(offs[7], i));
        }
    }
    Ok(f)
}

/// The weak tables of the files, in memory or in a `Spill`.
pub(crate) struct Weak<'a> {
    held: Vec<FileFeatures>,
    count: usize,
    bytes: usize,
    spill: Option<&'a mut dyn Spill>,
    spilled: bool,
    /// m/z of every weak row (to cut the slabs) and bytes the spill keeps inside the module.
    all: Vec<f64>,
    inside: usize,
}

impl<'a> Weak<'a> {
    pub(crate) fn new(spill: Option<&'a mut dyn Spill>) -> Weak<'a> {
        Weak {
            held: Vec::new(),
            count: 0,
            bytes: 0,
            spill,
            spilled: false,
            all: Vec::new(),
            inside: 0,
        }
    }

    /// Takes the weak table of the next file. The tables of all the files are held together in memory (29 bytes a row) until that passes half of what is
    /// free; then they go to the spill, or, with none, the phase is refused in order.
    pub(crate) fn add(&mut self, w: FileFeatures, budget: Option<&MemBudget>) -> Result<()> {
        self.bytes += w.len() * WEAK_ROW;
        let Some(b) = budget else {
            self.push(w);
            return Ok(());
        };
        if !self.spilled && self.bytes > b.left() / 2 {
            if self.spill.is_none() {
                return Err(b.refuse("align", self.bytes * 2));
            }
            self.spilled = true;
            self.count = 0;
            for t in std::mem::take(&mut self.held) {
                self.send(&t, b)?;
            }
        }
        if self.spilled {
            return self.send(&w, b);
        }
        self.push(w);
        Ok(())
    }

    fn push(&mut self, w: FileFeatures) {
        self.held.push(w);
        self.count += 1;
    }

    fn send(&mut self, t: &FileFeatures, b: &MemBudget) -> Result<()> {
        let bytes = encode(t);
        self.all
            .try_reserve(t.len())
            .map_err(|_| b.refuse("align", (self.all.len() + t.len()) * 8))?;
        self.all.extend_from_slice(&t.mz);
        let spill = self.spill.as_deref_mut().expect("spilled has a spill");
        if !spill.external() {
            self.inside += bytes.len();
            if self.inside + self.all.len() * 8 > b.left() / 2 {
                return Err(b.refuse("align", (self.inside + self.all.len() * 8) * 2));
            }
        }
        spill.put(self.count, bytes)?;
        self.count += 1;
        Ok(())
    }
}

/// First index in `0..n` where `before` is false (`before` is true and then false).
fn partition(n: usize, before: impl Fn(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (0, n);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if before(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

impl<'a> Weak<'a> {
    /// The weak ions of the series that recur (`retain_recurring`): the tables aligned all together, or, once spilled, slab by slab.
    pub(crate) fn recurring(mut self, r: &Recipe, budget: Option<&MemBudget>) -> Result<Aligned> {
        let n = self.count;
        if !self.spilled {
            let refs: Vec<&FileFeatures> = self.held.iter().collect();
            let mut ions = align(&refs, &r.align.ion)?;
            ions.retain_recurring(r.weak_files);
            return Ok(ions);
        }
        let b = budget.expect("spilled has a budget");
        let spill = self.spill.take().expect("spilled has a spill");
        let mut all = std::mem::take(&mut self.all);
        let row = LEAN_ROW + WORK_ROW;
        let cuts = cuts(&mut all, (b.left() / 4 / row).max(16), r.align.ion.ppm);
        drop(all);
        let mut out = Aligned {
            n_files: n,
            ..Aligned::default()
        };
        for w in cuts.windows(2) {
            let files = (0..n)
                .map(|k| decode_range(&spill.get(k)?, w[0], w[1]))
                .collect::<Result<Vec<_>>>()?;
            let rows: usize = files.iter().map(FileFeatures::len).sum();
            let _g = b.reserve("align", rows * row)?;
            let refs: Vec<&FileFeatures> = files.iter().collect();
            let mut ions = align(&refs, &r.align.ion)?;
            ions.retain_recurring(r.weak_files);
            out.append(&ions);
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "spill_tests.rs"]
mod tests;
