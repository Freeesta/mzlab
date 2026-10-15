//! The Dig of mzFinder on a low-resolution series (unit resolution, full scan), computed on the pipeline of the crate (profile `lr`: L1-L6 and
//! the hypotheses of L7) instead of the Python engine (`TP_Mine/py/tpmine/engine.py`), with the JSON of `api.run` and `api.detail` that
//! `TP_Mine/js/20_tpmine.js` reads. The rows of the table are the peaks: no second peak search. MS2, MRM and the in-source fragments come later.

pub mod detail;
pub mod isf;
pub mod kin;
pub mod labels;
pub mod mrm;
pub mod ms2;
pub mod ms2link;
pub mod ms2target;
pub mod peak1d;
pub mod plan;
pub mod rows;
pub mod spec;
pub mod summary;
pub mod tree;

use std::collections::BTreeSet;

use mzlab_core::elements::{fmt, parse_formula, Formula};

use super::series::FileIn;
use super::Res;
use crate::aligned::Aligned;
use kin::{decay, gen_of, kinetics, Decay, Gens};
use labels::{score, status, Cell, Crit, GROWS};
use plan::{FileLr, ParentLr, Settings};

/// The calibration offset measured on the parent (observed - calculated m/z of its ion).
#[derive(Debug, Clone)]
pub struct Offset {
    pub calc: f64,
    pub observed: f64,
    pub offset: f64,
    pub sample: String,
    pub applied: bool,
}

/// One row of the Dig: the parent (id 0), a hypothesis (or one of its isomer peaks) or an unexpected ion.
#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub id: i64,
    pub unexpected: bool,
    pub name: String,
    pub alternatives: Vec<String>,
    pub formula: String,
    pub delta: String,
    /// m/z of the ion: calculated for the parent and the hypotheses, observed minus the offset for an unexpected ion.
    pub mz: f64,
    /// m/z of the extraction (with the offset).
    pub mz_x: f64,
    pub steps: u32,
    pub neutral_mass: Option<f64>,
    pub row: Option<usize>,
    /// m/z of the row in the table (the observed centroid).
    pub observed: Option<f64>,
    pub area: f64,
    pub cells: Vec<Cell>,
    pub ref_rt: Option<f64>,
    pub score: Option<f64>,
    pub label: &'static str,
    pub criteria: Vec<Crit>,
    pub isotope_of: Option<i64>,
    pub generation: Option<&'static str>,
    /// A row made from an MS2 experiment whose ion the Dig does not list (`ms2target`).
    pub ms2_target: bool,
}

/// The whole Dig, kept by the worker for `detail`.
#[derive(Debug, Clone)]
pub struct LrDig {
    pub files: Vec<FileLr>,
    pub lc: Vec<FileIn>,
    pub parent: ParentLr,
    pub settings: Settings,
    pub offset: Option<Offset>,
    pub decay: Option<Decay>,
    pub entries: Vec<Entry>,
    /// Evidence of in-source fragments, `(entry id, verdict)` (`isf::evaluate`; empty until the page asks for it).
    pub isf: Vec<(i64, crate::legacy::lrisf::IsfVerdict)>,
    /// The MS2 matches (`ms2link::ms2_link`; `None` until computed: the pools are empty).
    pub links: Option<ms2link::Links>,
    /// The aligned table of the columns, for the rows that `ms2target` makes from the MS2 experiments.
    pub table: std::sync::Arc<Aligned>,
}

/// `+X-Y`: atoms gained and lost from the parent (empty without both formulas).
pub fn delta(formula: &str, parent: &str) -> String {
    let (Ok(f), Ok(p)) = (parse_formula(formula), parse_formula(parent)) else {
        return String::new();
    };
    let els: BTreeSet<&str> = f.iter().chain(&p).map(|e| e.0.as_str()).collect();
    let n = |g: &Formula, e: &str| g.iter().find(|x| x.0 == e).map_or(0, |x| x.1 as i64);
    let d: Vec<(&str, i64)> = els
        .into_iter()
        .map(|e| (e, n(&f, e) - n(&p, e)))
        .filter(|x| x.1 != 0)
        .collect();
    let part = |pos: bool| -> String {
        let g: Formula = d
            .iter()
            .filter(|x| (x.1 > 0) == pos)
            .map(|x| (x.0.to_string(), x.1.unsigned_abs()))
            .collect();
        if g.is_empty() {
            String::new()
        } else {
            format!("{}{}", if pos { "+" } else { "-" }, fmt(&g))
        }
    };
    format!("{}{}", part(true), part(false))
}

pub(super) fn evaluate(e: &mut Entry, t: &Aligned, lc: &[FileIn], p: labels::Persist) {
    e.cells = match e.row {
        Some(r) => rows::cells(t, r),
        None => vec![Cell::default(); lc.len()],
    };
    e.ref_rt = e.row.map(|r| mzlab_core::json::round(t.rt[r], 4));
    e.observed = e.row.map(|r| t.mz[r]);
    let sc = score(lc, &e.cells, p);
    (e.score, e.label, e.criteria) = (sc.score, sc.label, sc.criteria);
}

/// Isotope check and kinetic generation of one entry (`snapshot` = the entries the isotope is looked for in).
pub(super) fn finish_one(
    e: &mut Entry,
    snapshot: &[Entry],
    lc: &[FileIn],
    s: &Settings,
    gens: &Gens,
) {
    let hit = rows::isotope_of(e, snapshot, s.tol_da, s.rt_tol_min);
    rows::isotope_crit(e, hit);
    if status(&e.criteria, GROWS) == "pass" {
        if let Some((v, text)) = gen_of(gens, &kinetics(lc, &e.cells)) {
            e.criteria
                .push(labels::crit("kinetic generation", "n/a", text));
            e.generation = Some(v);
        }
    }
}

/// Isotope check and kinetic generation of every entry but the parent.
fn finish(entries: &mut [Entry], lc: &[FileIn], s: &Settings, gens: &Gens) {
    let snapshot: Vec<Entry> = entries.to_vec();
    for e in entries
        .iter_mut()
        .filter(|e| e.id != 0 && e.score.is_some())
    {
        finish_one(e, &snapshot, lc, s, gens);
    }
}

/// The unexpected ions that still pass after the isotope check, by score then area, at most `max_unexpected`.
fn keep_unexpected(entries: Vec<Entry>, max: usize) -> Vec<Entry> {
    let (known, unexp): (Vec<Entry>, Vec<Entry>) = entries.into_iter().partition(|e| !e.unexpected);
    let good: Vec<&Entry> = unexp
        .iter()
        .filter(|e| {
            matches!(e.label, "forte" | "possibile") && status(&e.criteria, GROWS) != "fail"
        })
        .collect();
    let mut ord: Vec<usize> = (0..good.len()).collect();
    let sc = |e: &Entry| e.score.unwrap_or(0.0);
    mzlab_core::sortc::by(&mut ord, |&a, &b| {
        sc(good[b])
            .total_cmp(&sc(good[a]))
            .then(good[b].area.total_cmp(&good[a].area))
    });
    let mut out = known;
    out.extend(ord.into_iter().take(max).map(|i| good[i].clone()));
    out
}

/// `api.run` on a low-resolution series after the pipeline: `table` = the `.mzfeat` of the full-scan columns (in the order of `plan::columns`),
/// `registry` = the `ipotesi/1` JSON of the hypotheses of the parent.
pub fn dig(
    files_json: &str,
    parent_json: &str,
    settings_json: &str,
    table: &[u8],
    registry: &str,
) -> Res<LrDig> {
    let files = plan::files(files_json)?;
    let lc = plan::columns(&files);
    let parent = plan::parent(parent_json)?;
    let s = plan::settings(settings_json);
    let (t, _) = crate::store_read::read_table(table).map_err(|e| e.to_string())?;
    if t.n_files != lc.len() {
        return Err(format!(
            "the series has {} files, the Dig {}",
            t.n_files,
            lc.len()
        ));
    }
    if lc.is_empty() {
        return Err("full-scan (MS1) files of the sample are required: none found".into());
    }
    let hyps = rows::hypotheses(registry)?;
    let (prow, offset) = rows::calibrate(&t, &lc, &parent, &s);
    let off = offset
        .as_ref()
        .filter(|o| o.applied)
        .map_or(0.0, |o| o.offset);
    let mut p = Entry {
        name: "parent".into(),
        formula: parent.formula.clone(),
        mz: parent.mz,
        mz_x: parent.mz + off,
        neutral_mass: Some(parent.neutral_mass),
        alternatives: rows::parent_alternatives(&hyps, &parent, s.tol_da),
        row: prow,
        ..Entry::default()
    };
    evaluate(&mut p, &t, &lc, s.persist());
    (p.score, p.label, p.criteria) = (None, "progenitore", Vec::new());
    let dec = decay(&kinetics(&lc, &p.cells));
    let gens = Gens::new(&kinetics(&lc, &p.cells), dec.map(|d| d.k));
    let (cands, _) = rows::candidates(&t, &lc, &hyps, &parent, off, s.tol_da);
    let mut entries = vec![p];
    entries.extend(cands);
    for e in entries.iter_mut().skip(1) {
        evaluate(e, &t, &lc, s.persist());
    }
    if s.discover {
        let known: Vec<f64> = entries.iter().map(|e| e.mz_x).collect();
        let mut un = rows::unexpected(&t, &lc, &known, &s, off);
        for e in un.iter_mut() {
            evaluate(e, &t, &lc, s.persist());
        }
        entries.extend(un);
    }
    finish(&mut entries, &lc, &s, &gens);
    let entries = keep_unexpected(entries, s.max_unexpected);
    Ok(LrDig {
        files,
        lc,
        parent,
        settings: s,
        offset,
        decay: dec,
        entries,
        isf: Vec::new(),
        links: None,
        table: std::sync::Arc::new(t),
    })
}
