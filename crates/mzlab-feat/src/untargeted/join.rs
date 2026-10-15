//! Join aligner: one row per ion across the files, rewritten from scratch after the algorithm of mzmine 3.9.0
//! `modules/dataprocessing/align_join/JoinAlignerTask.java`, `RowVsRowScore.java` and `util/FeatureListUtils.java::getAlignmentScore` (MIT, (c) 2004-2022
//! The mzmine Development Team); no code copied. Paper: MZmine 3, Schmid et al., Nat. Biotechnol. 41:447 (2023), doi:10.1038/s41587-023-01690-2.
//!
//! What 3.9.0 does (on master the same loop lives in `BaseFeatureListAligner`; the result is the same):
//! 1. The lists wait with the features that are not in a row yet. Each round the list with the most waiting features is the base (a stable sort, so
//!    on a tie the earlier list wins); every waiting feature of the base becomes a new row, kept sorted by m/z (+ RT / 1e7).
//! 2. Each waiting feature of the other lists is scored against the base rows inside its tolerances (closed ranges: m/z +- max(Da, ppm), RT +- tolerance).
//!    The score of a pair is `(max(0, (1 - |dmz| / h_mz) * w_mz) + max(0, 1 - |drt| / h_rt * w_rt)) / (round(w_mz) + round(w_rt))` where `h` is the half
//!    width of the range and the base row supplies the tested value (the weight of the RT term sits INSIDE the bracket in mzmine, outside for m/z; with
//!    w_rt = 1, the default, the two forms are equal). RT is a `float` in mzmine: the RT arithmetic here is done in `f32` too.
//! 3. All the scores are sorted from the best; walking them, a feature that is not placed yet goes into the base row if that row has no feature of its
//!    file (one feature per file and row, so a better row that is already taken by the file lets the next best one be tried).
//! 4. Placed features leave the waiting lists; the rounds go on until nothing waits. The rows of all the rounds are then sorted by RT (+ m/z / 1e7) and
//!    their m/z and RT become the mean of their features.
//!
//! Where 3.9.0 is not deterministic (equal scores arrive in the order of a parallel stream) this module breaks the tie by the order of the waiting lists,
//! then the order of the features in the list, then the order of the base rows (m/z ascending): the same input always gives the same rows. Not carried
//! over: the optional checks of charge, identity, isotope pattern, spectra and mobility (all off in the default batch).

use super::Feature;

/// Parameters of the join (defaults of the batch of the gold data: 0.0015 Da or 5 ppm, weights 3 and 1, 0.2 min).
#[derive(Debug, Clone, Copy)]
pub struct JoinParams {
    pub tol_da: f64,
    pub tol_ppm: f64,
    pub w_mz: f64,
    pub w_rt: f64,
    /// Half width of the RT range, minutes.
    pub rt_tol: f64,
}

impl Default for JoinParams {
    fn default() -> Self {
        JoinParams {
            tol_da: 0.0015,
            tol_ppm: 5.0,
            w_mz: 3.0,
            w_rt: 1.0,
            rt_tol: 0.2,
        }
    }
}

/// One row of the aligned list: `cells[file]` is the feature of that file (`None` = empty cell). `mz` is the mean m/z of the features, `rt` the mean RT.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Row {
    pub mz: f64,
    pub rt: f64,
    pub cells: Vec<Option<Feature>>,
}

/// The aligned list, rows in the order of mzmine (RT ascending).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rows {
    pub n_files: usize,
    pub rows: Vec<Row>,
}

impl Row {
    /// Mean m/z and mean RT (as a `float`, like mzmine) of the features present; keeps the values when the row is empty.
    pub fn refresh(&mut self) {
        let f: Vec<&Feature> = self.cells.iter().flatten().collect();
        if f.is_empty() {
            return;
        }
        let n = f.len() as f64;
        self.mz = f.iter().map(|x| x.mz).sum::<f64>() / n;
        self.rt = (f.iter().map(|x| x.rt as f32 as f64).sum::<f64>() / n) as f32 as f64;
    }

    /// The sort key of mzmine for m/z (`FeatureListRowSorter.MZ_ASCENDING`).
    fn key_mz(&self) -> f64 {
        self.mz + (self.rt as f32) as f64 / 1e7
    }

    /// The sort key of mzmine for RT (`FeatureListRowSorter.DEFAULT_RT`).
    fn key_rt(&self) -> f64 {
        (self.rt as f32) as f64 + self.mz / 1e7
    }
}

/// Features of a list that are not in a row yet (indices into the list of `file`).
struct Waiting {
    file: usize,
    feats: Vec<usize>,
}

/// A candidate pair, ordered by the tie-break described in the module comment.
struct Score {
    score: f64,
    wait: usize,
    pos: usize,
    base: usize,
}

/// `Range.closed(rt - tol, rt + tol)` in `float`.
fn rt_range(rt: f64, tol: f64) -> (f32, f32) {
    (rt as f32 - tol as f32, rt as f32 + tol as f32)
}

/// `getAlignmentScore` of mzmine for a base row (`mz`, `rt`) against the ranges built around the feature to add (`x`).
fn pair_score(x: &Feature, mz: f64, rt: f64, mz_tol: f64, p: &JoinParams) -> f64 {
    let (mut score, mut scorers) = (0.0f64, 0.0f64);
    if p.w_mz > 0.0 {
        let half = mz_tol; // (hi - lo) / 2 of the closed range around x.mz
        score += ((1.0 - (mz - x.mz).abs() / half) * p.w_mz).max(0.0);
        scorers += p.w_mz.round();
    }
    if p.w_rt > 0.0 {
        let (lo, hi) = rt_range(x.rt, p.rt_tol);
        let (centre, len) = ((lo + hi) / 2.0, hi - lo);
        let diff = (rt as f32 - centre).abs();
        score += (1.0 - (diff / (len / 2.0)) * p.w_rt as f32).max(0.0) as f64;
        scorers += p.w_rt.round();
    }
    if scorers == 0.0 {
        0.0
    } else {
        score / scorers
    }
}

/// Scores of every waiting feature against the base rows (sorted by m/z) that lie inside its ranges, best first.
fn score_all(
    lists: &[Vec<Feature>],
    waiting: &[Waiting],
    base: &[Row],
    p: &JoinParams,
) -> Vec<Score> {
    let keys: Vec<f64> = base.iter().map(Row::key_mz).collect();
    let mut out = Vec::new();
    for (wait, w) in waiting.iter().enumerate() {
        for (pos, &fi) in w.feats.iter().enumerate() {
            let x = &lists[w.file][fi];
            let mz_tol = p.tol_da.max(x.mz / 1e6 * p.tol_ppm);
            let (mz_lo, mz_hi) = if p.w_mz > 0.0 {
                (x.mz - mz_tol, x.mz + mz_tol)
            } else {
                (f64::MIN, f64::MAX)
            };
            // The sort key adds RT / 1e7 to m/z: widen the search by far more than that and test the exact ranges below.
            let a = keys.partition_point(|&k| k < mz_lo - 1e-3);
            for (b, row) in base.iter().enumerate().skip(a) {
                if keys[b] > mz_hi + 1e-3 {
                    break;
                }
                let (lo, hi) = rt_range(x.rt, p.rt_tol);
                let rt_in = p.w_rt <= 0.0 || ((row.rt as f32) >= lo && (row.rt as f32) <= hi);
                if row.mz >= mz_lo && row.mz <= mz_hi && rt_in {
                    out.push(Score {
                        score: pair_score(x, row.mz, row.rt, mz_tol, p),
                        wait,
                        pos,
                        base: b,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.wait.cmp(&b.wait))
            .then(a.pos.cmp(&b.pos))
            .then(a.base.cmp(&b.base))
    });
    out
}

/// Places the waiting features onto the base rows, best score first, one feature per file and row; placed features leave the waiting lists.
fn place(lists: &[Vec<Feature>], waiting: &mut Vec<Waiting>, base: &mut [Row], p: &JoinParams) {
    let scores = score_all(lists, waiting, base, p);
    let mut placed: Vec<Vec<bool>> = waiting.iter().map(|w| vec![false; w.feats.len()]).collect();
    for s in &scores {
        if placed[s.wait][s.pos] {
            continue;
        }
        let file = waiting[s.wait].file;
        let cell = &mut base[s.base].cells[file];
        if cell.is_none() {
            *cell = Some(lists[file][waiting[s.wait].feats[s.pos]].clone());
            placed[s.wait][s.pos] = true;
        }
    }
    for (w, done) in waiting.iter_mut().zip(&placed) {
        let mut it = done.iter();
        w.feats.retain(|_| !*it.next().unwrap());
    }
    waiting.retain(|w| !w.feats.is_empty());
}

/// Aligns the lists (`lists[k]` = the features of file `k`) into rows. Both weights at zero is an error in mzmine (nothing can be aligned): the
/// result is then empty.
pub fn align(lists: &[Vec<Feature>], p: &JoinParams) -> Rows {
    let n = lists.len();
    let mut out = Rows {
        n_files: n,
        rows: Vec::new(),
    };
    if p.w_mz <= 0.0 && p.w_rt <= 0.0 {
        return out;
    }
    let mut waiting: Vec<Waiting> = (0..n)
        .map(|file| Waiting {
            file,
            feats: (0..lists[file].len()).collect(),
        })
        .collect();
    while !waiting.is_empty() {
        waiting.sort_by_key(|w| std::cmp::Reverse(w.feats.len()));
        let base = waiting.remove(0);
        if base.feats.is_empty() {
            break;
        }
        let mut rows: Vec<Row> = base
            .feats
            .iter()
            .map(|&i| {
                let f = lists[base.file][i].clone();
                let mut cells = vec![None; n];
                let (mz, rt) = (f.mz, f.rt);
                cells[base.file] = Some(f);
                Row { mz, rt, cells }
            })
            .collect();
        rows.sort_by(|a, b| a.key_mz().total_cmp(&b.key_mz()));
        place(lists, &mut waiting, &mut rows, p);
        out.rows.extend(rows);
    }
    for r in &mut out.rows {
        r.refresh();
    }
    out.rows.sort_by(|a, b| a.key_rt().total_cmp(&b.key_rt()));
    out
}
