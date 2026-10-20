//! The row of the table for the page, `riga/1`, and its traces, `riga_tracce/1`: everything the `.mzfeat` knows about one row (summaries, identity, one cell per
//! file) and, read from the `.mzidx` of the series, the XIC of every file, the MS1 at the best apex and the MS2 of the row. Numbers only: no verdict. A summary
//! that was not computed is `null`, never a made-up number.

use crate::sortc::SortC;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;

use crate::aligned::{Aligned, CELL_ABSENT, CELL_DETECTED, CELL_FILLED, CELL_TARGETED, NO_FILE};
use crate::hypo::{SRC_CLEAVAGE, SRC_FAMILY, SRC_IMPORTED, SRC_NETWORK, SRC_RULE, SRC_SUSPECT};
use crate::ms2spec::{pick, read_peaks};
use crate::norm::measured;
use crate::quality::{Q_POINTS, Q_SHAPE, Q_SNR};
use crate::rowident::{adduct_names, SRC_DETECTED};
use crate::samples::Sample;
use crate::table::RowQ;

pub const SCHEMA: &str = "riga/1";
pub const TRACES_SCHEMA: &str = "riga_tracce/1";
/// Most points of the XIC of a file.
pub const MAX_POINTS: usize = 400;
/// Widths of the peak that the XIC shows on each side of its edges.
const PAD_WIDTHS: f64 = 3.0;
/// The MS1 spectrum spans from this far below to this far above the m/z of the row, Da.
const SPEC_BELOW: f64 = 1.0;
const SPEC_ABOVE: f64 = 4.0;

fn not_found(id: u32) -> Error {
    Error::new("error.rowNotFound").with("id", id)
}

fn state_word(state: u8) -> &'static str {
    match state {
        CELL_DETECTED => "detected",
        CELL_FILLED => "filled",
        CELL_TARGETED => "targeted",
        _ => "absent",
    }
}

fn check(t: &Aligned, s: &[Sample], id: u32) -> Result<usize> {
    if s.len() != t.n_files {
        return Err(Error::new("error.badView"));
    }
    t.row_of_id(id).ok_or_else(|| not_found(id))
}

fn fin(v: f32) -> Option<f64> {
    Some(f64::from(v)).filter(|x| x.is_finite())
}

/// The `riga/1` of the row with `id`: `{schema, id, mz, rt, rt_lo, rt_hi, charge, quality, iso_m1, ms2, ident, cells}`. `quality`, `ms2` and `ident` are
/// `null` when the table has no such summary for the row (a targeted row has no quality).
pub fn row(t: &Aligned, s: &[Sample], id: u32) -> Result<String> {
    let r = check(t, s, id)?;
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(SCHEMA)
        .raw(",\"id\":")
        .int(i64::from(id));
    o.raw(",\"mz\":").num(t.mz[r]).raw(",\"rt\":").num(t.rt[r]);
    let q = (!t.meta.is_empty()).then(|| t.meta.get(r));
    o.raw(",\"rt_lo\":")
        .opt(q.and_then(|q| fin(q.rt_lo)))
        .raw(",\"rt_hi\":")
        .opt(q.and_then(|q| fin(q.rt_hi)));
    o.raw(",\"charge\":");
    match q.filter(|q| q.charge != 0) {
        Some(q) => o.int(i64::from(q.charge)),
        None => o.raw("null"),
    };
    write_quality(&mut o, q.as_ref());
    o.raw(",\"iso_m1\":").opt(q.and_then(|q| fin(q.iso_m1)));
    o.raw(",\"ms2\":");
    match q.filter(|q| q.ms2_file != NO_FILE && (q.ms2_file as usize) < t.n_files) {
        Some(q) => o
            .raw("{\"file\":")
            .str(&s[q.ms2_file as usize].file)
            .raw(",\"purity\":")
            .opt(fin(q.ms2_purity).filter(|p| *p >= 0.0))
            .raw("}"),
        None => o.raw("null"),
    };
    write_ident(&mut o, t, r);
    write_family(&mut o, t, r);
    write_cells(&mut o, t, s, r);
    Ok(o.0)
}

/// `"quality":{...}` (`null` when no figure was computed).
fn write_quality(o: &mut Out, q: Option<&RowQ>) {
    o.raw(",\"quality\":");
    match q
        .filter(|q| fin(q.q_shape).is_some() || fin(q.q_snr).is_some() || fin(q.q_fwhm).is_some())
    {
        Some(q) => {
            o.raw("{\"shape\":")
                .opt(fin(q.q_shape))
                .raw(",\"snr\":")
                .opt(fin(q.q_snr))
                .raw(",\"fwhm\":")
                .opt(fin(q.q_fwhm))
                .raw(",\"flags\":[");
            let flags = [(Q_SHAPE, "shape"), (Q_SNR, "snr"), (Q_POINTS, "points")]
                .into_iter()
                .filter(|(b, _)| q.qreason & b != 0);
            for (i, (_, w)) in flags.enumerate() {
                o.raw(if i > 0 { "," } else { "" }).str(w);
            }
            o.raw("]}")
        }
        None => o.raw("null"),
    };
}

/// `"family":{size, members}` (`null` in "per ion" mode): the ions the row is made of, the reference ion first, with the role they have most often.
fn write_family(o: &mut Out, t: &Aligned, r: usize) {
    o.raw(",\"family\":");
    if t.member_start.is_empty() {
        o.raw("null");
        return;
    }
    o.raw("{\"size\":")
        .int(t.family_size(r) as i64)
        .raw(",\"members\":[");
    for (k, m) in t.members(r).enumerate() {
        if k > 0 {
            o.raw(",");
        }
        o.raw("{\"mz\":")
            .num(t.member_mz[m])
            .raw(",\"rt\":")
            .num(f64::from(t.member_rt[m]))
            .raw(",\"role\":")
            .str(crate::famrecord::role_name(t.member_role[m]))
            .raw("}");
    }
    o.raw("]}");
}

/// `"ident":{...}` (`null` when the table has no identity).
fn write_ident(o: &mut Out, t: &Aligned, r: usize) {
    o.raw(",\"ident\":");
    if t.ident.is_empty() {
        o.raw("null");
    } else {
        let adduct = adduct_names()
            .get(usize::from(t.ident.ion[r]).wrapping_sub(1))
            .copied();
        o.raw("{\"neutral\":")
            .num(t.ident.neutral[r])
            .raw(",\"adduct\":");
        match adduct {
            Some(a) => o.str(a),
            None => o.raw("null"),
        };
        o.raw(",\"sources\":[");
        let src = t.ident.src[r];
        let hyp = src >> 1;
        let mut words: Vec<&str> = Vec::new();
        if src & SRC_DETECTED != 0 {
            words.push("detected");
        }
        if hyp != 0 {
            words.push("hypothesis");
        }
        let names = [
            (SRC_RULE, "rule"),
            (SRC_SUSPECT, "suspect"),
            (SRC_FAMILY, "family"),
            (SRC_IMPORTED, "imported"),
            (SRC_CLEAVAGE, "cleavage"),
            (SRC_NETWORK, "network"),
        ];
        words.extend(names.iter().filter(|(b, _)| hyp & b != 0).map(|(_, w)| *w));
        for (i, w) in words.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(w);
        }
        o.raw("]}");
    }
}

/// `"cells":[...]`, one per file.
fn write_cells(o: &mut Out, t: &Aligned, s: &[Sample], r: usize) {
    o.raw(",\"cells\":[");
    for (k, smp) in s.iter().enumerate() {
        let c = t.cell(r, k);
        let a = measured(t, r, k);
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"file\":")
            .str(&smp.file)
            .raw(",\"type\":")
            .str(&smp.kind)
            .raw(",\"time\":")
            .opt(smp.time_min);
        o.raw(",\"area\":")
            .opt(a)
            .raw(",\"height\":")
            .opt(a.map(|_| f64::from(t.height[c])));
        o.raw(",\"apex_rt\":")
            .opt(fin(t.apex_rt[c]).filter(|_| t.state[c] != CELL_ABSENT))
            .raw(",\"width\":")
            .opt(fin(t.width[c]).filter(|_| t.state[c] != CELL_ABSENT));
        o.raw(",\"state\":")
            .str(state_word(if a.is_some() {
                t.state[c]
            } else {
                CELL_ABSENT
            }))
            .raw("}");
    }
    o.raw("]}");
}

/// At most `MAX_POINTS` points: the window is cut in equal groups and each keeps its point of highest intensity (the peak is never flattened).
pub fn decimate(x: &[f64], y: &[f64]) -> (Vec<f64>, Vec<f64>) {
    if x.len() <= MAX_POINTS {
        return (x.to_vec(), y.to_vec());
    }
    let (mut ox, mut oy) = (
        Vec::with_capacity(MAX_POINTS),
        Vec::with_capacity(MAX_POINTS),
    );
    for g in 0..MAX_POINTS {
        let (a, b) = (g * x.len() / MAX_POINTS, (g + 1) * x.len() / MAX_POINTS);
        let m = (a..b).max_by(|&i, &j| y[i].total_cmp(&y[j])).unwrap_or(a);
        ox.push(x[m]);
        oy.push(y[m]);
    }
    (ox, oy)
}

/// A spectrum read from an index: `(rt of the scan or precursor m/z, m/z, intensity)`.
type Peaks = (f64, Vec<f64>, Vec<f64>);

fn section_pol<R: ReadAt>(index: &Index<R>, level: u8) -> Option<(usize, i8)> {
    [1i8, -1]
        .into_iter()
        .find_map(|p| index.section_of(level, Some(p)).map(|si| (si, p)))
}

fn list(o: &mut Out, v: &[f64]) {
    o.floats(v);
}

/// The apex of the cell of file `k` and the window of the XIC: the edges of the peak (±0.1 min around the apex when the table has none) plus `PAD_WIDTHS` widths on each side.
fn window(t: &Aligned, r: usize, k: usize, q: Option<&RowQ>) -> (f64, f64, f64) {
    let c = t.cell(r, k);
    let apex = fin(t.apex_rt[c])
        .filter(|_| t.state[c] != CELL_ABSENT)
        .unwrap_or(t.rt[r]);
    let edges = q.map(|q| (f64::from(q.rt_lo), f64::from(q.rt_hi)));
    let (lo, hi) = match edges {
        Some((lo, hi)) if lo.is_finite() && hi.is_finite() && hi > lo => (lo, hi),
        _ => (apex - 0.1, apex + 0.1),
    };
    let pad = PAD_WIDTHS * (hi - lo);
    (apex, lo - pad, hi + pad)
}

/// The scans of section `si` of one file inside `(from, to)` with the summed intensity within `tol` of `mz`: `(rt of every scan, rt kept, intensity kept)`.
fn xic_of<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
    mz: f64,
    tol: f64,
    (from, to): (f64, f64),
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let (a, b) = (
        index.search(si, mz - tol, false)?,
        index.search(si, mz + tol, true)?,
    );
    let sum = index.sum_window(si, a, b)?;
    let rts = index.arrays(si)?.rt.clone();
    let kept: Vec<usize> = (0..rts.len().min(sum.len()))
        .filter(|&i| rts[i] >= from && rts[i] <= to)
        .collect();
    let (x, y) = kept.iter().map(|&i| (rts[i], sum[i])).unzip();
    Ok((rts, x, y))
}

/// One spectrum of `riga_tracce/1`: `{"file", <extra>, "mz", "y"}`.
fn spectrum(o: &mut Out, file: &str, extra: (&str, f64), mz: &[f64], y: &[f64]) {
    o.raw("{\"file\":")
        .str(file)
        .raw(",\"")
        .raw(extra.0)
        .raw("\":")
        .num(extra.1)
        .raw(",\"mz\":");
    list(o, mz);
    o.raw(",\"y\":");
    list(o, y);
    o.raw("}");
}

/// The MS1 spectrum at the scan of section `si` nearest to `apex`, from `SPEC_BELOW` below to `SPEC_ABOVE` above `mz`: `(rt of the scan, m/z, intensity)`.
fn ms1_at<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
    rts: &[f64],
    apex: f64,
    mz: f64,
) -> Result<Peaks> {
    let near = (0..rts.len())
        .min_by(|&i, &j| (rts[i] - apex).abs().total_cmp(&(rts[j] - apex).abs()))
        .unwrap_or(0);
    let (a, b) = (
        index.search(si, mz - SPEC_BELOW, false)?,
        index.search(si, mz + SPEC_ABOVE, true)?,
    );
    let (mut sx, mut sy) = (Vec::new(), Vec::new());
    index.each_peak(si, a, b, |p, m, v| {
        if p == near {
            sx.push(m);
            sy.push(v);
        }
    })?;
    Ok((rts[near], sx, sy))
}

/// The MS2 the exports use for a row of the file of `q`: `(precursor, m/z, intensity)`.
fn ms2_of<R: ReadAt>(
    index: &mut Index<R>,
    q: &crate::table::RowQ,
    mz: f64,
    pol: i8,
) -> Result<Option<Peaks>> {
    let mut by_rt: Vec<u32> = (0..index.scans.len() as u32)
        .filter(|&g| index.scans[g as usize].level == 2)
        .collect();
    by_rt.sortc_by(|&a, &b| {
        index.scans[a as usize]
            .rt
            .total_cmp(&index.scans[b as usize].rt)
    });
    let Some(g) = pick(
        index,
        &by_rt,
        mz,
        pol,
        f64::from(q.rt_lo),
        f64::from(q.rt_hi),
    ) else {
        return Ok(None);
    };
    let prec = index.scans[g as usize].prec(0.0);
    Ok(read_peaks(index, pol, &[g])?
        .remove(&g)
        .map(|(m, v)| (prec, m, v)))
}

/// The `riga_tracce/1` of the row with `id`: `xic` (one per file: `rt`, `y` in the window of the edges of the peak plus `PAD_WIDTHS` widths on each side, at
/// most `MAX_POINTS`), `ms1` (spectrum at the apex of the file with the largest area, from `SPEC_BELOW` below to `SPEC_ABOVE` above the m/z of the row) and
/// `ms2` (the one `ms2spec` chooses for the exports, from `RowMeta::ms2_file`); `ms1` and `ms2` are `null` when there is none. `open(k)` gives the index of file `k`.
pub fn traces<R: ReadAt>(
    t: &Aligned,
    s: &[Sample],
    id: u32,
    mut open: impl FnMut(usize) -> Result<Index<R>>,
    ppm: f64,
) -> Result<String> {
    let r = check(t, s, id)?;
    let mz = t.mz[r];
    let tol = mz * ppm.abs() * 1e-6;
    let q = (!t.meta.is_empty()).then(|| t.meta.get(r));
    let best = (0..t.n_files)
        .filter_map(|k| measured(t, r, k).map(|a| (k, a)))
        .max_by(|x, y| x.1.total_cmp(&y.1))
        .map(|x| x.0);
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(TRACES_SCHEMA)
        .raw(",\"id\":")
        .int(i64::from(id))
        .raw(",\"mz\":")
        .num(mz)
        .raw(",\"ppm\":")
        .num(ppm)
        .raw(",\"xic\":[");
    let mut ms1 = None;
    let mut pol = 1i8;
    for (k, smp) in s.iter().enumerate() {
        let mut index = open(k)?;
        let (apex, lo, hi) = window(t, r, k, q.as_ref());
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"file\":")
            .str(&smp.file)
            .raw(",\"rt\":");
        let (mut x, mut y) = (Vec::new(), Vec::new());
        if let Some((si, p)) = section_pol(&index, 1) {
            if k == 0 {
                pol = p;
            }
            let rts;
            (rts, x, y) = xic_of(&mut index, si, mz, tol, (lo, hi))?;
            if best == Some(k) && !rts.is_empty() {
                ms1 = Some((k, ms1_at(&mut index, si, &rts, apex, mz)?));
            }
        }
        let (x, y) = decimate(&x, &y);
        list(&mut o, &x);
        o.raw(",\"y\":");
        list(&mut o, &y);
        o.raw("}");
    }
    o.raw("],\"ms1\":");
    match ms1 {
        Some((k, (rt, sx, sy))) => spectrum(&mut o, &s[k].file, ("rt", rt), &sx, &sy),
        None => {
            o.raw("null");
        }
    }
    o.raw(",\"ms2\":");
    let ms2 = match q.filter(|q| q.ms2_file != NO_FILE && (q.ms2_file as usize) < t.n_files) {
        Some(q) => {
            ms2_of(&mut open(q.ms2_file as usize)?, &q, mz, pol)?.map(|m| (q.ms2_file as usize, m))
        }
        None => None,
    };
    match ms2 {
        Some((k, (prec, m, v))) => spectrum(&mut o, &s[k].file, ("prec", prec), &m, &v),
        None => {
            o.raw("null");
        }
    }
    o.raw("}");
    Ok(o.0)
}
