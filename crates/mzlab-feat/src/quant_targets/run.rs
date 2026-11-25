//! One pass per file: the targets grouped by polarity, one sweep of the index per group (every XIC at once, cut in chunks the memory budget
//! takes), each trace through the `peak auto-lr` engine, then the blank flags once every file is done. Every automatic choice goes to the log.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{round, Value};
use mzlab_core::mzcal::{Offset, Spec};
use mzlab_core::rawxic::tallest;
use mzlab_core::routes::Routes;

use super::gates;
use super::params::{OffsetMode, Params};
use super::targets::{parse, Role, Target};
use super::text;

/// The most files a caller reads at once: past a few, one disk only seeks between the files, and the memory budget is shared by them.
pub const MAX_WORKERS: usize = 8;

/// The tallest peak of one target in one file (raw trace, rounded like `trace xic`), or why there is none: the type `raw xic` shares.
pub(crate) use mzlab_core::rawxic::Hit;

/// One row of the long table.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Row {
    pub file: usize,
    pub target: usize,
    pub hit: Option<Hit>,
    pub flags: Vec<&'static str>,
    pub peaks: usize,
    pub half: usize,
    pub offset: f64,
}

impl Row {
    /// Detected: a peak with S/N and shape over the thresholds and its scans present.
    pub fn detected(&self) -> bool {
        self.hit.is_some()
            && !self
                .flags
                .iter()
                .any(|f| matches!(*f, "no_scans" | "no_peak" | "below_threshold" | "bad_shape"))
    }

    /// The state of the value: `detected`, `forced` (a peak under `min_sn` or the shape minimum, its numbers kept and flagged) or
    /// `absent` (no peak in the window, or no scan). No value is gap-filled, so there is no `filled` state.
    pub fn state(&self) -> &'static str {
        match (&self.hit, self.detected()) {
            (None, _) => "absent",
            (Some(_), true) => "detected",
            (Some(_), false) => "forced",
        }
    }
}

/// The offset in force for one polarity group of one file.
pub(crate) struct GroupOffset {
    pub polarity: Option<i8>,
    pub offset: Offset,
    /// The reference row it was measured on (`offset = auto`).
    pub reference: Option<String>,
}

pub(crate) struct FileLog {
    pub name: String,
    pub blank: bool,
    pub offsets: Vec<GroupOffset>,
    pub detected: usize,
    /// Rows with a peak that missed a check (state `forced`): their evidence is in the log `decisions`.
    pub forced: usize,
    /// The weakest detected rows of the file (target id and value): how close the accepted peaks are to the thresholds.
    pub min_sn: Option<(String, f64)>,
    pub min_half: Option<(String, usize)>,
}

/// An automatic choice of the run: what, the value taken, why.
pub(crate) struct Choice {
    pub key: &'static str,
    pub value: String,
    pub reason: String,
}

/// What a run made: the long table (CSV), the log (`quant-targets-log/1`) and the warning keys; `json` writes the same rows as
/// `quant-targets/1` from the rows themselves.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub csv: String,
    pub log: String,
    pub warnings: Vec<String>,
    rows: Vec<Row>,
    names: Vec<String>,
    targets: Vec<Target>,
}

impl Outcome {
    /// The rows and the log as `quant-targets/1`.
    pub fn json(&self) -> String {
        text::json(&self.rows, &self.names, &self.targets, &self.log)
    }
}

/// The polarity groups of the targets in first-seen order.
fn groups(targets: &[Target]) -> Vec<Option<i8>> {
    let mut g: Vec<Option<i8>> = Vec::new();
    for t in targets {
        if !g.contains(&t.polarity) {
            g.push(t.polarity);
        }
    }
    g
}

/// The reference row `offset = auto` measures a group on: the first of that polarity, else the first without one.
fn reference(targets: &[Target], pol: Option<i8>) -> Option<&Target> {
    let refs = || targets.iter().filter(|t| t.role == Role::Reference);
    refs()
        .find(|t| t.polarity == pol)
        .or_else(|| refs().find(|t| t.polarity.is_none()))
}

/// The offset of each polarity group of one file, measured before the sweep takes the index.
fn offsets<R: ReadAt>(
    idx: &mut Index<R>,
    targets: &[Target],
    p: &Params,
) -> Result<Vec<GroupOffset>> {
    let mut out = Vec::new();
    for pol in groups(targets) {
        let (offset, reference) = match p.offset {
            OffsetMode::Fixed(da) => (Offset { da, auto: None }, None),
            OffsetMode::Ion(s) => (s.resolve(idx, pol)?, None),
            OffsetMode::Auto => match reference(targets, pol) {
                None => (Offset::default(), None),
                Some(r) => {
                    let s = Spec::Auto {
                        mz: r.mz,
                        rt: Some(round((r.rt_min + r.rt_max) / 2.0, 4)),
                    };
                    (s.resolve(idx, pol)?, Some(r.id.clone()))
                }
            },
        };
        out.push(GroupOffset {
            polarity: pol,
            offset,
            reference,
        });
    }
    Ok(out)
}

/// The tallest peak of a trace between `x0` and `x1` and how many the window holds (`rawxic::tallest`, which `raw xic` calls too).
fn integrate(rt: &[f64], y: &[f64], t: &Target, p: &Params) -> (Option<Hit>, usize, usize, bool) {
    tallest(rt, y, t.rt_min, t.rt_max, p.smooth_half, p.valley_pct)
}

/// The detected row with the smallest `key` of its peak: (target id, value). The first one wins a tie, so the log is stable.
fn weakest(rows: &[Row], targets: &[Target], key: impl Fn(&Hit) -> f64) -> Option<(String, f64)> {
    rows.iter()
        .filter(|r| r.detected())
        .filter_map(|r| r.hit.as_ref().map(|h| (r.target, key(h))))
        .fold(None, |best: Option<(usize, f64)>, (t, v)| match best {
            Some((_, b)) if b <= v => best,
            _ => Some((t, v)),
        })
        .map(|(t, v)| (targets[t].id.clone(), v))
}

/// The flags of a row from its hit (`in_blank` comes after every file).
fn flags(hit: &Option<Hit>, edge: bool, p: &Params) -> Vec<&'static str> {
    let Some(h) = hit else {
        return vec!["no_peak"];
    };
    let mut f: Vec<&'static str> = Vec::new();
    for c in gates::checks(h, p).iter().filter(|c| !c.pass) {
        let w = gates::flag_of(c.name);
        if !f.contains(&w) {
            f.push(w);
        }
    }
    if edge {
        f.push("edge");
    }
    f
}

/// The rows of one file, in the order of the targets.
fn one_file<R: ReadAt>(
    idx: Index<R>,
    k: usize,
    targets: &[Target],
    p: &Params,
    offs: &[GroupOffset],
    workers: usize,
) -> Result<Vec<Row>> {
    let mut routes = Routes::new(idx);
    let mut rows: Vec<Option<Row>> = vec![None; targets.len()];
    for g in offs {
        routes.pol = g.polarity;
        routes.mz_off = g.offset.da;
        let members: Vec<usize> = (0..targets.len())
            .filter(|&i| targets[i].polarity == g.polarity)
            .collect();
        let n = routes.xic_len(p.ms_level);
        // the columns a sweep may hold within half the budget shared by the files read at once (`sweep_matrix` refuses more than half of
        // it): the chunks change nothing in a column
        let per = (mzlab_core::budget::TARGET_BYTES / 2 / workers.max(1) / (8 * n.max(1))).max(1);
        for chunk in members.chunks(per) {
            let centres: Vec<f64> = chunk.iter().map(|&i| targets[i].mz).collect();
            // the memo of the previous chunk is dropped first, so one matrix at a time is held
            routes.forget_sweep();
            let m = routes.sweep_matrix(&centres, &vec![p.da; centres.len()], p.ms_level, None)?;
            let rt: Vec<f64> = m.rt.iter().map(|&x| round(x, 4)).collect();
            for (c, &i) in chunk.iter().enumerate() {
                let row = if rt.is_empty() {
                    Row {
                        file: k,
                        target: i,
                        hit: None,
                        flags: vec!["no_scans"],
                        peaks: 0,
                        half: 0,
                        offset: g.offset.da,
                    }
                } else {
                    let y: Vec<f64> = m.y[c].iter().map(|&v| round(v, 1)).collect();
                    let (hit, peaks, half, edge) = integrate(&rt, &y, &targets[i], p);
                    let flags = flags(&hit, edge, p);
                    Row {
                        file: k,
                        target: i,
                        hit,
                        flags,
                        peaks,
                        half,
                        offset: g.offset.da,
                    }
                };
                rows[i] = Some(row);
            }
            drop(m);
            routes.forget_sweep();
        }
    }
    Ok(rows.into_iter().flatten().collect())
}

/// A run checked and ready: parameters, targets, file names with their blank flag, and the choices and warnings known before any file is read.
/// Each file is then one independent `file` pass (any order, any thread) and `finish` joins them in file order, so the bytes do not depend on
/// how the passes were scheduled.
pub struct Job {
    p: Params,
    targets: Vec<Target>,
    names: Vec<String>,
    blank: Vec<bool>,
    choices: Vec<Choice>,
    warnings: Vec<String>,
    workers: usize,
}

/// The rows, the log entry and the warning keys of one file.
pub struct FilePass {
    rows: Vec<Row>,
    log: FileLog,
    warnings: Vec<&'static str>,
}

fn warn(w: &str, list: &mut Vec<String>) {
    if !list.iter().any(|x| x == w) {
        list.push(w.to_string());
    }
}

/// Parameters and targets checked, the blank files and the run-wide choices made; no data read.
pub fn prepare(names: &[String], targets: &[u8], params: &[(String, Value)]) -> Result<Job> {
    let p = Params::of(params)?;
    let targets = parse(targets)?;
    if names.is_empty() {
        return Err(Error::new("err.quant.targets.noFiles"));
    }
    let mut choices: Vec<Choice> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let blank: Vec<bool> = names.iter().map(|n| p.is_blank(n)).collect();
    if !p.blank_pattern.trim().is_empty() && !blank.contains(&true) {
        warn("warn.quant.targets.noBlank", &mut warnings);
    }
    if p.offset == OffsetMode::Auto {
        for pol in groups(&targets) {
            let (value, reason) = match reference(&targets, pol) {
                Some(r) => (
                    r.id.clone(),
                    "first standard/parent row of this polarity".to_string(),
                ),
                None => {
                    warn("warn.quant.targets.noReference", &mut warnings);
                    (
                        "0".to_string(),
                        "no standard/parent row: offset 0".to_string(),
                    )
                }
            };
            choices.push(Choice {
                key: text::pol_key(pol),
                value,
                reason,
            });
        }
    }
    if p.smooth_half == 0 {
        choices.push(Choice {
            key: "smooth_half",
            value: "auto".into(),
            reason: "per trace from the FWHM of its tallest peak (column smooth_half)".into(),
        });
    }
    Ok(Job {
        p,
        targets,
        names: names.to_vec(),
        blank,
        choices,
        warnings,
        workers: 1,
    })
}

impl Job {
    /// The files the caller reads at once: each pass then takes that share of the memory budget for its sweeps (the bytes do not change).
    pub fn with_workers(mut self, workers: usize) -> Self {
        self.workers = workers.max(1);
        self
    }

    /// How many files the run has.
    pub fn files(&self) -> usize {
        self.names.len()
    }

    /// The pass over file `k` (its index `idx`): the offset of every polarity group, then one sweep per group.
    pub fn file<R: ReadAt>(&self, k: usize, mut idx: Index<R>) -> Result<FilePass> {
        let offs = offsets(&mut idx, &self.targets, &self.p)?;
        let mut warnings: Vec<&'static str> =
            offs.iter().filter_map(|g| g.offset.warning()).collect();
        let rows = one_file(idx, k, &self.targets, &self.p, &offs, self.workers)?;
        if rows.iter().any(|x| x.flags.contains(&"no_scans")) {
            warnings.push("warn.quant.targets.noScans");
        }
        let log = FileLog {
            name: self.names[k].clone(),
            blank: self.blank[k],
            offsets: offs,
            detected: rows.iter().filter(|x| x.detected()).count(),
            forced: rows.iter().filter(|x| x.state() == "forced").count(),
            min_sn: weakest(&rows, &self.targets, |h| h.sn),
            min_half: weakest(&rows, &self.targets, |h| h.half_pts as f64)
                .map(|(id, v)| (id, v as usize)),
        };
        Ok(FilePass {
            rows,
            log,
            warnings,
        })
    }

    /// The passes of every file, in file order, joined: the blank flags, the table and the log.
    pub fn finish(&self, passes: Vec<FilePass>) -> Outcome {
        let mut warnings = self.warnings.clone();
        let mut rows: Vec<Row> = Vec::new();
        let mut files: Vec<FileLog> = Vec::new();
        for mut f in passes {
            for w in &f.warnings {
                warn(w, &mut warnings);
            }
            rows.append(&mut f.rows);
            files.push(f.log);
        }
        let blank = &self.blank;
        // a target detected in a blank flags it in every other file: one pass over the rows
        let mut in_blank = vec![false; self.targets.len()];
        for r in &rows {
            if blank[r.file] && r.detected() {
                in_blank[r.target] = true;
            }
        }
        if rows.iter().any(|r| r.state() == "forced") {
            warn("warn.quant.targets.forced", &mut warnings);
        }
        for r in rows.iter_mut() {
            if in_blank[r.target] && !blank[r.file] && r.hit.is_some() {
                r.flags.push("in_blank");
            }
        }
        Outcome {
            csv: text::table(&rows, &self.names, &self.targets),
            log: text::log(
                &self.p,
                &files,
                &self.choices,
                &warnings,
                &rows,
                &self.names,
                &self.targets,
            ),
            warnings,
            rows,
            names: self.names.clone(),
            targets: self.targets.clone(),
        }
    }
}

/// Every target over every file `open(k)` serves (`names[k]` its file name), one file after the other: the long table, the log and the warnings.
pub fn run<R: ReadAt>(
    open: &mut dyn FnMut(usize) -> Result<Index<R>>,
    names: &[String],
    targets: &[u8],
    params: &[(String, Value)],
) -> Result<Outcome> {
    let job = prepare(names, targets, params)?;
    let mut passes = Vec::with_capacity(job.files());
    for k in 0..job.files() {
        passes.push(job.file(k, open(k)?)?);
    }
    Ok(job.finish(passes))
}

/// What a run would do, reading no data (`--dry-run`): `quant-targets-plan/1`.
pub fn plan(names: &[String], targets: &[u8], params: &[(String, Value)]) -> Result<String> {
    let p = Params::of(params)?;
    let targets = parse(targets)?;
    if names.is_empty() {
        return Err(Error::new("err.quant.targets.noFiles"));
    }
    Ok(text::plan(&p, names, &targets))
}
