//! Products hidden under an in-source fragment of the parent (`hr/filters.py::isf_hidden`): a fragment mass whose ratio to the parent, in the peak of
//! the parent, rises with the treatment above the ratio of the references. The ratio is that of the areas of the table (the apex shift against the parent is
//! not tested). A mass that passes becomes a candidate with a negative id, "possible TP co-eluting with an ISF".

use mzlab_core::elements::{fmt, mass, Formula, ELECTRON};

use super::dig::{areas, candidate, Cand};
use std::collections::HashMap;

use super::series::{FileIn, Hyp, Ident, Parent};
use crate::table::Aligned;

/// Parent area under which a file says nothing about the ratio.
const MIN_PARENT_AREA: f64 = 1e7;
/// Relative error of the ratio, area floor (counts), and the z the treated ratio must exceed.
const REL_SE: f64 = 0.05;
const FLOOR_AREA: f64 = 3e5;
const Z_MIN: f64 = 3.0;
/// Smallest area of the ion in a treated file, and number of treated files that must pass.
const MIN_ION_AREA: f64 = 2e5;
const MIN_SAMPLES: usize = 2;
/// Tolerance of the formula of the fragment (ppm).
const FORMULA_PPM: f64 = 3.0;

/// Does the ratio `ion / parent` rise with the treatment? `ion` and `parent` are the areas of the row in every file.
pub(super) fn rises(files: &[FileIn], parent: &[f64], ion: &[f64]) -> bool {
    let ok: Vec<usize> = (0..files.len().min(parent.len()))
        .filter(|&k| parent[k] >= MIN_PARENT_AREA)
        .collect();
    let r = |k: usize| ion[k] / parent[k];
    let se = |k: usize| (REL_SE * r(k)).hypot(FLOOR_AREA / parent[k]);
    let refs: Vec<usize> = ok.iter().copied().filter(|&k| files[k].is_ref()).collect();
    if refs.is_empty() {
        return false;
    }
    let w: Vec<f64> = refs.iter().map(|&k| 1.0 / se(k).powi(2)).collect();
    let sw: f64 = w.iter().sum();
    let r0 = refs.iter().zip(&w).map(|(&k, w)| w * r(k)).sum::<f64>() / sw;
    let se0 = (1.0 / sw).sqrt();
    let up = ok
        .iter()
        .copied()
        .filter(|&k| files[k].is_treated() && ion[k] >= MIN_ION_AREA)
        .filter(|&k| (r(k) - r0) / se(k).hypot(se0) > Z_MIN)
        .count();
    up >= MIN_SAMPLES
}

/// The neutral formula of the fragment ion of m/z `mz` among the sub-formulas of the ion of the parent (smallest error within 3 ppm).
pub(super) fn fragment_formula(parent: &Parent, mz: f64) -> Option<String> {
    let ion: &Formula = &parent.ion;
    let mut best: Option<(f64, Formula)> = None;
    let mut cur: Formula = ion.iter().map(|(e, _)| (e.clone(), 0)).collect();
    fn walk(
        i: usize,
        ion: &Formula,
        cur: &mut Formula,
        mz: f64,
        best: &mut Option<(f64, Formula)>,
    ) {
        if i == ion.len() {
            let m = mass(cur) - ELECTRON;
            let err = ((m - mz) / mz * 1e6).abs();
            if cur.iter().any(|x| x.1 > 0)
                && err <= FORMULA_PPM
                && best.as_ref().is_none_or(|b| err < b.0)
            {
                *best = Some((err, cur.clone()));
            }
            return;
        }
        for n in 0..=ion[i].1 {
            cur[i].1 = n;
            walk(i + 1, ion, cur, mz, best);
        }
    }
    walk(0, ion, &mut cur, mz, &mut best);
    let mut f = best?.1;
    if parent.adduct == "[M+H]+" {
        let h = f.iter_mut().find(|x| x.0 == "H").filter(|x| x.1 > 0)?;
        h.1 -= 1;
    }
    f.retain(|x| x.1 > 0);
    Some(fmt(&f))
}

/// Window around the RT of the parent (min) and tolerance of the fragment mass (ppm).
const RT_WINDOW: f64 = 0.5;
const MASS_PPM: f64 = 5.0;

/// The rows of the table, by m/z, that are hidden products (see `add`).
fn hidden_rows(
    t: &Aligned,
    files: &[FileIn],
    prow: usize,
    fragments: &[f64],
    cands: &[Cand],
) -> Vec<usize> {
    let pa = areas(t, prow);
    let mut rows: Vec<usize> = (0..t.len())
        .filter(|&r| r != prow && (t.rt[r] - t.rt[prow]).abs() <= RT_WINDOW)
        .filter(|&r| !cands.iter().any(|c| c.row == r))
        .filter(|&r| {
            fragments
                .iter()
                .any(|f| ((t.mz[r] - f) / f).abs() * 1e6 <= MASS_PPM)
        })
        .filter(|&r| rises(files, &pa, &areas(t, r)))
        .collect();
    rows.sort_by(|&a, &b| t.mz[a].total_cmp(&t.mz[b]));
    rows
}

/// What the hidden products are read against: the files, the parent and the hypotheses of the Dig.
pub(super) struct Inputs<'a> {
    pub files: &'a [FileIn],
    pub parent: &'a Parent,
    pub hyps: &'a HashMap<u32, Hyp>,
}

impl<'a> Inputs<'a> {
    pub fn new(files: &'a [FileIn], parent: &'a Parent, hyps: &'a HashMap<u32, Hyp>) -> Self {
        Inputs {
            files,
            parent,
            hyps,
        }
    }
}

/// Appends the hidden products: rows of the table that sit at the RT of the parent, at the mass of a fragment of its MS2 (`fragments`), that are not
/// candidates already and whose ratio to the parent rises with the treatment.
pub(super) fn add(
    t: &Aligned,
    s: &Inputs,
    prow: usize,
    fragments: &[f64],
    w: &[f64; 10],
    cands: &mut Vec<Cand>,
) {
    let (files, parent, hyps) = (s.files, s.parent, s.hyps);
    let rows = hidden_rows(t, files, prow, fragments, cands);
    for (j, r) in rows.into_iter().enumerate() {
        let formula = fragment_formula(parent, t.mz[r]);
        let id = Ident {
            row: r,
            hypothesis: None,
            formula,
            total: 0.0,
            proofs: Vec::new(),
        };
        let mut c = candidate(t, files, parent, id, hyps, w);
        c.hidden = Some(-(j as i64 + 1));
        c.flags.push("possible TP co-eluting with an ISF");
        cands.push(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(t: f64, kind: &str) -> FileIn {
        FileIn {
            name: String::new(),
            label: String::new(),
            kind: kind.into(),
            time: Some(t),
            t,
            cond: String::new(),
            replica: None,
            order: 0,
        }
    }

    fn series() -> Vec<FileIn> {
        vec![
            file(-1.0, "sample"),
            file(0.0, "sample"),
            file(5.0, "sample"),
            file(10.0, "sample"),
        ]
    }

    #[test]
    fn a_ratio_that_rises_with_the_treatment_is_a_hidden_product() {
        let parent = [6e7, 6e7, 5e7, 4e7];
        let ion = [1.2e6, 1.2e6, 3.5e6, 4.5e6];
        assert!(rises(&series(), &parent, &ion));
    }

    #[test]
    fn a_constant_ratio_is_a_plain_fragment() {
        let parent = [6e7, 6e7, 5e7, 4e7];
        let ion = [1.2e6, 1.2e6, 1.0e6, 0.8e6];
        assert!(!rises(&series(), &parent, &ion));
    }

    #[test]
    fn the_fragment_formula_is_a_sub_formula_of_the_ion_in_neutral_convention() {
        let parent =
            crate::legacy::series::parent(r#"{"neutral":"C8H10N4O2","adduct":"[M+H]+"}"#).unwrap();
        assert_eq!(
            fragment_formula(&parent, 138.0662).as_deref(),
            Some("C6H7N3O")
        );
        assert_eq!(fragment_formula(&parent, 500.0), None);
    }
}
