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
    /// `dedup_overlap` of `features.gapfill`: the least share of the narrower peak two rows must overlap in a file where both were detected for the
    /// `overlap` rule to merge them; `None` is the default (`lrscreen_dedup::DEFAULT_OVERLAP`), 0 turns the rule off.
    pub dedup_overlap: Option<f64>,
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
    /// The switches (`read`) and the number `dedup_overlap` of the params object `v`; a value outside 0..=1 or a missing key keeps the threshold.
    pub fn read_json(&mut self, v: &mzlab_core::json::Value) {
        use mzlab_core::json::Value;
        self.read(|k| v.get(k).and_then(Value::str));
        if let Some(x) = v
            .get("dedup_overlap")
            .and_then(Value::f64)
            .filter(|x| (0.0..=1.0).contains(x))
        {
            self.dedup_overlap = Some(x);
        }
    }

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

/// A row dropped because it is the same ion as the one kept: (m/z, RT) of each, the distances and the evidence of the rule that merged them. `r` (Pearson
/// r of the areas) and `overlap` (median share of the narrower peak the two cover, over the files where both were detected) are NaN when not computable.
#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    pub kept: (f64, f64),
    pub dropped: (f64, f64),
    pub dmz_da: f64,
    pub drt_min: f64,
    pub r: f64,
    /// `correlation` (r at or above `MIN_R`) or `overlap` (overlap at or above `threshold`).
    pub rule: &'static str,
    pub overlap: f64,
    /// Files where both rows were detected / have an area, and the threshold of the rule that decided.
    pub shared: usize,
    pub threshold: f64,
    /// Highest cell height of the kept and the dropped row.
    pub height: (f64, f64),
}

/// Two rows left within the dedup window: the numbers `dedup` saw and why neither rule merged them.
#[derive(Debug, Clone, PartialEq)]
pub struct Near {
    pub a: (f64, f64),
    pub b: (f64, f64),
    pub dmz_da: f64,
    pub drt_min: f64,
    pub r: f64,
    pub overlap: f64,
    pub shared: usize,
}

#[derive(Debug, Default)]
pub struct Log {
    pub centroided: Vec<Centroided>,
    pub shifts: Vec<Shift>,
    /// The m/z offset of each file (`mz_offset` of `features.align`), in the order of the files.
    pub offsets: Vec<mzlab_core::mzcal::Offset>,
    pub merged: Vec<Merge>,
    /// Pairs of rows still within the dedup window after the merges (the hint that `dedup_overlap` exists).
    pub near: Vec<Near>,
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

/// Whether `r` centroids this file before detecting: the unit axis, and `on`, or `auto` with at least half of the MS1 scans in profile mode.
pub fn centroid_wanted<R: ReadAt>(index: &Index<R>, r: &Recipe) -> bool {
    if !r.is_lr() {
        return false;
    }
    match r.screen.centroid {
        Switch::Off => false,
        Switch::On => true,
        Switch::Auto => {
            let (p, n) = ms1_share(index);
            is_profile(p, n)
        }
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
