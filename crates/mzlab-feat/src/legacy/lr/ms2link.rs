//! The MS2 matches of the low-resolution Dig (`ms2_links/1`): every product-ion experiment of the series (file, polarity, precursor, collision
//! energy) is matched once, after the Dig, to the rows it belongs to by the rules of `docs/studi/TABUI_MS2_LR.md` §4. A row takes the scans of
//! the peaks of the TRACES of its experiments (never a window of its own cells); the user can correct a match (`relink`).
//! `exp` builds the experiments and their traces, `rules` matches them, `json` writes the contract.

mod exp;
mod follow;
mod json;
mod rules;

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Value;

use super::{ms2target, LrDig};
use crate::legacy::Res;
pub use exp::{Exp, PeakL};
pub use json::json;

/// Half the Q1 window of a unit-resolution instrument: a precursor this close to the m/z of a row is that row's, Da.
pub const PREC_TOL: f64 = 0.5;
/// A peak of the trace of an experiment is the ion of a row from this S/N.
pub const SN_LINK: f64 = 3.0;
/// The peaks of a trace that are listed.
pub const SN_PEAK: f64 = 5.0;
/// The matches that measure the drift of a file have at least this S/N.
pub const SN_DRIFT: f64 = 10.0;
/// An experiment with fewer scans than this outside the window of a row was acquired only around the row.
pub const FEW_OUTSIDE: usize = 10;
/// Mass differences of the first isotope peaks, Da.
pub const ISO: [f64; 2] = [1.00335, 2.0067];

/// How a row got the peak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Auto,
    Ambiguous,
    Isotope,
    Isf,
    User,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Auto => "auto",
            State::Ambiguous => "ambiguous",
            State::Isotope => "isotope",
            State::Isf => "isf",
            State::User => "user",
        }
    }
    /// Whether the scans of the peak go to the pool of the row.
    pub fn pooled(self) -> bool {
        self != State::Isotope
    }
}

/// A peak of an experiment given to a row.
#[derive(Debug, Clone)]
pub struct Link {
    pub exp: usize,
    pub peak: usize,
    /// Id of the row (`Entry.id`).
    pub row: i64,
    pub state: State,
    pub dmz: f64,
    pub drt: f64,
    pub alternatives: Vec<i64>,
    pub why: String,
}

/// An experiment, or one peak of it, that no row got.
#[derive(Debug, Clone)]
pub struct Unmatched {
    pub exp: Option<usize>,
    pub peak: Option<usize>,
    pub reason: &'static str,
    pub rows: Vec<i64>,
    pub why: String,
}

/// A correction of the user: the peak of `file` at `precursor`, `ce` and `apex` belongs to `row` (`None`: to no row).
#[derive(Debug, Clone)]
pub struct Override {
    pub file: String,
    pub precursor: f64,
    pub ce: Option<f64>,
    pub apex: f64,
    pub row: Option<i64>,
}

/// The matches of a Dig.
#[derive(Debug, Clone, Default)]
pub struct Links {
    pub exps: Vec<Exp>,
    pub links: Vec<Link>,
    pub unmatched: Vec<Unmatched>,
    pub parent_has_ms2: bool,
    pub rt_tol: f64,
}

impl Links {
    /// The links whose scans go to the pool of the row `id`.
    pub fn of(&self, id: i64) -> impl Iterator<Item = (usize, &Link)> {
        self.links
            .iter()
            .enumerate()
            .filter(move |(_, l)| l.row == id && l.state.pooled())
    }
}

/// Stable insertion sort for the few items of this module (the instantiation of a merge sort per type and closure costs several times more `.wasm`).
pub(super) fn small<T: Copy>(v: &mut [T], mut cmp: impl FnMut(&T, &T) -> std::cmp::Ordering) {
    for i in 1..v.len() {
        let x = v[i];
        let mut j = i;
        while j > 0 && cmp(&v[j - 1], &x) == std::cmp::Ordering::Greater {
            v[j] = v[j - 1];
            j -= 1;
        }
        v[j] = x;
    }
}

/// The values of `sorted` (ordered by key) that share a key, in order: a map that costs less `.wasm` than a `BTreeMap` per type.
pub(super) fn runs<K: Copy + PartialEq, V: Copy>(sorted: &[(K, V)]) -> Vec<(K, Vec<V>)> {
    let mut out: Vec<(K, Vec<V>)> = Vec::new();
    for (k, v) in sorted {
        match out.last_mut() {
            Some((last, vs)) if last == k => vs.push(*v),
            _ => out.push((*k, vec![*v])),
        }
    }
    out
}

/// The corrections of the page, `[{file, precursor, ce, apex, row}]` (`row` null or absent: not this ion).
pub fn overrides(text: &str) -> Res<Vec<Override>> {
    let v = mzlab_core::json::parse(text.as_bytes()).map_err(|e| e.to_string())?;
    let num = |x: &Value, k: &str| x.get(k).and_then(Value::f64);
    Ok(v.arr()
        .iter()
        .filter_map(|x| {
            Some(Override {
                file: x.get("file")?.str()?.to_string(),
                precursor: num(x, "precursor")?,
                ce: num(x, "ce"),
                apex: num(x, "apex")?,
                row: num(x, "row").map(|r| r as i64),
            })
        })
        .collect())
}

/// Matches the experiments of the series to the rows of `d`. `open(k)` gives the index of file `k` of the columns then the MS2 files.
pub fn compute<R: ReadAt>(
    d: &LrDig,
    ov: &[Override],
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Links> {
    let (exps, polarity) = exp::experiments(d, open)?;
    rules::link(d, exps, polarity, ov, open)
}

/// `compute` and keep the result in `d` (`pool`, `summary` and `detail` read it). The peaks that no row got and that have a feature in the table
/// make a row each (`ms2target`) and the matches are computed again with them.
pub fn ms2_link<R: ReadAt>(
    d: &mut LrDig,
    ov: &[Override],
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<()> {
    ms2target::clear(d);
    let mut l = compute(d, ov, open)?;
    let had_isf = !d.isf.is_empty();
    let made = ms2target::make(d, &l);
    if !made.is_empty() {
        if had_isf {
            ms2target::verdicts(d, open)?;
        }
        l = compute(d, ov, open)?;
    }
    ms2target::finish_links(d, &mut l, &made);
    d.links = Some(l);
    Ok(())
}
