//! Ion families of one file, before any alignment: which peaks are the same molecule seen as an isotopologue, an adduct, a dimer or an
//! in-source fragment of one ion.
//!
//! Port of the relations of `TP_Mine/py/tpmine/hr/iimn.py` (`relations`, `dimer_of`), of `identity.py` (an edge needs co-elution, a known step
//! and shape correlation) and of `edge_evidence.py` (the proofs of the label). The families are made inside each file, as khipu and asari do:
//! the pairs are peaks that co-elute (apex within 0.05 min) and whose traces correlate (Pearson >= 0.9 scan by scan around the apex, within one
//! FWHM) with a step of the tables in `rel`; the isotopologues come from `isotopes`.
//! - An **in-source fragment** is declared only with the proof of the MS2 of its parent (its m/z among the fragments of the co-eluting ion) and,
//!   when the ratio over the samples is known (`Ratio`), a constant one. Otherwise the pair is «undetermined».
//! - The **alternative readings stay**: +17.0265 is `[M+NH4]+` or the loss of NH3, +46.0055 formate or HCOOH; `Families::alternatives` lists the
//!   neutral masses of the readings with their proofs, `FileFeatures::alt` marks the rows.
//! - A **family** (`family_id` = row of its `ROLE_M` + 1) is the unit that is aligned across files; the role of every member and the arc with its
//!   proofs stay here (`Arc`, `evidence::Proof`).
//!
//! Determinism: sorted pairs, stable orderings, sums in a fixed order, no `HashMap`. Cost O(n log n) in the number of peaks.

pub mod evidence;
mod pairs;
pub mod rel;
mod roles;
mod spectra;
mod traces;

use crate::sortc;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::ms2::Ms2Links;
use crate::table::*;
use crate::trace::Params;
use evidence::{edge_evidence, EdgeKind, Pair, Proof, Ratio, Rules, PROOF_VERSION};
use rel::{RKind, Reading, Tables};
pub use spectra::Spectra;
pub use traces::Traces;

/// Version of the rules of the families (the proofs have their own, `PROOF_VERSION`).
pub const FAMILY_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct FamParams {
    pub rules: Rules,
    /// Shape correlation needed to join two peaks.
    pub r_min: f64,
    /// Window of m/z of the traces, ppm.
    pub trace_ppm: f64,
    /// An in-source fragment needs a parent at least this many times higher.
    pub parent_factor: f32,
}

impl Default for FamParams {
    fn default() -> Self {
        FamParams {
            rules: Rules::default(),
            r_min: 0.9,
            trace_ppm: 6.0,
            parent_factor: 2.0,
        }
    }
}

/// A link between two peaks of a family with the readings of its step and the proofs of its label.
#[derive(Debug, Clone)]
pub struct Arc {
    pub lo: u32,
    pub up: u32,
    /// Shape correlation of the two traces.
    pub r: f32,
    pub kind: EdgeKind,
    pub readings: Vec<Reading>,
    pub proofs: Vec<Proof>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AltReading {
    /// The heavier ion is an adduct (or dimer) of the lighter: the base is the lighter.
    Adduct,
    /// The lighter ion is an in-source fragment of the heavier: the base is the heavier.
    Fragment,
}

/// A neutral mass the family can have, with the points of the proofs that support the reading (the lists are kept until a proof separates them).
#[derive(Debug, Clone, PartialEq)]
pub struct NeutralAlt {
    pub family: u32,
    pub row: u32,
    pub mass: f64,
    pub reading: AltReading,
    pub points: f32,
}

#[derive(Debug, Clone)]
pub struct Families {
    pub version: u32,
    pub proof_version: u32,
    pub params: FamParams,
    pub family_id: Vec<u32>,
    pub role: Vec<u8>,
    pub alt: Vec<u8>,
    /// Row each row derives from (`u32::MAX` = none).
    pub parent: Vec<u32>,
    pub arcs: Vec<Arc>,
    pub alternatives: Vec<NeutralAlt>,
}

impl Families {
    /// Write `family_id`, `role` and `alt` into the table.
    pub fn apply(&self, f: &mut FileFeatures) {
        f.family_id = self.family_id.clone();
        f.role = self.role.clone();
        f.alt = self.alt.clone();
    }

    /// Number of families (rows with `ROLE_M`).
    pub fn count(&self) -> usize {
        self.role.iter().filter(|&&r| r == ROLE_M).count()
    }
}

/// Candidate pairs: a step of the tables, or the lighter peak among the fragments of the MS2 of the heavier one; both co-eluting.
pub fn candidates(f: &FileFeatures, spectra: &Spectra, pol: i8, p: &FamParams) -> Vec<(u32, u32)> {
    let t = Tables::new(pol);
    let r = &p.rules;
    let mut out = pairs::candidate_pairs(f, &t, r.ppm, r.abs_da, r.coelution_min);
    let with_ms2: Vec<u32> = (0..f.len() as u32)
        .filter(|&i| spectra.of(i as usize).is_some())
        .collect();
    out.extend(pairs::ms2_pairs(f, &with_ms2, |i| spectra.of(i), r));
    sortc::by_key(&mut out, |&(i, j)| (j, i));
    out.dedup();
    out
}

/// The candidates whose traces correlate (Pearson >= `r_min` around the apexes), with the correlation.
pub fn correlate<R: ReadAt>(
    index: &mut Index<R>,
    f: &FileFeatures,
    cand: Vec<(u32, u32)>,
    base: &Params,
    p: &FamParams,
) -> Result<Vec<(u32, u32, f32)>> {
    let mut rows: Vec<u32> = cand.iter().flat_map(|&(i, j)| [i, j]).collect();
    sortc::ord(&mut rows);
    rows.dedup();
    let tr = traces::gather(index, f, &rows, base, p.trace_ppm)?;
    Ok(correlate_traces(f, cand, &tr, p))
}

/// `correlate` with the traces already in hand.
pub fn correlate_traces(
    f: &FileFeatures,
    cand: Vec<(u32, u32)>,
    tr: &Traces,
    p: &FamParams,
) -> Vec<(u32, u32, f32)> {
    cand.into_iter()
        .filter_map(|(i, j)| {
            let r = tr.correlation(f, i as usize, j as usize)?;
            (r >= p.r_min).then_some((i, j, r as f32))
        })
        .collect()
}

/// Arcs, roles and alternatives from the correlated pairs, the MS2 spectra and (when known) the ratios over the samples.
pub fn build_families(
    f: &FileFeatures,
    pairs: &[(u32, u32, f32)],
    spectra: &Spectra,
    ratio: impl Fn(usize, usize) -> Ratio,
    pol: i8,
    p: &FamParams,
) -> Result<Families> {
    let t = Tables::new(pol);
    let mut arcs = Vec::new();
    for &(i, j, r) in pairs {
        let (lo, up) = (i as usize, j as usize);
        let e = edge_evidence(
            &t,
            &p.rules,
            &Pair {
                mz_lo: f.mz[lo],
                mz_up: f.mz[up],
                ms2_lo: spectra.of(lo),
                ms2_up: spectra.of(up),
                ratio: ratio(lo, up),
                drt_min: Some((f.rt[up] - f.rt[lo]).abs() as f64),
            },
        );
        if e.kind != EdgeKind::Other {
            arcs.push(Arc {
                lo: i,
                up: j,
                r,
                kind: e.kind,
                readings: e.readings,
                proofs: e.proofs,
            });
        }
    }
    let a = roles::assign(f, &arcs, p.parent_factor)?;
    let alternatives = alternatives(f, &arcs, &a.family, &t);
    Ok(Families {
        version: FAMILY_VERSION,
        proof_version: PROOF_VERSION,
        params: p.clone(),
        family_id: a.family,
        role: a.role,
        alt: a.alt,
        parent: a.parent,
        arcs,
        alternatives,
    })
}

/// Neutral mass under each reading of the undetermined steps that two readings explain, ordered by family, row and reading.
fn alternatives(f: &FileFeatures, arcs: &[Arc], family: &[u32], t: &Tables) -> Vec<NeutralAlt> {
    let neutral = |row: u32| f.mz[row as usize] - t.pol as f64 * t.proton;
    let mut out = Vec::new();
    for a in arcs.iter().filter(|a| a.kind == EdgeKind::Undetermined) {
        let adduct = a
            .readings
            .iter()
            .any(|r| matches!(r.kind, RKind::Adduct | RKind::Dimer));
        if !(adduct && a.readings.iter().any(|r| r.kind == RKind::Loss)) {
            continue;
        }
        for (row, reading) in [(a.lo, AltReading::Adduct), (a.up, AltReading::Fragment)] {
            out.push(NeutralAlt {
                family: family[a.lo as usize],
                row,
                mass: neutral(row),
                reading,
                points: roles::reading_points(a, reading == AltReading::Fragment),
            });
        }
    }
    sortc::any_by(&mut out, |a, b| {
        a.family
            .cmp(&b.family)
            .then(a.row.cmp(&b.row))
            .then((a.reading as u8).cmp(&(b.reading as u8)))
    });
    out.dedup();
    out
}

/// The whole step: pairs, traces, MS2 spectra of the rows that need them, proofs and roles. Fills `family_id`, `role` and `alt` of `f`.
pub fn analyse_families<R: ReadAt>(
    index: &mut Index<R>,
    f: &mut FileFeatures,
    links: &Ms2Links,
    base: &Params,
    p: &FamParams,
) -> Result<Families> {
    let pol = base.polarity.unwrap_or(1);
    let with_ms2: Vec<u32> = (0..f.len() as u32)
        .filter(|&i| pairs::eligible(f, i as usize) && f.ms2_n[i as usize] > 0)
        .collect();
    let spectra = spectra::gather(index, f, links, &with_ms2, base, &p.rules)?;
    let cand = candidates(f, &spectra, pol, p);
    let pairs = correlate(index, f, cand, base, p)?;
    let fam = build_families(f, &pairs, &spectra, |_, _| Ratio::Unavailable, pol, p)?;
    fam.apply(f);
    Ok(fam)
}
