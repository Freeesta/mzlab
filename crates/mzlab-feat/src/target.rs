//! The targeted pass: every hypothesis of `hypo::Registry` is searched in all the files, with lower thresholds than the detection and in the
//! window of RT that its rule allows. What is found enters as `CELL_TARGETED` linked to the hypothesis; a peak the detection had already found
//! stays `CELL_DETECTED` and receives the proof «predicted by» (`TargetResult::evidence`). One pass, in the order of the registry.
//!
//! Against false positives: a row without a detected cell and without a linked MS2 never goes above level 4 (`TargetRow::level_cap`, read by the
//! identity grading); the registry keeps how many hypotheses were tried and how many were found; and every row with a targeted cell carries the
//! chance that random ions of the same density give as many hits in the same window (`null`, Poisson). The detection table of each file must
//! come from `pipeline::analyse` (its MS2 links are counted).

mod extract;
pub mod null;

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::hypo::{Dir, Hypothesis, Outcome, Registry};
use crate::ms2::{self, Ms2Links, Ms2Params};
use crate::table::{Aligned, FileFeatures, CELL_ABSENT, CELL_DETECTED, CELL_TARGETED};
use crate::trace::Params;
use extract::{detected_row, peaks, push_peak, Cand, Section};

#[derive(Debug, Clone)]
pub struct TargetParams {
    /// Polarity searched (hypotheses of the other polarity are left alone).
    pub polarity: i8,
    pub ppm: f64,
    /// Lowest S/N, shape and number of points of a peak (the quality limits of the detection; the minimum height is replaced by the S/N).
    pub min_snr: f64,
    pub min_shape: f64,
    pub min_points: u16,
    /// Intensity floor of the detection: the noise is never below a third of it.
    pub floor: f64,
    /// Scans of flank on each side of the window.
    pub pad: usize,
    /// RT of the parent, minutes: a product more polar elutes before it, a less polar one after (`Dir`); `None` = the window is the whole run.
    pub parent_rt: Option<f64>,
    /// Minutes the window goes past the parent RT on the side opposite to the direction of the rule.
    pub rt_slack: f64,
    /// Half width of the window of an expected member of a family around the RT of its base, minutes.
    pub family_tol: f64,
    /// A detected feature is the same ion as a targeted peak if the apexes differ by at most this, minutes (or one lies inside the other).
    pub rt_tol: f64,
    /// Decoy masses per file for the chance of a coincidence (0 = not computed, the chance is 1).
    pub decoys: usize,
    pub ms2: Ms2Params,
}

impl Default for TargetParams {
    fn default() -> Self {
        TargetParams {
            polarity: 1,
            ppm: 5.0,
            min_snr: 3.0,
            min_shape: 0.5,
            min_points: 5,
            floor: crate::trace::FLOOR,
            pad: 15,
            parent_rt: None,
            rt_slack: 0.3,
            family_tol: 0.1,
            rt_tol: 0.1,
            decoys: 64,
            ms2: Ms2Params::default(),
        }
    }
}

/// One file of one row: `state` is `CELL_DETECTED`, `CELL_TARGETED` or `CELL_ABSENT` (NaN values).
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub state: u8,
    pub area: f64,
    pub height: f32,
    pub apex_rt: f32,
    pub width: f32,
    pub snr: f32,
    pub ms2_n: u16,
    /// Row of the detection table of the file (detected) or of `FilePeaks::peaks` (targeted); `u32::MAX` when absent.
    pub feature: u32,
}

/// One hypothesis with a peak in at least one file.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetRow {
    pub hyp: u32,
    pub mz: f64,
    /// Median apex RT of the cells with a peak.
    pub rt: f64,
    /// Window of RT searched, minutes.
    pub window: (f32, f32),
    pub cells: Vec<Cell>,
    pub n_detected: u16,
    pub n_targeted: u16,
    /// MS2 scans linked to the peaks of the row (detected and targeted).
    pub ms2_total: u32,
    /// Chance that random ions give as many hits in this window (1 when the density was not measured).
    pub coincidence: f64,
}

impl TargetRow {
    /// The best level the row can reach: 4 for a row the detection never found (all cells targeted) and with no MS2; `None` = no cap.
    pub fn level_cap(&self) -> Option<u8> {
        (self.n_detected == 0 && self.ms2_total == 0).then_some(4)
    }
}

/// The targeted peaks of one file (rows sorted as the hypotheses) with the MS2 linked to them.
#[derive(Debug, Clone, Default)]
pub struct FilePeaks {
    pub peaks: FileFeatures,
    /// Hypothesis of every row of `peaks`.
    pub hyp: Vec<u32>,
    pub links: Ms2Links,
}

#[derive(Debug, Clone)]
pub struct TargetResult {
    pub n_files: usize,
    pub rows: Vec<TargetRow>,
    pub files: Vec<FilePeaks>,
    /// Hypotheses searched and hypotheses with a peak (detected or targeted) in at least one file.
    pub tried: u32,
    pub found: u32,
    /// Random peaks per decoy per minute, per file.
    pub decoy_rate: Vec<f64>,
}

/// The proof attached to a row: what predicted it and how much was tried.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence<'a> {
    pub hypothesis: &'a Hypothesis,
    pub tried: u32,
    pub found: u32,
    pub coincidence: f64,
    pub level_cap: Option<u8>,
}

/// Window of RT searched for a hypothesis.
pub fn window(h: &Hypothesis, p: &TargetParams, run: (f64, f64)) -> (f64, f64) {
    if let Some(r) = h.rt_ref {
        let r = r as f64;
        return (r - p.family_tol, r + p.family_tol);
    }
    match (h.dir, p.parent_rt) {
        (Dir::MorePolar, Some(r)) => (run.0, (r + p.rt_slack).min(run.1)),
        (Dir::LessPolar, Some(r)) => ((r - p.rt_slack).max(run.0), run.1),
        _ => run,
    }
}

/// What the file shows for one hypothesis: the accepted peaks and which of them the detection already had.
struct Seen {
    hyp: u32,
    cell: Cell,
}

fn cell_of_feature(f: &FileFeatures, i: usize) -> Cell {
    Cell {
        state: CELL_DETECTED,
        area: f.area[i],
        height: f.height[i],
        apex_rt: f.rt[i],
        width: f.right[i] - f.left[i],
        snr: f.snr[i],
        ms2_n: f.ms2_n[i],
        feature: i as u32,
    }
}

fn cell_of_peak(c: &Cand, row: usize) -> Cell {
    Cell {
        state: CELL_TARGETED,
        area: c.area,
        height: c.height,
        apex_rt: c.rt,
        width: c.right - c.left,
        snr: c.snr,
        ms2_n: 0,
        feature: row as u32,
    }
}

fn absent() -> Cell {
    Cell {
        state: CELL_ABSENT,
        area: f64::NAN,
        height: f32::NAN,
        apex_rt: f32::NAN,
        width: f32::NAN,
        snr: f32::NAN,
        ms2_n: 0,
        feature: u32::MAX,
    }
}

/// Searches every hypothesis in one file: the cells it finds (detected first, else the highest targeted peak) and the table of targeted peaks.
fn search_file<R: ReadAt>(
    index: &mut Index<R>,
    f: &FileFeatures,
    reg: &Registry,
    p: &TargetParams,
) -> Result<(Vec<Seen>, FilePeaks)> {
    let mut fp = FilePeaks {
        peaks: FileFeatures {
            dt: f.dt,
            ..Default::default()
        },
        ..Default::default()
    };
    let Some(sec) = Section::open(index, p.polarity)? else {
        return Ok((Vec::new(), fp));
    };
    let run = sec.run();
    let mut seen = Vec::new();
    for h in reg.hyps.iter().filter(|h| h.polarity == p.polarity) {
        let found = peaks(index, &sec, h.mz, window(h, p, run), p)?;
        let hit = found.iter().find_map(|c| detected_row(f, h.mz, c, p));
        if let Some(i) = hit {
            seen.push(Seen {
                hyp: h.id,
                cell: cell_of_feature(f, i),
            });
        } else if let Some(c) = found.first() {
            push_peak(&mut fp.peaks, h.mz, c);
            fp.hyp.push(h.id);
            seen.push(Seen {
                hyp: h.id,
                cell: cell_of_peak(c, fp.hyp.len() - 1),
            });
        }
    }
    let tp = Params {
        polarity: Some(p.polarity),
        ..Default::default()
    };
    fp.links = ms2::link(index, &mut fp.peaks, &tp, &p.ms2)?;
    for s in seen.iter_mut().filter(|s| s.cell.state == CELL_TARGETED) {
        s.cell.ms2_n = fp.peaks.ms2_n[s.cell.feature as usize];
    }
    Ok((seen, fp))
}

fn median(v: &mut [f32]) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2] as f64,
        n => (v[n / 2 - 1] as f64 + v[n / 2] as f64) / 2.0,
    }
}

/// Searches all the hypotheses of `reg` in every file and fills the outcome of each. `feats[i]` is the detection table of `index[i]`.
pub fn run<R: ReadAt>(
    index: &mut [Index<R>],
    feats: &[&FileFeatures],
    reg: &mut Registry,
    p: &TargetParams,
) -> Result<TargetResult> {
    let n = index.len().min(feats.len());
    let mut per: Vec<Vec<Cell>> = vec![vec![absent(); n]; reg.hyps.len()];
    let (mut files, mut rate) = (Vec::with_capacity(n), Vec::with_capacity(n));
    let masses: Vec<f64> = reg
        .hyps
        .iter()
        .filter(|h| h.polarity == p.polarity)
        .map(|h| h.mz)
        .collect();
    for k in 0..n {
        let (seen, fp) = search_file(&mut index[k], feats[k], reg, p)?;
        for s in seen {
            per[s.hyp as usize][k] = s.cell;
        }
        rate.push(match Section::open(&mut index[k], p.polarity)? {
            Some(sec) => null::decoy_rate(&mut index[k], &sec, &masses, p)?,
            None => 0.0,
        });
        files.push(fp);
    }
    let mut res = TargetResult {
        n_files: n,
        rows: Vec::new(),
        files,
        tried: masses.len() as u32,
        found: 0,
        decoy_rate: rate,
    };
    assemble(&mut res, per, reg, p, index)?;
    Ok(res)
}

fn assemble<R: ReadAt>(
    res: &mut TargetResult,
    per: Vec<Vec<Cell>>,
    reg: &mut Registry,
    p: &TargetParams,
    index: &mut [Index<R>],
) -> Result<()> {
    let run = match index.first_mut() {
        Some(ix) => Section::open(ix, p.polarity)?
            .map(|s| s.run())
            .unwrap_or((0.0, 0.0)),
        None => (0.0, 0.0),
    };
    for (h, cells) in reg.hyps.iter().zip(per) {
        let hits = cells.iter().filter(|c| c.state != CELL_ABSENT).count();
        if h.polarity != p.polarity {
            continue;
        }
        reg.outcomes[h.id as usize] = if hits == 0 {
            Outcome::NotFound
        } else {
            Outcome::Found(hits as u32)
        };
        if hits == 0 {
            continue;
        }
        let w = window(h, p, run);
        let mut rts: Vec<f32> = cells
            .iter()
            .filter(|c| c.state != CELL_ABSENT)
            .map(|c| c.apex_rt)
            .collect();
        let n_det = cells.iter().filter(|c| c.state == CELL_DETECTED).count() as u16;
        let n_tar = cells.iter().filter(|c| c.state == CELL_TARGETED).count() as u16;
        let coincidence = if p.decoys == 0 {
            1.0
        } else {
            null::coincidence(&res.decoy_rate, w.1 - w.0, hits)
        };
        res.rows.push(TargetRow {
            hyp: h.id,
            mz: h.mz,
            rt: median(&mut rts),
            window: (w.0 as f32, w.1 as f32),
            ms2_total: cells.iter().map(|c| c.ms2_n as u32).sum(),
            n_detected: n_det,
            n_targeted: n_tar,
            coincidence,
            cells,
        });
    }
    res.found = res.rows.len() as u32;
    Ok(())
}

impl TargetResult {
    /// The proof of `row`: the hypothesis that predicted it, how many were tried and found, the chance of a coincidence and the level cap.
    pub fn evidence<'a>(&self, reg: &'a Registry, row: usize) -> Evidence<'a> {
        let r = &self.rows[row];
        Evidence {
            hypothesis: &reg.hyps[r.hyp as usize],
            tried: self.tried,
            found: self.found,
            coincidence: r.coincidence,
            level_cap: r.level_cap(),
        }
    }

    /// The rows as an aligned table (one row per hypothesis found, cells in the order of the files) to put next to the detection.
    pub fn to_aligned(&self) -> Aligned {
        let n = self.n_files;
        let mut a = Aligned {
            n_files: n,
            ..Default::default()
        };
        for r in &self.rows {
            a.mz.push(r.mz);
            a.rt.push(r.rt);
            for c in &r.cells {
                a.area.push(c.area);
                a.height.push(c.height);
                a.apex_rt.push(c.apex_rt);
                a.width.push(c.width);
                a.state.push(c.state);
            }
        }
        a
    }

    /// Bytes of the rows in a fixed layout (the base of the SHA-256 that compares two runs).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&(self.rows.len() as u64).to_le_bytes());
        for r in &self.rows {
            b.extend_from_slice(&r.hyp.to_le_bytes());
            b.extend_from_slice(&r.mz.to_le_bytes());
            b.extend_from_slice(&r.rt.to_le_bytes());
            b.extend_from_slice(&r.coincidence.to_le_bytes());
            b.extend_from_slice(&r.ms2_total.to_le_bytes());
            for c in &r.cells {
                b.push(c.state);
                b.extend_from_slice(&c.area.to_le_bytes());
                b.extend_from_slice(&c.height.to_le_bytes());
                b.extend_from_slice(&c.apex_rt.to_le_bytes());
                b.extend_from_slice(&c.ms2_n.to_le_bytes());
            }
        }
        b
    }
}
