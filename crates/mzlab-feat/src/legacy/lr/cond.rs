//! Conditions in the Dig: a series that compares several experimental conditions (`serie` of `campioni/2`) is judged condition by condition, never as
//! one kinetic. The treated samples of a condition, its time zero and its blanks (or the shared ones, with an empty condition, when it has none of
//! its own) make the view `members`; a row takes the criteria of the condition where it scores best, and the detail per condition is in the block
//! `gruppi` of the summary (`gruppi_out.rs`). A series with a single condition (or none) goes through the code of one kinetic unchanged.

use super::super::series::FileIn;
use super::kin::{decay, gen_of, kinetics, Decay, Gens, Point};
use super::labels::{score_in, score_one, Cell, Persist, Score};

/// The distinct non-empty conditions of the samples, in the order of the files.
pub fn conditions(files: &[FileIn]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for f in files
        .iter()
        .filter(|f| f.kind == "sample" && !f.cond.is_empty())
    {
        if !out.contains(&f.cond) {
            out.push(f.cond.clone());
        }
    }
    out
}

/// How many distinct conditions the `(type, condition)` pairs of the samples hold.
pub fn count<'a>(files: impl Iterator<Item = (&'a str, &'a str)>) -> usize {
    let mut seen: Vec<&str> = Vec::new();
    for (t, c) in files {
        if t == "sample" && !c.is_empty() && !seen.contains(&c) {
            seen.push(c);
        }
    }
    seen.len()
}

fn is_t0(f: &FileIn) -> bool {
    f.kind == "sample" && f.time == Some(0.0)
}

fn is_blank(f: &FileIn) -> bool {
    f.kind == "blank" || f.kind == "control"
}

/// The columns that judge condition `cond`: its treated samples, and per kind of control its own files if it has any, else the shared ones.
pub fn members(files: &[FileIn], cond: &str) -> Vec<usize> {
    let of =
        |k: &usize, sel: &dyn Fn(&FileIn) -> bool, c: &str| sel(&files[*k]) && files[*k].cond == c;
    let all: Vec<usize> = (0..files.len()).collect();
    let mut out: Vec<usize> = all
        .iter()
        .filter(|k| files[**k].kind == "sample" && !is_t0(&files[**k]) && files[**k].cond == cond)
        .copied()
        .collect();
    for sel in [&is_t0 as &dyn Fn(&FileIn) -> bool, &is_blank] {
        let own: Vec<usize> = all.iter().filter(|k| of(k, sel, cond)).copied().collect();
        out.extend(if own.is_empty() {
            all.iter().filter(|k| of(k, sel, "")).copied().collect()
        } else {
            own
        });
    }
    out.sort_unstable();
    out
}

/// The files and cells of the members of `cond`.
pub fn view(files: &[FileIn], cells: &[Cell], cond: &str) -> (Vec<FileIn>, Vec<Cell>) {
    let m = members(files, cond);
    (
        m.iter().map(|&k| files[k].clone()).collect(),
        m.iter().map(|&k| cells[k].clone()).collect(),
    )
}

/// The score of a row and the condition it comes from (`""` for a series of one condition): the best score wins, the first condition on a tie.
pub fn best(files: &[FileIn], cells: &[Cell], p: Persist) -> (Score, String) {
    let conds = conditions(files);
    if conds.len() < 2 {
        return (score_one(files, cells, p), String::new());
    }
    let mut top: Option<(Score, String)> = None;
    for c in conds {
        let (f, x) = view(files, cells, &c);
        let s = score_in(&f, &x, p, 3);
        let better = top
            .as_ref()
            .is_none_or(|(t, _)| s.score.unwrap_or(-1.0) > t.score.unwrap_or(-1.0));
        if better {
            top = Some((s, c));
        }
    }
    top.unwrap_or_else(|| (score_one(files, cells, p), String::new()))
}

/// `kinetics` of the samples of one condition (all the samples when `cond` is empty).
pub fn kinetics_in(files: &[FileIn], cells: &[Cell], cond: &str) -> Vec<Point> {
    if cond.is_empty() {
        return kinetics(files, cells);
    }
    let (f, x) = view(files, cells, cond);
    kinetics(&f, &x)
}

/// The models of the kinetic generation of a product: one per condition when the series has several, else a single one.
pub struct GensBy {
    all: Gens,
    by: Vec<(String, Gens)>,
    /// First-order decay of the parent in the reference condition (the first one without a reference), `None` without 3 points.
    pub decay: Option<Decay>,
}

impl GensBy {
    /// From the cells of the parent row (`None` when it has none): the times and the decay of the parent in each condition.
    pub fn new(files: &[FileIn], parent: Option<&[Cell]>, reference: Option<&str>) -> GensBy {
        let conds = conditions(files);
        let kin = |c: &str| parent.map_or(Vec::new(), |x| kinetics_in(files, x, c));
        if conds.len() < 2 {
            let k = kin("");
            let d = decay(&k);
            return GensBy {
                all: Gens::new(&k, d.map(|x| x.k)),
                by: Vec::new(),
                decay: d,
            };
        }
        let by: Vec<(String, Gens)> = conds
            .iter()
            .map(|c| {
                let k = kin(c);
                (c.clone(), Gens::new(&k, decay(&k).map(|x| x.k)))
            })
            .collect();
        let first = reference
            .filter(|r| conds.iter().any(|c| c == r))
            .unwrap_or(&conds[0]);
        let k = kin(first);
        let d = decay(&k);
        GensBy {
            all: Gens::new(&k, d.map(|x| x.k)),
            by,
            decay: d,
        }
    }

    /// First or second generation of the product in the cells `cells`, judged in its best condition `cond`.
    pub fn generation(
        &self,
        files: &[FileIn],
        cells: &[Cell],
        cond: &str,
    ) -> Option<(&'static str, String)> {
        let g = self
            .by
            .iter()
            .find(|b| b.0 == cond)
            .map_or(&self.all, |b| &b.1);
        gen_of(g, &kinetics_in(files, cells, cond))
    }
}
