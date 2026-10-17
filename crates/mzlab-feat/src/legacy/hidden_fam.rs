//! Products hidden under an in-source fragment of the parent when the pipeline runs in family mode: the fragment is then a member of the family of the
//! parent, not a row of the table, so its areas are read from the XIC of every file at the RT of the parent and tested with the ratio of `hidden::rises`.

use crate::sortc::SortC;
use mzlab_core::differenza::area;
use mzlab_core::idxread::{Index, ReadAt};

use super::dig::{candidate, Cand};
use super::hidden::{fragment_formula, rises, Inputs};
use super::kinetics::{descriptors, not_in_reference};
use super::series::FileIn;
use super::series::Ident;
use super::Res;
use crate::table::Aligned;

#[cfg(test)]
mod golden;

/// Half window around the RT of the parent (min) and tolerance of the fragment mass (ppm).
const RT_WINDOW: f64 = 0.5;
const MASS_PPM: f64 = 5.0;
/// Parent area (counts x s), height of an own peak, shift of its apex (scans) and number of treated files that must pass.
const MIN_PARENT_AREA: f64 = 1e7;
const MIN_HEIGHT: f64 = 5e4;
const SHIFT_SCANS: f64 = 10.0;
const MIN_SAMPLES: usize = 2;

/// What one file says of one mass in the window of the parent: area (counts x s), height and apex (scan of the window, 1-2-1 smoothed).
struct Trace {
    area: f64,
    height: f64,
    apex: usize,
}

fn measure<R: ReadAt>(ix: &mut Index<R>, mz: f64, rt: f64) -> Res<Trace> {
    let e = |e: mzlab_core::error::Error| e.to_string();
    let Some(si) = ix.section_of(1, None) else {
        return Ok(Trace {
            area: 0.0,
            height: 0.0,
            apex: 0,
        });
    };
    let t = ix.arrays(si).map_err(e)?.rt.clone();
    let d = mz * MASS_PPM * 1e-6;
    let (a, b) = (
        ix.search(si, mz - d, false).map_err(e)?,
        ix.search(si, mz + d, true).map_err(e)?,
    );
    let y = ix.sum_window(si, a, b).map_err(e)?;
    let (x, y): (Vec<f64>, Vec<f64>) = t
        .into_iter()
        .zip(y)
        .filter(|p| (p.0 - rt).abs() <= RT_WINDOW)
        .unzip();
    let sm = |i: usize| {
        let at = |j: Option<usize>| j.and_then(|j| y.get(j)).copied().unwrap_or(0.0);
        (at(i.checked_sub(1)) + 2.0 * y[i] + at(Some(i + 1))) / 4.0
    };
    let apex = (0..y.len())
        .fold((0, 0.0), |m, i| if sm(i) > m.1 { (i, sm(i)) } else { m })
        .0;
    let seconds: Vec<f64> = x.iter().map(|v| v * 60.0).collect();
    Ok(Trace {
        area: area(&seconds, &y),
        height: y.iter().copied().fold(0.0, f64::max),
        apex,
    })
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sortc_by(f64::total_cmp);
    v.get(v.len() / 2).copied().unwrap_or(0.0)
}

/// Is the mass a product? Its ratio to the parent rises with the treatment, or it has a peak of its own (`MIN_HEIGHT`) whose apex sits `SHIFT_SCANS`
/// scans or more away from that of the parent than in the references, in at least two treated files.
fn is_hidden(files: &[FileIn], parent: &[Trace], ion: &[Trace]) -> bool {
    let (pa, ia): (Vec<f64>, Vec<f64>) = parent
        .iter()
        .zip(ion)
        .map(|(p, i)| (p.area, i.area))
        .unzip();
    if rises(files, &pa, &ia) {
        return true;
    }
    let shift = |k: usize| ion[k].apex as f64 - parent[k].apex as f64;
    let refs: Vec<f64> = (0..files.len())
        .filter(|&k| files[k].is_ref())
        .map(shift)
        .collect();
    if refs.is_empty() {
        return false;
    }
    let m0 = median(refs);
    (0..files.len())
        .filter(|&k| files[k].is_treated() && parent[k].area >= MIN_PARENT_AREA)
        .filter(|&k| ion[k].height >= MIN_HEIGHT && (shift(k) - m0).abs() >= SHIFT_SCANS)
        .count()
        >= MIN_SAMPLES
}

/// Both kinds of hidden products: rows of the table (`hidden::add`) and fragments without a row (`add`); `at` is the row of the parent and the m/z of its fragments.
pub(super) fn add_all<R: ReadAt>(
    t: &Aligned,
    s: &Inputs,
    ix: &mut [Index<R>],
    at: (usize, &[f64]),
    w: &[f64; 10],
    cands: &mut Vec<Cand>,
) -> Res<()> {
    super::hidden::add(t, s, at.0, at.1, w, cands);
    add(t, s, ix, at.0, at.1, w, cands)
}

fn traces<R: ReadAt>(ix: &mut [Index<R>], mz: f64, rt: f64) -> Res<Vec<Trace>> {
    ix.iter_mut().map(|x| measure(x, mz, rt)).collect()
}

/// Does the table have a row of its own for the mass `f` at the RT of the parent?
fn has_row(t: &Aligned, prow: usize, f: f64) -> bool {
    let rt = t.rt[prow];
    (0..t.len()).any(|r| {
        r != prow
            && (t.rt[r] - rt).abs() <= RT_WINDOW
            && ((t.mz[r] - f) / f).abs() * 1e6 <= MASS_PPM
    })
}

/// The candidate of a fragment: the row of the parent with the m/z, the areas and the kinetics of the fragment.
fn product(
    t: &Aligned,
    s: &Inputs,
    w: &[f64; 10],
    at: (usize, f64, String),
    ion: (Vec<f64>, i64),
) -> Cand {
    let (prow, f, formula) = at;
    let id = Ident {
        row: prow,
        hypothesis: None,
        formula: Some(formula),
        total: 0.0,
        proofs: Vec::new(),
    };
    let mut c = candidate(t, s.files, s.parent, id, s.hyps, w);
    let times: Vec<f64> = s.files.iter().map(|x| x.t).collect();
    let pick = |sel: fn(&FileIn) -> bool| -> Vec<f64> {
        s.files
            .iter()
            .zip(&ion.0)
            .filter(|p| sel(p.0))
            .map(|p| *p.1)
            .collect()
    };
    let (treated, refs) = (pick(FileIn::is_treated), pick(FileIn::is_ref));
    let ref_max = refs.iter().copied().fold(0.0, f64::max);
    c.kin = descriptors(&times, &ion.0).map(|d| {
        (
            d,
            !treated.is_empty() && not_in_reference(&treated, ref_max),
        )
    });
    c.area_max = treated.iter().copied().fold(0.0, f64::max);
    c.areas = ion.0;
    c.mz = f;
    c.filled = Vec::new();
    c.hidden = Some(ion.1);
    c.flags.push("possible TP co-eluting with an ISF");
    c
}

/// Appends the fragments of the MS2 of the parent (`fragments`) that have no row of their own, are a sub-formula of the parent and whose ratio to the
/// parent rises with the treatment. The candidate stands on the row of the parent, with the m/z of the fragment and its own areas.
pub(super) fn add<R: ReadAt>(
    t: &Aligned,
    s: &Inputs,
    ix: &mut [Index<R>],
    prow: usize,
    fragments: &[f64],
    w: &[f64; 10],
    cands: &mut Vec<Cand>,
) -> Res<()> {
    let rt = t.rt[prow];
    let par = traces(ix, s.parent.mz, rt)?;
    let has_row = |f: f64| has_row(t, prow, f);
    let mut next = cands.iter().filter_map(|c| c.hidden).min().unwrap_or(0);
    let mut frags: Vec<f64> = fragments.to_vec();
    frags.sortc_by(f64::total_cmp);
    for f in frags {
        let Some(formula) = fragment_formula(s.parent, f) else {
            continue;
        };
        if has_row(f)
            || cands
                .iter()
                .any(|c| ((c.mz - f) / f).abs() * 1e6 <= MASS_PPM)
        {
            continue;
        }
        let tr = traces(ix, f, rt)?;
        if !is_hidden(s.files, &par, &tr) {
            continue;
        }
        let ion: Vec<f64> = tr.iter().map(|x| x.area).collect();
        next -= 1;
        let c = product(t, s, w, (prow, f, formula), (ion, next));
        cands.push(c);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(t: f64) -> FileIn {
        FileIn {
            name: String::new(),
            label: String::new(),
            kind: "sample".into(),
            time: Some(t),
            t,
            cond: String::new(),
            replica: None,
            order: 0,
        }
    }

    fn tr(area: f64, height: f64, apex: usize) -> Trace {
        Trace { area, height, apex }
    }

    #[test]
    fn an_ion_with_its_own_peak_away_from_the_parent_is_hidden() {
        let files = [file(-1.0), file(0.0), file(5.0), file(10.0)];
        let parent: Vec<Trace> = (0..4).map(|_| tr(5e8, 1e7, 20)).collect();
        let ion = vec![
            tr(2e5, 6e4, 20),
            tr(2e5, 6e4, 20),
            tr(2e5, 8e4, 45),
            tr(2e5, 9e4, 50),
        ];
        assert!(is_hidden(&files, &parent, &ion));
    }

    #[test]
    fn a_fragment_that_follows_the_parent_is_not_hidden() {
        let files = [file(-1.0), file(0.0), file(5.0), file(10.0)];
        let parent: Vec<Trace> = (0..4).map(|_| tr(5e8, 1e7, 20)).collect();
        let ion: Vec<Trace> = (0..4).map(|_| tr(2e5, 6e4, 22)).collect();
        assert!(!is_hidden(&files, &parent, &ion));
    }
}
