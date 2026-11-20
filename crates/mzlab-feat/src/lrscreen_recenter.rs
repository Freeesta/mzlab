//! `recenter` of `features.align` at unit resolution: each file's m/z drift against the series, measured on anchor rows and subtracted from the
//! file's features before the grouping. The channel defect keeps each file's own grid right, but the alignment chains on m/z gaps below the
//! half window, so files that drift in opposite directions can fall into separate rows.

use mzlab_core::error::Result;

use crate::align::align_grouped;
use crate::lrscreen::{choose, Shift, Switch};
use crate::recipe::Recipe;
use crate::table::FileFeatures;

/// A shift smaller than this is measured and reported but not subtracted.
pub const MIN_SHIFT_DA: f64 = 0.05;
/// Anchors needed to trust a shift.
pub const MIN_ANCHORS: usize = 5;

fn median(v: &mut [f64]) -> f64 {
    mzlab_core::sortc::by(v, f64::total_cmp);
    match v.len() {
        0 => 0.0,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// The groups of the preliminary alignment that every file sees at most once, seen by at least half of the files and in the top half by height: the
/// anchors, each as its (file, feature) members.
fn anchors(files: &[&FileFeatures], r: &Recipe) -> Result<Vec<Vec<(usize, usize)>>> {
    let g = align_grouped(files, &r.align.ion, None, true)?;
    let mut members: Vec<Vec<(usize, usize)>> = vec![Vec::new(); g.table.len()];
    for (k, of) in g.group_of.iter().enumerate() {
        for (i, &id) in of.iter().enumerate() {
            if let Some(m) = members.get_mut(id as usize) {
                m.push((k, i));
            }
        }
    }
    let n = files.len();
    let mut cand: Vec<(f64, Vec<(usize, usize)>)> = members
        .into_iter()
        .filter(|m| {
            let mut seen: Vec<usize> = m.iter().map(|x| x.0).collect();
            seen.dedup();
            seen.len() == m.len() && 2 * m.len() >= n
        })
        .map(|m| {
            let mut h: Vec<f64> = m.iter().map(|&(k, i)| f64::from(files[k].height[i])).collect();
            (median(&mut h), m)
        })
        .collect();
    let mut hs: Vec<f64> = cand.iter().map(|c| c.0).collect();
    let cut = median(&mut hs);
    cand.retain(|c| c.0 >= cut);
    Ok(cand.into_iter().map(|c| c.1).collect())
}

/// The shift of every file (`mz - row median`, median over the anchors that hold the file).
pub fn shifts(files: &[&FileFeatures], r: &Recipe) -> Result<Vec<Shift>> {
    let none = |why: &str| {
        choose(r, "recenter", "off", why);
        vec![Shift { mz_da: 0.0, anchors: 0, applied: false }; files.len()]
    };
    if files.len() < 2 {
        return Ok(none("fewer than 2 files"));
    }
    let rows = anchors(files, r)?;
    if rows.len() < MIN_ANCHORS {
        let why = format!("{} anchor rows, at least {MIN_ANCHORS} needed", rows.len());
        return Ok(none(&why));
    }
    let mut per_file: Vec<Vec<f64>> = vec![Vec::new(); files.len()];
    for m in &rows {
        let mut mz: Vec<f64> = m.iter().map(|&(k, i)| files[k].mz[i]).collect();
        let mid = median(&mut mz);
        for &(k, i) in m {
            per_file[k].push(files[k].mz[i] - mid);
        }
    }
    Ok(per_file
        .into_iter()
        .map(|mut d| {
            let anchors = d.len();
            let mz_da = if anchors == 0 { 0.0 } else { median(&mut d) };
            Shift { mz_da, anchors, applied: mz_da.abs() >= MIN_SHIFT_DA }
        })
        .collect())
}

/// The files with each shift of the series subtracted from their m/z, when the recipe recentres (unit axis only) and some file is shifted (`None`
/// otherwise: the caller keeps the originals, so the default path copies nothing); the measure goes to the evidence.
pub(crate) fn recentred(files: &[&FileFeatures], r: &Recipe) -> Result<Option<Vec<FileFeatures>>> {
    if !r.is_lr() || r.screen.recenter == Switch::Off {
        return Ok(None);
    }
    let found = shifts(files, r)?;
    let n = found.iter().filter(|s| s.applied).count();
    if found.iter().any(|s| s.anchors > 0) {
        choose(
            r,
            "recenter",
            if n > 0 { "on" } else { "off" },
            format!("{n} of {} files shifted by at least {MIN_SHIFT_DA} Da", found.len()),
        );
    }
    if let Some(e) = &r.screen.evidence {
        e.with(|l| l.shifts = found.clone());
    }
    if n == 0 {
        return Ok(None);
    }
    Ok(Some(
        files
            .iter()
            .zip(&found)
            .map(|(f, s)| {
                let mut f = (*f).clone();
                if s.applied {
                    f.mz.iter_mut().for_each(|m| *m -= s.mz_da);
                }
                f
            })
            .collect(),
    ))
}
