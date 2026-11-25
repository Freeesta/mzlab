//! Untargeted screening at unit resolution across samples: the three switches of the recipe (`centroid` of `features.detect`, `recenter` of
//! `features.align`, `dedup` of `features.gapfill`), the decision to centroid a file, and the evidence the steps leave for a report. They act only on
//! the unit axis; a high-resolution recipe ignores them and its text and bytes do not change. Details in `docs/agenti/mzfeat-lr.md`.

use std::sync::{Arc, Mutex};

use mzlab_core::centroids::{centroid_index, Over};
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::recipe::Recipe;

/// `auto` decides from the data, `on` forces, `off` keeps the path of before (`recenter` and `dedup` have no `on`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Switch {
    #[default]
    Auto,
    On,
    Off,
}

impl Switch {
    pub fn name(self) -> &'static str {
        match self {
            Switch::Auto => "auto",
            Switch::On => "on",
            Switch::Off => "off",
        }
    }

    /// `auto`, `off`, and `on` only when `allow_on`.
    pub fn parse(s: &str, allow_on: bool) -> Option<Switch> {
        match s {
            "auto" => Some(Switch::Auto),
            "off" => Some(Switch::Off),
            "on" if allow_on => Some(Switch::On),
            _ => None,
        }
    }
}

/// The three switches and the optional collector of what they did.
#[derive(Debug, Clone, Default)]
pub struct Screen {
    pub centroid: Switch,
    pub recenter: Switch,
    pub dedup: Switch,
    /// Set by `adjusted` when `centroid` acts on the file or the series the recipe was made for (profile scans centroided). Only then does `auto` enter the
    /// text: a file that is centroided already keeps the recipe text, and so the bytes, it had before the switch existed.
    pub centroided: bool,
    /// `mz_offset` of `features.align`: 0, a number of Da, or the known ion to measure it on, per file (`lrscreen_offset`).
    pub mz_offset: mzlab_core::mzcal::Spec,
    /// The offset of each file as resolved for the run in hand (empty: none), subtracted from the features' m/z and added to the fill windows. Set by
    /// `lrscreen_offset::resolve` at the start of a run; never part of the text.
    pub offsets: Offsets,
    pub evidence: Option<Evidence>,
}

impl Screen {
    /// The switches `get` names (`centroid`, `recenter`, `dedup`) as texts; an unknown text or a missing key keeps the switch. Only `centroid` has `on`.
    pub fn read<'a>(&mut self, get: impl Fn(&str) -> Option<&'a str>) {
        let pick = |k: &str, on: bool, now: Switch| {
            get(k).and_then(|s| Switch::parse(s, on)).unwrap_or(now)
        };
        self.centroid = pick("centroid", true, self.centroid);
        self.recenter = pick("recenter", false, self.recenter);
        self.dedup = pick("dedup", false, self.dedup);
        if let Some(s) = get("mz_offset").and_then(|t| mzlab_core::mzcal::Spec::parse(t).ok()) {
            self.mz_offset = s;
        }
    }
}

/// A file counts as profile from half of its MS1 scans, the share at which the recipe takes a per-point floor and the warning of `features` speaks.
pub fn is_profile(profile: usize, scans: usize) -> bool {
    scans > 0 && 2 * profile >= scans
}

/// One automatic choice with the reason the user can read.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub key: String,
    pub value: String,
    pub reason: String,
}

/// A file as it was centroided: its name in the `.mzfeatf` and the width the centroider measured.
#[derive(Debug, Clone, PartialEq)]
pub struct Centroided {
    pub name: String,
    pub fwhm: f64,
}

/// The shift of one file's m/z measured against the series: Da, the anchor rows it rests on, and whether it was subtracted.
#[derive(Debug, Clone, PartialEq)]
pub struct Shift {
    pub mz_da: f64,
    pub anchors: usize,
    pub applied: bool,
}

/// A row dropped because it is the same ion as the one kept: (m/z, RT) of each, the distances and the Pearson r of their areas.
#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    pub kept: (f64, f64),
    pub dropped: (f64, f64),
    pub dmz_da: f64,
    pub drt_min: f64,
    pub r: f64,
}

#[derive(Debug, Default)]
pub struct Log {
    pub centroided: Vec<Centroided>,
    pub shifts: Vec<Shift>,
    /// The m/z offset of each file (`mz_offset` of `features.align`), in the order of the files.
    pub offsets: Vec<mzlab_core::mzcal::Offset>,
    pub merged: Vec<Merge>,
    pub choices: Vec<Choice>,
}

/// The per-file m/z offsets of the run in hand, shared by the clones of a recipe (a cell, so the run sets them without copying the recipe).
#[derive(Debug, Clone, Default)]
pub struct Offsets(Arc<Mutex<Vec<f64>>>);

impl Offsets {
    pub(crate) fn set(&self, v: Vec<f64>) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = v;
    }

    /// The offset of file `k` (0 when there is none).
    pub(crate) fn get(&self, k: usize) -> f64 {
        let g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        g.get(k).copied().unwrap_or(0.0)
    }
}

/// What the steps did, shared by the threads of one run. Not part of the recipe text: it never changes a byte of a table.
#[derive(Debug, Clone, Default)]
pub struct Evidence(Arc<Mutex<Log>>);

impl Evidence {
    pub fn new() -> Evidence {
        Evidence::default()
    }

    pub(crate) fn with(&self, f: impl FnOnce(&mut Log)) {
        f(&mut self.0.lock().unwrap_or_else(|e| e.into_inner()));
    }

    /// A copy of what was collected.
    pub fn take(&self) -> Log {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *g)
    }
}

impl Recipe {
    /// Whether this is the unit axis, the only one the switches act on.
    pub fn is_lr(&self) -> bool {
        self.detect.axis.is_da()
    }

    /// The part of the file text the switch adds: on the unit axis, `centroid=on` when forced and `centroid=auto` when it centroided the profile scans
    /// (the cache of a file made on the profile points must not answer for its centroids). Nothing for `off`, for a file that needed no centroiding, and
    /// for the high-resolution axis. `recenter` and `dedup` act on the table only, so they add nothing to the text of a file.
    pub(crate) fn screen_file_text(&self) -> String {
        let s = &self.screen;
        match s.centroid {
            Switch::On if self.is_lr() => "\ncentroid=on".into(),
            Switch::Auto if self.is_lr() && s.centroided => "\ncentroid=auto".into(),
            _ => String::new(),
        }
    }
}

/// An MS1 scan share of profile mode, the way the warning counts it (all the MS1 scans).
fn ms1_share<R: ReadAt>(index: &Index<R>) -> (usize, usize) {
    let ms1 = || index.scans.iter().filter(|s| s.level == 1);
    (ms1().filter(|s| s.profile).count(), ms1().count())
}

/// What the points of a file labelled profile say about the label (`spectrum_type::Runs` on up to `PROBE` MS1 scans, the detector of `raw info`).
#[derive(Debug, Clone, PartialEq)]
pub struct LabelCheck {
    /// MS1 scans labelled profile and all MS1 scans.
    pub profile: usize,
    pub scans: usize,
    /// Scans probed, and the points and neighbour spacings they hold.
    pub probed: usize,
    pub points: usize,
    pub spacings: usize,
    /// Share of the probed points that sit in runs of a profile peak, and of the neighbour spacings of a fine grid, in whole percent.
    pub runs_pct: usize,
    pub fine_pct: usize,
    /// The points are sparse ion counts (no run, no fine grid): centroids under a profile label.
    pub sparse: bool,
}

/// The cut-offs behind `LabelCheck::sparse`, the ones of `raw info` (`spectrum_type`): whole numbers for the notice, the report and the log.
pub struct Cuts {
    /// Sparse below this share of the points in runs (percent); a fine grid from this share of the spacings (percent).
    pub runs_pct: usize,
    pub fine_pct: usize,
    /// A run: at least this many points, no further apart than this (mDa) on average; a fine spacing: under this (mDa).
    pub run_points: usize,
    pub run_step_mda: usize,
    pub fine_step_mda: usize,
}

pub fn cuts() -> Cuts {
    use mzlab_core::spectrum_type as t;
    let pct = |v: f64| (v * 100.0).round() as usize;
    let mda = |v: f64| (v * 1000.0).round() as usize;
    Cuts {
        runs_pct: pct(t::SPARSE_BELOW),
        fine_pct: pct(t::FINE_SHARE),
        run_points: t::RUN_POINTS,
        run_step_mda: mda(t::MAX_STEP),
        fine_step_mda: mda(t::FINE_STEP),
    }
}

/// The label of an MS1 level against its points: `None` unless at least half of its scans are labelled profile (a centroid label is never questioned).
pub fn label_check<R: ReadAt>(index: &mut Index<R>) -> Option<LabelCheck> {
    let (profile, scans) = ms1_share(index);
    if !is_profile(profile, scans) {
        return None;
    }
    let p = mzlab_core::rawinfo::probe_index(index, 1, None, mzlab_core::rawinfo::PROBE).ok()?;
    let (_, _, runs) = mzlab_core::rawinfo::spectrum_type(Some(&p));
    let pct = |v: Option<f64>| v.map_or(0, |v| (v * 100.0).round() as usize);
    Some(LabelCheck {
        profile,
        scans,
        probed: p.scans.len(),
        points: runs.points(),
        spacings: runs.spacings(),
        runs_pct: pct(runs.runs_share()),
        fine_pct: pct(runs.fine_share()),
        sparse: runs.runs_share().is_some() && runs.sparse(),
    })
}

/// Whether `r` centroids this file before detecting: the unit axis, and `on`, or `auto` with at least half of the MS1 scans labelled profile whose
/// points are not sparse ion counts (the label alone never decides: centroiding counts that are centroids already costs time and counts).
pub fn centroid_wanted<R: ReadAt>(index: &mut Index<R>, r: &Recipe) -> bool {
    if !r.is_lr() {
        return false;
    }
    match r.screen.centroid {
        Switch::Off => false,
        Switch::On => true,
        Switch::Auto => label_check(index).is_some_and(|c| !c.sparse),
    }
}

/// The centroids of a file with profile scans, read like any index (`None`: the recipe does not centroid it, or it has no profile scan). The reader is
/// the boxed one the `.wasm` opens every index with, so the detection of a file and of its centroids is one compiled copy there.
pub struct Centroids {
    pub index: Index<Box<dyn ReadAt>>,
    pub fwhm: f64,
}

pub fn centroids_of<R: ReadAt>(index: &mut Index<R>, r: &Recipe) -> Result<Option<Centroids>> {
    if !centroid_wanted(index, r) {
        return Ok(None);
    }
    let Some(c) = centroid_index(index, Over::default())? else {
        return Ok(None);
    };
    let size = c.bytes.len() as u64;
    Ok(Some(Centroids {
        index: Index::open(Box::new(c.bytes) as Box<dyn ReadAt>, size)?,
        fwhm: c.estimate.params.w,
    }))
}

/// `r` for the centroids of the file when it is centroided: the channel defect and the floors come from the centroid points (a per-point floor of a
/// profile file would cut the centroid areas), unless the recipe holds a value of its own. The same recipe is used to detect, to fill and to rebuild.
pub fn adjusted<R: ReadAt>(index: &mut Index<R>, mut r: Recipe) -> Result<Recipe> {
    if r.is_lr()
        && r.screen.centroid == Switch::Auto
        && let Some(c) = label_check(index).filter(|c| c.sparse)
    {
        choose(&r, "centroid", "off", label_reason(&c));
    }
    let Some(c) = centroids_of(index, &r)? else {
        return Ok(r);
    };
    r.screen.centroided = true;
    let mut c = c.index;
    let pol = r.detect.polarity;
    let (was, now) = (
        Recipe::for_index_in(index, pol)?,
        Recipe::for_index_in(&mut c, pol)?,
    );
    if r.detect.axis == was.detect.axis {
        r.detect.axis = now.detect.axis;
    }
    if r.detect.floor == was.detect.floor {
        r.detect.floor = now.detect.floor;
    }
    if r.fill.floor == was.fill.floor {
        r.fill.floor = now.fill.floor;
    }
    Ok(r)
}

/// The reason logged when `auto` leaves a profile-labelled file as it is: the numbers, the cut-offs they were judged by, and what the user can do instead.
pub fn label_reason(c: &LabelCheck) -> String {
    let k = cuts();
    format!(
        "{} of {} MS1 scans are labelled profile but their points are sparse ion counts: of {} points in {} scans probed, {} % are in runs (at least {} points, {} mDa apart at most; sparse below {} %) and {} % of the {} spacings are under {} mDa (a fine grid from {} %); centroid skipped; centroid=on forces it, file convert --spectrum centroid relabels",
        c.profile, c.scans, c.points, c.probed, c.runs_pct, k.run_points, k.run_step_mda, k.runs_pct, c.fine_pct, c.spacings, k.fine_step_mda, k.fine_pct
    )
}

/// Adds a choice to the evidence of the recipe, if it has one.
pub(crate) fn choose(r: &Recipe, key: &str, value: impl Into<String>, reason: impl Into<String>) {
    if let Some(e) = &r.screen.evidence {
        e.with(|l| {
            l.choices.push(Choice {
                key: key.into(),
                value: value.into(),
                reason: reason.into(),
            })
        });
    }
}
