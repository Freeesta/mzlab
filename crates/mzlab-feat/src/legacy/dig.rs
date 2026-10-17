//! The Dig of mzFinder on the series of the pipeline: the candidates are the identities of `evid/1` with a proof in favour (`totale` above 0), ranked
//! by their points; formula and derivation come from the hypothesis of the identity, the kinetics from the areas of the row, the MS2 from the scans
//! of every file inside the peak (consensus and three-component similarity with the MS2 of the parent).

use std::collections::{BTreeSet, HashMap};

use mzlab_core::elements::{fmt, parse_formula, Formula};
use mzlab_core::idxread::{Index, ReadAt};

use super::kinetics::{descriptors, not_in_reference, Descriptors};
use super::ms2::{consensus, read_scans, select, Consensus, Ms2Index, Pick};
use super::series::{FileIn, Hyp, Ident, Parent};
use super::similarity::{pair_components, prepare, Components, Spec};
use super::Res;
use crate::evid::points::PointsTable;
use crate::evid::Kind;
use crate::rtmap::{build, RtMap, RtParams};
use crate::table::{Aligned, CELL_ABSENT, CELL_FILLED};
use crate::tp::{NOTE_CHIMERIC_MS2, NOTE_ISF_COELUTES, NO_PARENT, ST_ISF};

/// Isolation centre within this many ppm of the m/z of the row (`select_ms2`).
const MS2_PPM: f64 = 10.0;
/// The parent row searched within this many ppm of the [M+H]+ when the table has no TP view.
const PARENT_PPM: f64 = 5.0;
/// Shifted fragments (or common neutral losses) that confirm a rule, and their share of the modified cosine.
const MIN_SHIFTED: usize = 2;
const MIN_SHIFTED_COS: f64 = 0.05;
pub const MAX_ROWS: usize = 500;
pub const INCLUSION: usize = 50;

/// The proof types of `evid/1` with the names the page shows (keys of `weights` and `components`) and the name of the criterion.
pub const KINDS: [(Kind, &str, &str); 10] = [
    (Kind::Mass, "mass", "Exact mass of the hypothesis"),
    (Kind::Isotopes, "isotopes", "Isotope pattern"),
    (Kind::Ms2, "ms2", "MS2 linked to the parent"),
    (Kind::Fragments, "fragments", "Fragments of the parent"),
    (Kind::Network, "network", "Network of transformations"),
    (Kind::Kinetics, "kinetics", "Kinetics of a product"),
    (Kind::Controls, "controls", "Absent in the controls"),
    (Kind::Rt, "rt", "Retention time"),
    (Kind::Frequency, "frequency", "Frequency in the series"),
    (Kind::External, "external", "External sources"),
];

/// The MS2 of a candidate: consensus, its prepared peaks, similarity with the parent, the times of the scans.
#[derive(Debug, Clone)]
pub struct CandMs2 {
    pub cons: Consensus,
    pub sim: Components,
}

/// One candidate of the Dig.
#[derive(Debug, Clone)]
pub struct Cand {
    pub row: usize,
    pub mz: f64,
    pub rt: f64,
    pub areas: Vec<f64>,
    pub area_max: f64,
    pub formula: Option<String>,
    pub ppm: Option<f64>,
    pub delta: Option<String>,
    pub hyp: Option<Hyp>,
    pub kin: Option<(Descriptors, bool)>,
    pub ms2_scans: usize,
    pub ms2: Option<CandMs2>,
    pub score: f64,
    /// Points of every type of `KINDS` divided by the most it can give.
    pub comps: [f64; 10],
    pub ident: Ident,
    pub flags: Vec<&'static str>,
    pub filled: Vec<usize>,
    pub limits: Vec<(f64, f64)>,
    /// Negative id of a product hidden under an in-source fragment (`hidden.rs`); `row` is then the row of the fragment.
    pub hidden: Option<i64>,
}

impl Cand {
    /// The id that the page uses: the row, or the negative id of a hidden product.
    pub fn id(&self) -> i64 {
        self.hidden.unwrap_or(self.row as i64)
    }
    /// 5 appears with the treatment, 4 formula of a hypothesis with its exact mass, 3 also the MS2 linked to the parent.
    pub fn level(&self) -> u8 {
        let pass = |k: &str| {
            self.ident
                .proofs
                .iter()
                .any(|p| p.kind == k && p.points > 0.0 && p.sign == "a favore")
        };
        match (self.formula.is_some() && pass("massa"), pass("MS2")) {
            (true, true) => 3,
            (true, false) => 4,
            _ => 5,
        }
    }

    /// Does the MS2 confirm the transformation? At least `MIN_SHIFTED` peaks moved by the mass difference (shifted fragments or common neutral
    /// losses) carrying at least `MIN_SHIFTED_COS` of the cosine, and the ion not present in the reference samples. A cleavage has no single
    /// mass difference.
    pub fn confirmation(&self) -> (bool, String) {
        if self.hyp.as_ref().is_some_and(|h| h.label.starts_with("rottura")) {
            return (false, "a cleavage has no single mass difference".into());
        }
        let Some(m) = &self.ms2 else {
            return (false, "no MS2 for this ion".into());
        };
        let (n, share) = (m.sim.n[1] + m.sim.n[2], m.sim.cos[1] + m.sim.cos[2]);
        if n < MIN_SHIFTED || share < MIN_SHIFTED_COS {
            return (false, format!("{n} shifted fragments, {share:.2} of the cosine: not enough"));
        }
        if self.kin.as_ref().is_some_and(|k| !k.1) {
            return (false, "present in the reference samples".into());
        }
        (true, format!("{n} fragments shifted by {:+.4} (share of the cosine {share:.2})", m.sim.delta))
    }

    /// Prediction level (1 = strongest): the weakest step of the derivation, raised to 1 when the data confirm it; `None` without a rule.
    pub fn pred(&self) -> Option<u8> {
        let h = self.hyp.as_ref().filter(|h| (1..=4).contains(&h.level))?;
        Some(if self.confirmation().0 { 1 } else { h.level })
    }
}

/// The whole Dig, kept for `detail` and the inclusion list.
#[derive(Debug, Clone)]
pub struct Dig {
    pub files: Vec<FileIn>,
    pub parent: Parent,
    pub parent_row: Option<usize>,
    pub parent_rt: Option<f64>,
    pub parent_ms2: Option<Consensus>,
    /// The shift of the apex of the anchor features in every file against the median over the files (drawn by the page; no signal is moved).
    pub rt_map: RtMap,
    /// The (m/z, RT) of the rows left out because the gate of the TP view takes them for in-source fragments of the parent or of a product.
    pub isf_hidden: Vec<(f64, f64)>,
    pub n_features: Vec<usize>,
    pub cands: Vec<Cand>,
    pub weights: [f64; 10],
    pub funnel: Vec<(usize, &'static str)>,
    pub warnings: Vec<String>,
}

/// The most points of each type (its best step, under its ceiling).
pub fn weights(p: &PointsTable) -> [f64; 10] {
    let mut w = [0.0; 10];
    for (i, (k, _, _)) in KINDS.iter().enumerate() {
        if let Some(r) = p.rule(*k) {
            let best = r.steps.iter().map(|s| s.points).fold(0.0, f64::max);
            w[i] = r.cap.map_or(best, |c| best.min(c));
        }
    }
    w
}

/// `+X` or `-X`: the atoms gained or lost from the parent.
fn delta(f: &Formula, parent: &Formula) -> String {
    let els: BTreeSet<&str> = f.iter().chain(parent).map(|e| e.0.as_str()).collect();
    let n = |g: &Formula, e: &str| g.iter().find(|x| x.0 == e).map_or(0, |x| x.1 as i64);
    let d: Vec<(&str, i64)> = els
        .into_iter()
        .map(|e| (e, n(f, e) - n(parent, e)))
        .collect();
    let sign = if d.iter().map(|x| x.1).sum::<i64>() < 0 {
        "-"
    } else {
        "+"
    };
    let abs: Formula = d
        .iter()
        .filter(|x| x.1 != 0)
        .map(|x| (x.0.to_string(), x.1.unsigned_abs()))
        .collect();
    format!("{sign}{}", fmt(&abs))
}

/// The parent row: the row nearest in RT to the one the user gave, else the one of the TP view when the table has one (an empty `tp/1` block says
/// nothing: its `parent_row` is the default 0), else the one with the largest area within `PARENT_PPM` of the ion in the files at time 0 (the
/// standard), or in all of them when none is at time 0.
pub(super) fn parent_row(t: &Aligned, p: &Parent, files: &[FileIn]) -> Option<usize> {
    let near = |r: usize| ((t.mz[r] - p.mz) / p.mz * 1e6).abs() <= PARENT_PPM;
    if let Some(rt) = p.rt {
        return (0..t.len())
            .filter(|&r| near(r))
            .min_by(|&a, &b| (t.rt[a] - rt).abs().total_cmp(&(t.rt[b] - rt).abs()));
    }
    if !t.tp.is_empty() && t.tp.parent_row != NO_PARENT && (t.tp.parent_row as usize) < t.len() {
        return Some(t.tp.parent_row as usize);
    }
    let t0: Vec<bool> = files.iter().map(|f| f.t == 0.0).collect();
    let only_t0 = t0.iter().any(|&b| b);
    let area = |r: usize| {
        t.area[r * t.n_files..(r + 1) * t.n_files]
            .iter()
            .enumerate()
            .filter(|(k, a)| a.is_finite() && (!only_t0 || t0.get(*k).copied().unwrap_or(false)))
            .map(|(_, a)| a)
            .sum::<f64>()
    };
    (0..t.len())
        .filter(|&r| near(r))
        .max_by(|&a, &b| area(a).total_cmp(&area(b)))
}

/// The rows that the gate of the TP view took for in-source fragments of the parent or of a product, by m/z.
fn isf_rows(t: &Aligned) -> Vec<usize> {
    let mut rows: Vec<usize> = (0..t.len())
        .filter(|&r| t.tp.status.get(r) == Some(&ST_ISF))
        .collect();
    rows.sort_by(|&a, &b| t.mz[a].total_cmp(&t.mz[b]));
    rows
}

/// Peak limits of a row in every file: apex ± width / 2 where the file has a peak, the edges of the row elsewhere (NaN without them).
fn limits(t: &Aligned, r: usize) -> Vec<(f64, f64)> {
    (0..t.n_files)
        .map(|k| {
            let c = r * t.n_files + k;
            let (a, w) = (f64::from(t.apex_rt[c]), f64::from(t.width[c]));
            if t.state[c] == CELL_ABSENT {
                (f64::NAN, f64::NAN)
            } else if a.is_finite() && w.is_finite() {
                (a - w / 2.0, a + w / 2.0)
            } else if t.meta.len() == t.len() {
                (f64::from(t.meta.rt_lo[r]), f64::from(t.meta.rt_hi[r]))
            } else {
                (f64::NAN, f64::NAN)
            }
        })
        .collect()
}

pub(super) fn areas(t: &Aligned, r: usize) -> Vec<f64> {
    (0..t.n_files)
        .map(|k| {
            let c = r * t.n_files + k;
            if t.state[c] == CELL_ABSENT || !t.area[c].is_finite() {
                0.0
            } else {
                t.area[c]
            }
        })
        .collect()
}

/// Areas of the row by file, the largest in a treated sample and the kinetic descriptors (with `absent_in_reference`).
fn kinetics_of(
    t: &Aligned,
    files: &[FileIn],
    r: usize,
) -> (Vec<f64>, f64, Option<(Descriptors, bool)>) {
    let y = areas(t, r);
    let times: Vec<f64> = files.iter().map(|f| f.t).collect();
    let pick = |sel: fn(&FileIn) -> bool| -> Vec<f64> {
        files
            .iter()
            .zip(&y)
            .filter(|(f, _)| sel(f))
            .map(|x| *x.1)
            .collect()
    };
    let (treated, refs) = (pick(FileIn::is_treated), pick(FileIn::is_ref));
    let (area_max, ref_max) = (
        treated.iter().copied().fold(0.0, f64::max),
        refs.iter().copied().fold(0.0, f64::max),
    );
    let kin = descriptors(&times, &y).map(|d| {
        (
            d,
            !treated.is_empty() && not_in_reference(&treated, ref_max),
        )
    });
    (y, area_max, kin)
}

/// Points of every type of `KINDS` divided by the most it can give.
fn comps_of(id: &Ident, w: &[f64; 10]) -> [f64; 10] {
    let mut comps = [0.0; 10];
    for (i, (k, _, _)) in KINDS.iter().enumerate() {
        let pts: f64 = id
            .proofs
            .iter()
            .filter(|p| p.kind == k.name())
            .map(|p| p.points)
            .fold(0.0, |a, b| a + b);
        comps[i] = if w[i] > 0.0 {
            (pts / w[i]).min(1.0)
        } else {
            0.0
        };
    }
    comps
}

fn flags_of(t: &Aligned, r: usize) -> Vec<&'static str> {
    let notes = t.tp.notes.get(r).copied().unwrap_or(0);
    let mut flags = Vec::new();
    if notes & NOTE_ISF_COELUTES != 0 {
        flags.push("possible TP co-eluting with an ISF");
    }
    if notes & NOTE_CHIMERIC_MS2 != 0 {
        flags.push("MS2 possibly chimeric (co-isolated ions)");
    }
    flags
}

pub(super) fn candidate(
    t: &Aligned,
    files: &[FileIn],
    parent: &Parent,
    id: Ident,
    hyps: &HashMap<u32, Hyp>,
    w: &[f64; 10],
) -> Cand {
    let r = id.row;
    let (areas, area_max, kin) = kinetics_of(t, files, r);
    let hyp = id.hypothesis.and_then(|h| hyps.get(&h)).cloned();
    let formula = id
        .formula
        .clone()
        .or_else(|| hyp.as_ref().map(|h| h.formula.clone()))
        .filter(|f| !f.is_empty());
    // a row stands for its family: the ion of the hypothesis may be one of its members (the [M+H]+ of a product whose row is led by its [M+Na]+)
    let mz = hyp
        .as_ref()
        .filter(|h| h.mz.is_finite())
        .map_or(t.mz[r], |h| {
            std::iter::once(t.mz[r])
                .chain(t.members(r).map(|k| t.member_mz[k]))
                .min_by(|a, b| (a - h.mz).abs().total_cmp(&(b - h.mz).abs()))
                .unwrap_or(t.mz[r])
        });
    let ppm = hyp
        .as_ref()
        .filter(|h| h.mz.is_finite())
        .map(|h| (mz - h.mz) / h.mz * 1e6);
    let delta = formula
        .as_deref()
        .and_then(|f| parse_formula(f).ok())
        .map(|f| delta(&f, &parent.neutral));
    let filled = (0..t.n_files)
        .filter(|&k| t.state[r * t.n_files + k] == CELL_FILLED)
        .collect();
    Cand {
        row: r,
        mz,
        rt: t.rt[r],
        areas,
        area_max,
        formula,
        ppm,
        delta,
        hyp,
        kin,
        ms2_scans: 0,
        ms2: None,
        score: id.total,
        comps: comps_of(&id, w),
        flags: flags_of(t, r),
        ident: id,
        filled,
        limits: limits(t, r),
        hidden: None,
    }
}

mod attach;
use attach::attach_ms2;

/// The inputs of a Dig on a series already through the pipeline.
pub struct Series<'a> {
    pub files: Vec<FileIn>,
    pub parent: Parent,
    pub table: &'a Aligned,
    pub idents: Vec<Ident>,
    pub hyps: HashMap<u32, Hyp>,
    pub found: u32,
}

pub fn dig<R: ReadAt>(s: Series, mut open: impl FnMut(usize) -> Res<Index<R>>) -> Res<Dig> {
    let t = s.table;
    if t.n_files != s.files.len() {
        return Err(format!(
            "the series has {} files, the Dig {}",
            t.n_files,
            s.files.len()
        ));
    }
    let w = weights(&PointsTable::bundled().map_err(|e| e.to_string())?);
    let n_ident = s.idents.len();
    let isf_rows = isf_rows(t);
    let is_isf = |r: usize| isf_rows.contains(&r);
    let isf_hidden: Vec<(f64, f64)> = isf_rows.iter().map(|&r| (t.mz[r], t.rt[r])).collect();
    let mut ids: Vec<Ident> = s
        .idents
        .into_iter()
        .filter(|i| i.total > 0.0 && i.row < t.len() && !is_isf(i.row))
        .collect();
    ids.sort_by(|a, b| b.total.total_cmp(&a.total).then(a.row.cmp(&b.row)));
    let mut cands: Vec<Cand> = ids
        .into_iter()
        .map(|i| candidate(t, &s.files, &s.parent, i, &s.hyps, &w))
        .collect();
    let prow = parent_row(t, &s.parent, &s.files);
    let mut ix = (0..t.n_files).map(&mut open).collect::<Res<Vec<_>>>()?;
    let parent_ms2 = attach_ms2(&mut ix, t, &s.parent, prow, &mut cands)?;
    if let (Some(p), Some(m)) = (prow, &parent_ms2) {
        super::hidden::add(t, &s.files, &s.parent, &s.hyps, p, &m.mz, &w, &mut cands);
    }
    let mut warnings = Vec::new();
    if !isf_hidden.is_empty() {
        warnings.push(format!(
            "{} ions left out as in-source fragments: they co-elute with the parent or with a product, in a constant ratio, and sit among the fragments of the parent.",
            isf_hidden.len()
        ));
    }
    if parent_ms2.is_none() {
        warnings.push("No MS2 of the parent in the series: the similarity of the candidates to the parent is not computed.".to_string());
    }
    let detected = |k: usize| {
        (0..t.len())
            .filter(|&r| t.state[r * t.n_files + k] == crate::table::CELL_DETECTED)
            .count()
    };
    let funnel = vec![
        (
            t.len(),
            "features aligned over the series (feature table of the pipeline)",
        ),
        (
            s.found as usize,
            "hypotheses of the parent found in at least one file",
        ),
        (n_ident, "identities of the proof graph"),
        (
            cands.len(),
            "identities with a proof in favour: the candidates",
        ),
    ];
    Ok(Dig {
        files: s.files,
        parent: s.parent,
        parent_row: prow,
        parent_rt: prow.map(|r| t.rt[r]),
        parent_ms2,
        rt_map: build(t, &RtParams::default()),
        isf_hidden,
        n_features: (0..t.n_files).map(detected).collect(),
        cands,
        weights: w,
        funnel,
        warnings,
    })
}

#[cfg(test)]
#[path = "dig_tests.rs"]
mod tests;
