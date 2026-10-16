//! Groups of experiments, `gruppi/1` (`docs/agenti/gruppi.md`, schema `docs/schemi/gruppi-1.json`): the same degradation in several conditions,
//! compared on ONE alignment. Input: the matrix rows x files, the metadata of the rows and the canonical `campioni/2`; output: per row and condition
//! the presence, mean area of the replicates, ratio to a reference condition, shape over time and the controls as "Prev." suggestions, the exact
//! sets of conditions and the decay of the parent per condition. The module reads no file and never realigns: it uses the rows it receives.
//! A condition is a non-empty `serie` of a `sample`; `serie == ""` marks a control shared by all conditions, and a control of the condition wins.

mod forme;
mod insiemi;
mod json;
mod presenza;
mod prev;

pub use json::{gruppi_json, parse_input, run_json, to_json};

use mzlab_core::campioni::Campione;
use mzlab_core::error::{Error, Result};
use presenza::{CondModel, GStat};

pub use presenza::Controls;

/// Cell states of `GroupTable::state` are the ones of `crate::table` (`CELL_DETECTED`, `CELL_FILLED`, `CELL_ABSENT`); a targeted cell counts as detected.
pub const FORMAT: &str = "gruppi";
pub const VERSION: u32 = 1;

/// What the caller knows of one row of the table (the module never looks inside the file).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RowMeta {
    pub id: u32,
    pub tp_name: String,
    pub mz: f64,
    pub polarity: String,
    pub rt: f64,
    pub neutral_formula: Option<String>,
    pub ppm: Option<f64>,
    /// Formula of the ion (e.g. of `[M+H]+`), filled by the high resolution caller; null at low resolution.
    pub ion_formula: Option<String>,
    pub is_parent: bool,
    /// Indices of the files whose MS2 is linked to the row.
    pub ms2_files: Vec<usize>,
    /// Id of the row of the other polarity with the same neutral mass, when the caller found one.
    pub other_polarity: Option<u32>,
}

/// The matrix rows x files of ONE alignment; cell `r * n_files + f`.
#[derive(Debug, Clone, Default)]
pub struct GroupTable {
    pub n_rows: usize,
    pub n_files: usize,
    pub area: Vec<f64>,
    pub height: Vec<f64>,
    pub state: Vec<u8>,
    pub apex_rt: Vec<f32>,
    pub rows: Vec<RowMeta>,
}

/// The options (`gruppi/1`); `norm` is `"raw"`, the only one implemented.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupOpts {
    pub reference: Option<String>,
    pub min_rep_frac: f64,
    pub min_area: f64,
    pub ctrl_ratio: f64,
    pub norm: String,
    pub shape_drop: f64,
    /// Condition -> free key/value strings: kept and reported, never used.
    pub factors: Vec<(String, Vec<(String, String)>)>,
}

impl Default for GroupOpts {
    fn default() -> Self {
        GroupOpts {
            reference: None,
            min_rep_frac: 0.5,
            min_area: 0.0,
            ctrl_ratio: 0.2,
            norm: "raw".into(),
            shape_drop: 0.3,
            factors: Vec::new(),
        }
    }
}

/// One point of the time course of a condition.
#[derive(Debug, Clone, PartialEq)]
pub struct KinPt {
    pub t: f64,
    pub area: f64,
    pub sd: Option<f64>,
    pub n: usize,
}

/// The suggestions of one row in one condition (every status `pass`/`fail`/`na`, `yes`/`no`/`na`, `linked`/`none`/`na`).
#[derive(Debug, Clone, PartialEq)]
pub struct Prev {
    pub absent_t0: &'static str,
    pub absent_blank: &'static str,
    pub in_parent_std: &'static str,
    pub above_min: &'static str,
    pub ms2: &'static str,
    pub verdict: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CondRow {
    pub id: String,
    pub present: bool,
    pub n_det: usize,
    pub n_rep: usize,
    pub area: f64,
    pub area_sd: Option<f64>,
    pub cv: Option<f64>,
    pub ratio_ref: Option<f64>,
    pub shape: Option<&'static str>,
    pub t_max: Option<f64>,
    pub t_half_rise: Option<f64>,
    pub rate_rel: Option<f64>,
    pub kin: Vec<KinPt>,
    pub prev: Option<Prev>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RowOut {
    pub meta: RowMeta,
    pub present_in: Vec<String>,
    pub only_in: Option<String>,
    pub in_all: bool,
    pub cond: Vec<CondRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParentOut {
    pub k: Option<f64>,
    pub half_life: Option<f64>,
    pub remaining: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CondOut {
    pub id: String,
    pub factors: Vec<(String, String)>,
    pub files: Vec<usize>,
    pub times: Vec<f64>,
    pub replicates: usize,
    pub controls: Controls,
    pub parent: ParentOut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetOut {
    pub conditions: Vec<String>,
    pub rows: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub code: &'static str,
    pub condition: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupsOut {
    pub reference: Option<String>,
    pub conditions: Vec<CondOut>,
    pub rows: Vec<RowOut>,
    pub sets: Vec<SetOut>,
    pub warnings: Vec<Warning>,
}

/// Everything a row needs to be computed.
pub(crate) struct Ctx<'a> {
    pub tb: &'a GroupTable,
    pub opts: &'a GroupOpts,
    pub conds: Vec<CondModel>,
    pub ref_ix: Option<usize>,
    pub has_ms2: bool,
}

fn size_err(what: &str) -> Error {
    Error::new("error.gruppi.sizeMismatch").with("what", what)
}

fn bad_opt(name: &'static str, value: impl ToString) -> Error {
    Error::new("error.gruppi.badOpts").with(name, value)
}

/// The sizes of the arrays and the ranges of the options.
fn check(tb: &GroupTable, cs: &[Campione], o: &GroupOpts) -> Result<()> {
    let cells = tb.n_rows * tb.n_files;
    if tb.area.len() != cells {
        return Err(size_err("area"));
    }
    if tb.height.len() != cells {
        return Err(size_err("height"));
    }
    if tb.state.len() != cells {
        return Err(size_err("state"));
    }
    if tb.apex_rt.len() != cells {
        return Err(size_err("apex_rt"));
    }
    if tb.rows.len() != tb.n_rows {
        return Err(size_err("rows"));
    }
    if cs.len() != tb.n_files {
        return Err(size_err("campioni"));
    }
    if o.norm != "raw" {
        return Err(bad_opt("norm", &o.norm));
    }
    let frac = |v: f64| v.is_finite() && v > 0.0 && v <= 1.0;
    if !frac(o.min_rep_frac) {
        return Err(bad_opt("min_rep_frac", o.min_rep_frac));
    }
    if !frac(o.shape_drop) {
        return Err(bad_opt("shape_drop", o.shape_drop));
    }
    if !(o.min_area.is_finite() && o.min_area >= 0.0) {
        return Err(bad_opt("min_area", o.min_area));
    }
    if !(o.ctrl_ratio.is_finite() && o.ctrl_ratio >= 0.0) {
        return Err(bad_opt("ctrl_ratio", o.ctrl_ratio));
    }
    Ok(())
}

/// The conditions named in `factors` must exist.
fn check_factors(o: &GroupOpts, conds: &[CondModel]) -> Result<()> {
    for (c, _) in &o.factors {
        if !conds.iter().any(|m| &m.id == c) {
            return Err(bad_opt("factors", c));
        }
    }
    Ok(())
}

/// The groups of a table: see the module documentation and `docs/agenti/gruppi.md`.
pub fn build(tb: &GroupTable, cs: &[Campione], opts: &GroupOpts) -> Result<GroupsOut> {
    check(tb, cs, opts)?;
    let conds = presenza::conditions(cs);
    if conds.is_empty() {
        return Err(Error::new("error.gruppi.noConditions"));
    }
    check_factors(opts, &conds)?;
    let ref_ix = opts
        .reference
        .as_ref()
        .and_then(|r| conds.iter().position(|c| &c.id == r));
    let cx = Ctx {
        tb,
        opts,
        ref_ix,
        has_ms2: tb.rows.iter().any(|m| !m.ms2_files.is_empty()),
        conds,
    };
    let rows: Vec<RowOut> = (0..tb.n_rows).map(|r| row_out(&cx, r)).collect();
    let sets = insiemi::sets(&cx.conds, &rows);
    let conditions = cond_outs(&cx);
    let warnings = warnings(&cx);
    Ok(GroupsOut {
        reference: ref_ix.and_then(|_| opts.reference.clone()),
        conditions,
        rows,
        sets,
        warnings,
    })
}

/// What one condition says about one row before the comparison with the reference.
pub(crate) struct CondCalc {
    pub stats: Vec<GStat>,
    pub present: bool,
    pub ymax: f64,
    pub kin: Vec<KinPt>,
}

fn calc(cx: &Ctx, r: usize, c: &CondModel) -> CondCalc {
    let stats: Vec<GStat> = c
        .groups
        .iter()
        .map(|g| presenza::group_stat(cx.tb, r, g))
        .collect();
    let present = presenza::is_present(&stats, cx.opts);
    let ymax = stats.iter().map(|s| s.mean).fold(0.0, f64::max);
    let kin = stats
        .iter()
        .filter_map(|s| {
            s.t.map(|t| KinPt {
                t,
                area: s.mean,
                sd: s.sd,
                n: s.n_used,
            })
        })
        .collect();
    CondCalc {
        stats,
        present,
        ymax,
        kin,
    }
}

fn cond_row(cx: &Ctx, r: usize, c: &CondModel, k: &CondCalc) -> CondRow {
    let end = k
        .stats
        .iter()
        .rposition(|s| s.t.is_some())
        .or(k.stats.len().checked_sub(1));
    let e = end.map(|i| &k.stats[i]);
    let area = e.map_or(0.0, |s| s.mean);
    let sd = e.and_then(|s| s.sd);
    let shape = k
        .present
        .then(|| forme::shape(&k.kin, cx.opts.shape_drop))
        .flatten();
    let meta = &cx.tb.rows[r];
    let prev = (k.present && !meta.is_parent).then(|| prev::prev(cx, r, c, k.ymax));
    CondRow {
        id: c.id.clone(),
        present: k.present,
        n_det: e.map_or(0, |s| s.n_det),
        n_rep: e.map_or(0, |s| s.n),
        area,
        area_sd: sd,
        cv: sd.filter(|_| area > 0.0).map(|s| s / area),
        ratio_ref: None,
        shape,
        t_max: k.present.then(|| forme::t_max(&k.kin)).flatten(),
        t_half_rise: shape.and_then(|s| forme::t_half_rise(s, &k.kin)),
        rate_rel: None,
        kin: k.kin.clone(),
        prev,
    }
}

/// The areas of two courses at the highest time they share.
fn common_ratio(a: &[KinPt], b: &[KinPt]) -> Option<f64> {
    for p in a.iter().rev() {
        if let Some(q) = b.iter().find(|q| (q.t - p.t).abs() < 1e-9) {
            return (q.area > 0.0).then(|| p.area / q.area);
        }
    }
    None
}

/// Ratio to the reference and relative rate, which need the reference row of the same ion.
fn against_ref(cond: &mut [CondRow], calcs: &[CondCalc], ri: usize) {
    let t_ref = cond[ri].t_half_rise;
    for i in 0..cond.len() {
        cond[i].ratio_ref = if !calcs[ri].present {
            None
        } else if i == ri {
            Some(1.0)
        } else if !calcs[i].present {
            Some(0.0)
        } else {
            common_ratio(&calcs[i].kin, &calcs[ri].kin)
        };
        cond[i].rate_rel = match (t_ref, cond[i].t_half_rise) {
            (Some(tr), Some(tc)) if tc > 0.0 => Some(tr / tc),
            _ => None,
        };
    }
}

fn row_out(cx: &Ctx, r: usize) -> RowOut {
    let calcs: Vec<CondCalc> = cx.conds.iter().map(|c| calc(cx, r, c)).collect();
    let mut cond: Vec<CondRow> = cx
        .conds
        .iter()
        .zip(&calcs)
        .map(|(c, k)| cond_row(cx, r, c, k))
        .collect();
    if let Some(ri) = cx.ref_ix {
        against_ref(&mut cond, &calcs, ri);
    }
    let present_in: Vec<String> = cond
        .iter()
        .filter(|c| c.present)
        .map(|c| c.id.clone())
        .collect();
    let many = cx.conds.len() >= 2;
    RowOut {
        meta: cx.tb.rows[r].clone(),
        only_in: (many && present_in.len() == 1).then(|| present_in[0].clone()),
        in_all: many && present_in.len() == cx.conds.len(),
        present_in,
        cond,
    }
}

fn cond_outs(cx: &Ctx) -> Vec<CondOut> {
    let parent = cx.tb.rows.iter().position(|m| m.is_parent);
    cx.conds
        .iter()
        .map(|c| {
            let mut factors = cx
                .opts
                .factors
                .iter()
                .find(|(n, _)| n == &c.id)
                .map(|(_, f)| f.clone())
                .unwrap_or_default();
            factors.sort();
            CondOut {
                id: c.id.clone(),
                factors,
                files: c.files.clone(),
                times: c.times(),
                replicates: c.groups.iter().map(|g| g.files.len()).max().unwrap_or(0),
                controls: c.ctrl.clone(),
                parent: parent.map_or(
                    ParentOut {
                        k: None,
                        half_life: None,
                        remaining: None,
                    },
                    |p| presenza::parent(cx, p, c),
                ),
            }
        })
        .collect()
}

fn warn(code: &'static str, c: &str) -> Warning {
    Warning {
        code,
        condition: c.to_string(),
    }
}

/// Warnings in the order of the codes, then of the conditions.
fn warnings(cx: &Ctx) -> Vec<Warning> {
    let mut w = Vec::new();
    if let (Some(r), None) = (&cx.opts.reference, cx.ref_ix) {
        w.push(warn("refMissing", r));
    }
    let times: Vec<Vec<f64>> = cx.conds.iter().map(|c| c.times()).collect();
    let base = cx.ref_ix.unwrap_or(0);
    for (i, c) in cx.conds.iter().enumerate() {
        if c.ctrl.t0.is_empty() && c.ctrl.blank.is_empty() {
            w.push(warn("condNoControls", &c.id));
        }
        if times[i].len() == 1 {
            w.push(warn("condOnePoint", &c.id));
        }
        if c.groups.iter().all(|g| g.files.len() == 1) {
            w.push(warn("condNoReplicates", &c.id));
        }
        if i != base && times[i] != times[base] {
            w.push(warn("mixedTimes", &c.id));
        }
    }
    let rank = |c: &str| {
        [
            "refMissing",
            "condNoControls",
            "condOnePoint",
            "condNoReplicates",
            "mixedTimes",
        ]
        .iter()
        .position(|k| *k == c)
        .unwrap_or(9)
    };
    // Stable: the conditions keep their order inside a code.
    let mut ix: Vec<usize> = (0..w.len()).collect();
    mzlab_core::sortc::by(&mut ix, |&a, &b| rank(w[a].code).cmp(&rank(w[b].code)));
    ix.into_iter().map(|i| w[i].clone()).collect()
}
