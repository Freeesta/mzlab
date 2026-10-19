//! The pass over the table: reads each file of the series once (the MS2 scans of the candidate rows and of the parent, the purity, the MS1 at the
//! apex), calibrates every file on the parent and its known fragments, then interprets each row. `build` is the entry point.

use mzlab_core::elements::{mass, parse_formula, Formula};
use mzlab_core::idxread::{Index, ReadAt};

use super::albero::{self, LibNode, NodeState};
use super::confronto::{self, VsParent};
use super::filtri::{self, FileCal, RowFacts};
use super::modifica::{self, Candidate, IsoCheck};
use super::ms2link::{self, Isomers, Ms2Scan, Pooled};
use crate::axis::half;
use crate::legacy::ms2::Consensus;
use crate::legacy::msn::{self, library};
use crate::legacy::series::{Hyp, Parent};
use crate::sweep::windows;
use crate::table::{Aligned, CELL_ABSENT};
use crate::tp::{NO_PARENT, ST_PARENT, ST_TP, ST_UNEXPLAINED};

pub type Res<T> = Result<T, String>;

/// What the pass reads.
pub struct Inputs<'a> {
    pub table: &'a Aligned,
    pub parent: &'a Parent,
    /// The registry `ipotesi/1` as a list.
    pub hyps: &'a [Hyp],
    pub polarity: i8,
}

/// The interpretation of one row.
pub struct RowInterp {
    pub row: usize,
    pub id: u32,
    pub change: Vec<Candidate>,
    pub iso: Option<IsoCheck>,
    /// Observed m/z of the monoisotopic peak at the apex and the file where it was read.
    pub mz_obs: Option<(f64, usize)>,
    pub scans: Vec<Ms2Scan>,
    pub files_with_ms2: usize,
    pub pooled: Pooled,
    pub vs_parent: Option<VsParent>,
    pub tree: Vec<NodeState>,
    /// The code of the filter that hid the row after the interpretation (F5).
    pub hidden: Option<&'static str>,
}

pub struct Output {
    pub rows: Vec<RowInterp>,
    pub registry: filtri::Registry,
    pub cals: Vec<FileCal>,
    pub nodes: Vec<LibNode>,
    pub entries: Vec<LibNode>,
    pub n_files: usize,
}

/// A row worth interpreting and where its peak lies in each file.
struct Work {
    row: usize,
    limits: Vec<(f64, f64)>,
    best: usize,
    scans: Vec<Ms2Scan>,
    iso: Option<([f64; 3], f64)>,
}

fn row_facts(t: &Aligned) -> Vec<RowFacts> {
    let mut first: std::collections::HashMap<u32, usize> = Default::default();
    if !t.iso.is_empty() {
        for (r, &g) in t.iso.group.iter().enumerate() {
            if g != 0 {
                first.entry(g).or_insert(r);
            }
        }
    }
    (0..t.len())
        .map(|r| RowFacts {
            bad_quality: !t.meta.is_empty() && t.meta.get(r).qreason != 0,
            isotope: !t.iso.is_empty() && t.iso.group[r] != 0 && first[&t.iso.group[r]] != r,
            tp_status: t.tp.status[r],
            mass_error: false,
        })
        .collect()
}

/// The window of the peak of `row` in file `k`: the edges of the row around the apex of that file; `NaN` where the row has no peak there.
fn limits(t: &Aligned, row: usize, k: usize) -> (f64, f64) {
    let cell = row * t.n_files + k;
    if t.state[cell] == CELL_ABSENT {
        return (f64::NAN, f64::NAN);
    }
    let apex = if t.apex_rt[cell].is_finite() {
        f64::from(t.apex_rt[cell])
    } else {
        t.rt[row]
    };
    let (lo, hi) = if t.meta.is_empty() {
        (0.05, 0.05)
    } else {
        let q = t.meta.get(row);
        (
            t.rt[row] - f64::from(q.rt_lo),
            f64::from(q.rt_hi) - t.rt[row],
        )
    };
    let ok = |x: f64| if x.is_finite() && x > 0.0 { x } else { 0.05 };
    (apex - ok(lo), apex + ok(hi))
}

fn best_file(t: &Aligned, row: usize) -> usize {
    (0..t.n_files)
        .filter(|&k| t.state[row * t.n_files + k] != CELL_ABSENT)
        .max_by(|&a, &b| t.area[row * t.n_files + a].total_cmp(&t.area[row * t.n_files + b]))
        .unwrap_or(0)
}

/// Intensities of M, M+1 and M+2 and the m/z of M in the MS1 scan nearest to `apex` (singly charged, as everything in HR here).
fn apex_ms1<R: ReadAt>(
    ix: &mut Index<R>,
    pol: i8,
    mz: f64,
    apex: f64,
) -> Res<Option<([f64; 3], f64)>> {
    let Some(si) = ix.section_of(1, Some(pol)) else {
        return Ok(None);
    };
    let rt = ix.arrays(si).map_err(|e| e.to_string())?.rt.clone();
    if rt.is_empty() {
        return Ok(None);
    }
    let p = rt.partition_point(|&t| t < apex);
    let p = if p > 0 && (p == rt.len() || apex - rt[p - 1] < rt[p] - apex) {
        p - 1
    } else {
        p.min(rt.len() - 1)
    };
    let mut out = ([0.0; 3], f64::NAN);
    let tol = half(mz, 10.0);
    let c13 = 1.003355;
    windows(ix, si, &[(mz - tol, mz + 2.2)], 1 << 20, |_, a, b, slab| {
        for j in (a..b).filter(|&j| slab.pos[j] == p as u32) {
            let d = slab.mz[j] - mz;
            if d.abs() <= tol && slab.it[j] > out.0[0] {
                out.0[0] = slab.it[j];
                out.1 = slab.mz[j];
            } else if (d - c13).abs() <= 0.01 {
                out.0[1] = out.0[1].max(slab.it[j]);
            } else if (d - 2.0).abs() <= 0.015 {
                out.0[2] += slab.it[j];
            }
        }
        Ok(())
    })
    .map_err(|e| e.to_string())?;
    Ok(Some(out))
}

fn ppm(obs: f64, theo: f64) -> f64 {
    (obs - theo) / theo * 1e6
}

/// Errors in ppm of the peaks of `scans` that match an entry of the library (at least 5 % of the base peak of the scan).
fn fragment_errors(scans: &[Ms2Scan], entries: &[LibNode]) -> Vec<f64> {
    let mut out = vec![];
    for s in scans {
        let top = s.it.iter().copied().fold(0.0, f64::max);
        for m in
            s.mz.iter()
                .zip(&s.it)
                .filter(|(_, i)| **i >= 0.05 * top)
                .map(|x| x.0)
        {
            if let Some(e) = entries.iter().find(|e| (m - e.mz).abs() <= e.mz * 10e-6) {
                out.push(ppm(*m, e.mz));
            }
        }
    }
    out
}

fn delta_mass(parent: &Formula, cand: Option<&Candidate>, mz: f64, parent_mz: f64) -> f64 {
    match cand.and_then(|c| parse_formula(&c.neutral_formula).ok()) {
        Some(f) => mass(&f) - mass(parent),
        None => mz - parent_mz,
    }
}

/// Reads the file `k`: fills the scans of the work items and the parent, and the MS1 at the apex. Returns the parent scans and the parent error.
fn read_file<R: ReadAt>(
    ix: &mut Index<R>,
    k: usize,
    inp: &Inputs,
    work: &mut [Work],
    parent_row: Option<usize>,
) -> Res<(Vec<Ms2Scan>, Option<f64>)> {
    let t = inp.table;
    let mut all: Vec<(usize, Ms2Scan)> = vec![];
    let rows = work
        .iter()
        .map(|w| w.row)
        .chain(parent_row.map(|_| usize::MAX));
    for (wi, row) in rows.enumerate() {
        let (r, lim) = match (row, parent_row) {
            (usize::MAX, Some(p)) => (p, limits(t, p, k)),
            _ => (row, work[wi].limits[k]),
        };
        if lim.0.is_finite() {
            all.extend(
                ms2link::pick_scans(ix, k, t.mz[r], lim)
                    .into_iter()
                    .map(|s| (wi, s)),
            );
        }
    }
    let mut scans: Vec<Ms2Scan> = all.iter().map(|x| x.1.clone()).collect();
    ms2link::fill_purity(ix, inp.polarity, &mut scans).map_err(|e| e.to_string())?;
    ms2link::fill_peaks(ix, &mut scans).map_err(|e| e.to_string())?;
    let mut parent_scans = vec![];
    for ((wi, _), s) in all.iter().zip(scans) {
        match work.get_mut(*wi) {
            Some(w) => w.scans.push(s),
            None => parent_scans.push(s),
        }
    }
    for w in work.iter_mut().filter(|w| w.best == k) {
        let cell = w.row * t.n_files + k;
        let apex = if t.apex_rt[cell].is_finite() {
            f64::from(t.apex_rt[cell])
        } else {
            t.rt[w.row]
        };
        w.iso = apex_ms1(ix, inp.polarity, t.mz[w.row], apex)?;
    }
    let perr = match parent_row.filter(|&p| limits(t, p, k).0.is_finite()) {
        Some(p) => {
            let apex = f64::from(t.apex_rt[p * t.n_files + k]);
            let apex = if apex.is_finite() { apex } else { t.rt[p] };
            apex_ms1(ix, inp.polarity, inp.parent.mz, apex)?
                .filter(|m| m.1.is_finite())
                .map(|m| ppm(m.1, inp.parent.mz))
        }
        None => None,
    };
    Ok((parent_scans, perr))
}

/// The tree of the infusion and the library with the DDA of the parent: `(nodes, entries)`, empty without an infusion.
fn library_of<R: ReadAt>(
    infusion: Option<&mut Index<R>>,
    inp: &Inputs,
    dda: &Consensus,
) -> Res<(Vec<LibNode>, Vec<LibNode>)> {
    let Some(ix) = infusion else {
        return Ok((vec![], vec![]));
    };
    let scans = msn::scans_of(ix)?;
    let ion = mzlab_core::elements::fmt(&inp.parent.ion);
    let tree = msn::build_tree(&scans, &ion, inp.parent.smiles.as_deref(), "auto")?;
    let lib = library::build(
        &tree,
        (!dda.mz.is_empty()).then_some((&dda.mz[..], &dda.rel[..])),
    );
    let entries = lib
        .entries
        .iter()
        .map(|(f, e)| LibNode {
            id: String::new(),
            formula: f.clone(),
            mz: msn::ion_mass(&e.vec, &tree.els),
        })
        .collect();
    Ok((albero::nodes_of(&tree), entries))
}

/// The whole pass. `open(k)` gives the index of file `k` of the series; `infusion` the one of the infusion file (role `infusion`), when there is one.
pub fn build<R: ReadAt>(
    inp: &Inputs,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
    infusion: Option<&mut Index<R>>,
) -> Res<Output> {
    let t = inp.table;
    if t.tp.is_empty() {
        return Err("error.interp.noTp".into());
    }
    let mut facts = row_facts(t);
    let mut registry = filtri::Registry::default();
    let mut work: Vec<Work> = vec![];
    for (r, f) in facts.iter().enumerate() {
        match filtri::first_hidden(f) {
            Some(i) if t.tp.status[r] != ST_PARENT => registry.hidden[i].push(t.id(r)),
            None if t.tp.status[r] == ST_TP || t.tp.status[r] == ST_UNEXPLAINED => {
                work.push(Work {
                    row: r,
                    limits: (0..t.n_files).map(|k| limits(t, r, k)).collect(),
                    best: best_file(t, r),
                    scans: vec![],
                    iso: None,
                })
            }
            _ => {}
        }
    }
    let parent_row = (t.tp.parent_row != NO_PARENT).then_some(t.tp.parent_row as usize);
    let (mut parent_scans, mut perr) = (vec![], vec![None; t.n_files]);
    for (k, slot) in perr.iter_mut().enumerate() {
        let mut ix = open(k)?;
        let (ps, pe) = read_file(&mut ix, k, inp, &mut work, parent_row)?;
        parent_scans.push(ps);
        *slot = pe;
    }
    let all_parent: Vec<Ms2Scan> = parent_scans.iter().flatten().cloned().collect();
    let dda = ms2link::pool(&all_parent).consensus;
    let (nodes, entries) = library_of(infusion, inp, &dda)?;
    let cals: Vec<FileCal> = (0..t.n_files)
        .map(|k| {
            let mut e = fragment_errors(&parent_scans[k], &entries);
            e.extend(perr[k]);
            filtri::calibrate(&e)
        })
        .collect();
    let hyps: Vec<&Hyp> = inp.hyps.iter().collect();
    let rows = work
        .into_iter()
        .map(|w| {
            interpret(
                inp,
                &hyps,
                &dda,
                &nodes,
                &entries,
                &cals,
                w,
                &mut registry,
                &mut facts,
            )
        })
        .collect();
    Ok(Output {
        rows,
        registry,
        cals,
        nodes,
        entries,
        n_files: t.n_files,
    })
}

#[allow(clippy::too_many_arguments)]
fn interpret(
    inp: &Inputs,
    hyps: &[&Hyp],
    dda: &Consensus,
    nodes: &[LibNode],
    entries: &[LibNode],
    cals: &[FileCal],
    w: Work,
    registry: &mut filtri::Registry,
    facts: &mut [RowFacts],
) -> RowInterp {
    let t = inp.table;
    let mz = t.mz[w.row];
    let change = modifica::candidates(mz, &inp.parent.neutral, hyps);
    let iso = change
        .first()
        .and_then(|c| w.iso.and_then(|i| modifica::iso_check(&c.ion_formula, i.0)));
    let mz_obs = w.iso.filter(|i| i.1.is_finite()).map(|i| (i.1, w.best));
    let hidden = match (change.first(), mz_obs) {
        (Some(c), Some((obs, k))) => {
            let theo = hyps
                .iter()
                .find(|h| {
                    mzlab_core::elements::parse_formula(&h.formula)
                        .is_ok_and(|f| mzlab_core::elements::fmt(&f) == c.neutral_formula)
                })
                .map_or(mz, |h| h.mz);
            let bad = filtri::out_of_tolerance(ppm(obs, theo), &cals[k]);
            facts[w.row].mass_error = bad;
            bad.then_some("F5")
        }
        _ => None,
    };
    if hidden.is_some() {
        registry.hidden[4].push(t.id(w.row));
    }
    let pooled = ms2link::pool(&w.scans);
    let files_with_ms2 = w
        .scans
        .iter()
        .map(|s| s.file)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let delta = delta_mass(&inp.parent.neutral, change.first(), mz, inp.parent.mz);
    let (vs_parent, tree) = compare(inp, dda, nodes, entries, &w.scans, &pooled, mz, delta);
    RowInterp {
        row: w.row,
        id: t.id(w.row),
        change,
        iso,
        mz_obs,
        scans: w.scans,
        files_with_ms2,
        pooled,
        vs_parent,
        tree,
        hidden,
    }
}

#[allow(clippy::too_many_arguments)]
fn compare(
    inp: &Inputs,
    dda: &Consensus,
    nodes: &[LibNode],
    entries: &[LibNode],
    scans: &[Ms2Scan],
    pooled: &Pooled,
    mz: f64,
    delta: f64,
) -> (Option<VsParent>, Vec<NodeState>) {
    let c = &pooled.consensus;
    if c.mz.is_empty() {
        return (None, vec![]);
    }
    let mut v = VsParent::default();
    let dda_cmp = (!dda.mz.is_empty())
        .then(|| confronto::against_dda((&dda.mz, &dda.rel, inp.parent.mz), (&c.mz, &c.rel, mz)))
        .flatten();
    if let Some((comp, e, s, l)) = dda_cmp {
        (v.cos_mod, v.entropy, v.equal, v.shifted, v.losses) =
            (comp.cos_total(), comp.ent_total(), e, s, l);
        v.reference = if entries.is_empty() { "dda" } else { "both" };
    } else if !entries.is_empty() {
        v.reference = "infusion";
    }
    if !entries.is_empty() {
        let (e, s) = confronto::by_formula(&c.mz, entries, delta);
        (v.formula_equal, v.formula_shifted) = (e, s);
    }
    let range = scans
        .iter()
        .filter(|s| !s.chimeric)
        .find_map(|s| s.scan_range);
    let tree = albero::states(nodes, &c.mz, delta, range, mz);
    (
        if v.reference.is_empty() {
            None
        } else {
            Some(v)
        },
        tree,
    )
}

impl RowInterp {
    pub fn isomers(&self) -> &'static str {
        match self.pooled.isomers {
            Isomers::Single => "single",
            Isomers::Separated(_) => "separated",
            Isomers::NotSeparated => "not_separated",
        }
    }
}
