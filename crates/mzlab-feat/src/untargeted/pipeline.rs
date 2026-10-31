//! The detection pipeline after mzmine 3.9.0 in one place: ADAP chromatograms -> local-minimum resolver (one file), join aligner -> gap filling -> the
//! `Aligned` table (a series). Each stage is rewritten from scratch in its own module (`adap`, `resolve`, `join`, `gapfill`, `table`: files of mzmine 3.9.0 and
//! licence in their headers; MIT, (c) 2004-2022 The mzmine Development Team; paper MZmine 3, Schmid et al., Nat. Biotechnol. 41:447 (2023),
//! doi:10.1038/s41587-023-01690-2); this file only connects them, so there is nothing of mzmine to attribute beyond what those headers say.
//!
//! The recipe is the JSON `{"engine": "untargeted", "polarity": 1 | -1 | null, "adap": {...}, "baseline": null | {...}, "resolve": {...}, "join": {...}, "gap": {...}, "iso": {...}, "ident": {...}}` (`"iso": null` leaves out the isotope groups and identities); a missing key
//! keeps the default of the module (the values of `tools/bench/parametri/mzmine/predefinita.json`). The names inside each object are the Rust fields of the
//! parameter structs, with the label of mzmine in the comments there. The features of a file travel between the two steps as a `.mzfeatf` (the usual store:
//! m/z, RT, limits, area in counts x seconds, height); to make that round trip exact the features of `run_file` are already rounded to what the store keeps
//! (RT and height as `f32`), and are sorted by m/z as the store wants.

use crate::sortc::SortC;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{self, Value};

use super::adap::{AdapParams, Chrom as AdapChrom};
use super::arpls;
use super::gapfill::{fill, GapParams};
use super::ident::{ident, IdentParams};
use super::isotopes::{group, IsoParams, Representative};
use super::join::{align, JoinParams};
use super::pipeline_stream::{common_mode_params, each_slab, Kept, TraceBuf};
use super::resolve::{resolve_with_broad, MinParams};
use super::table::to_aligned;
use super::Feature;
use crate::broad::BroadParams;
use crate::quality::REASON_BROAD;
use crate::table::{Aligned, FileFeatures};

/// Bytes of points that the ADAP builder may hold at once (`adap::each_slab`).
const SLAB_BYTES: usize = 64 << 20;

/// Counts x seconds per intensity x minute.
const MIN_TO_S: f64 = 60.0;

/// Optional arPLS baseline correction of each chromatogram before the resolver (mzmine 4.10 «Baseline correction»; off in the gold batches).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaselineParams {
    pub lambda: f64,
    pub max_iter: usize,
    /// «Exclude peaks»: fit the baseline on the trace with its peaks bridged.
    pub exclude_peaks: bool,
}

impl Default for BaselineParams {
    fn default() -> Self {
        BaselineParams {
            lambda: 1e5,
            max_iter: 50,
            exclude_peaks: false,
        }
    }
}

impl BaselineParams {
    /// The trace `y` on the RT axis `x` (minutes) without its baseline.
    pub fn apply(&self, x: &[f64], y: &[f64]) -> Vec<f64> {
        if self.exclude_peaks {
            arpls::correct_excluding_peaks(x, y, self.lambda, self.max_iter)
        } else {
            arpls::correct(y, self.lambda, self.max_iter)
        }
    }
}

/// The numbers of the whole pipeline.
#[derive(Debug, Clone)]
pub struct UntParams {
    /// Polarity of the MS1 scans used (`None` = all of them); also the polarity of the gap filler.
    pub polarity: Option<i8>,
    pub adap: AdapParams,
    /// `None` (the default, as in the mzmine 4.10 gold batches) leaves the traces as the ADAP builder gives them.
    pub baseline: Option<BaselineParams>,
    pub resolve: MinParams,
    /// Broad bands on the residue of the narrow peaks (off by default).
    pub broad: BroadParams,
    pub join: JoinParams,
    pub gap: GapParams,
    /// Isotope groups and identities of the rows of the table (`"iso": null` or `false` leaves them out).
    pub iso: Option<(IsoParams, IdentParams)>,
    /// Bytes of points the ADAP builder holds at once.
    pub slab_bytes: usize,
}

impl Default for UntParams {
    fn default() -> Self {
        UntParams {
            polarity: None,
            adap: AdapParams::default(),
            baseline: None,
            // the product follows mzmine 4.10: the zero next to an edge enters the feature (`MinParams::adjust_edges`)
            resolve: MinParams {
                adjust_edges: true,
                ..MinParams::default()
            },
            broad: BroadParams::default(),
            join: JoinParams::default(),
            // the mass list the filler reads is the one of the mass detector
            gap: GapParams {
                noise: AdapParams::default().noise,
                ..GapParams::default()
            },
            iso: Some((IsoParams::default(), IdentParams::default())),
            slab_bytes: SLAB_BYTES,
        }
    }
}

fn bad() -> Error {
    Error::new("error.badRecipe")
}

/// Sets `out` from `obj[key]` when it is a finite number; anything else that is present is an error.
fn num(obj: &Value, key: &str, out: &mut f64) -> Result<()> {
    match obj.get(key) {
        None => Ok(()),
        Some(v) => match v.f64().filter(|x| x.is_finite() && *x >= 0.0) {
            Some(x) => {
                *out = x;
                Ok(())
            }
            None => Err(bad()),
        },
    }
}

fn count(obj: &Value, key: &str, out: &mut usize) -> Result<()> {
    let mut x = *out as f64;
    num(obj, key, &mut x)?;
    *out = x as usize;
    Ok(())
}

impl UntParams {
    /// Reads the recipe; an empty text is the defaults. A wrong type or a negative number is `error.badRecipe`.
    pub fn from_json(text: &str) -> Result<UntParams> {
        let mut p = UntParams::default();
        if text.trim().is_empty() {
            return Ok(p);
        }
        let v = json::parse(text.as_bytes()).map_err(|_| bad())?;
        if let Some(e) = v.get("engine").and_then(Value::str)
            && e != "untargeted"
        {
            return Err(bad());
        }
        p.polarity = match v.get("polarity") {
            None | Some(Value::Null) => None,
            Some(x) => match x.f64() {
                Some(1.0) => Some(1),
                Some(-1.0) => Some(-1),
                _ => return Err(bad()),
            },
        };
        p.gap.polarity = p.polarity;
        let null = Value::Null;
        let sect = |k: &str| v.get(k).unwrap_or(&null);
        let (a, r, j, g) = (sect("adap"), sect("resolve"), sect("join"), sect("gap"));
        num(a, "noise", &mut p.adap.noise)?;
        count(a, "min_consecutive", &mut p.adap.min_consecutive)?;
        num(a, "min_group_intensity", &mut p.adap.min_group_intensity)?;
        num(a, "min_highest", &mut p.adap.min_highest)?;
        num(a, "tol_da", &mut p.adap.tol_da)?;
        num(a, "tol_ppm", &mut p.adap.tol_ppm)?;
        p.gap.noise = p.adap.noise;
        read_baseline(&v, &mut p)?;
        read_resolve(r, &mut p.resolve)?;
        if let Some(b) = r.get("broad") {
            p.broad.enabled = b.bool().ok_or_else(bad)?;
        }
        num(j, "tol_da", &mut p.join.tol_da)?;
        num(j, "tol_ppm", &mut p.join.tol_ppm)?;
        num(j, "w_mz", &mut p.join.w_mz)?;
        num(j, "w_rt", &mut p.join.w_rt)?;
        num(j, "rt_tol", &mut p.join.rt_tol)?;
        num(g, "intensity_tol", &mut p.gap.intensity_tol)?;
        num(g, "tol_da", &mut p.gap.tol_da)?;
        num(g, "tol_ppm", &mut p.gap.tol_ppm)?;
        num(g, "rt_tol", &mut p.gap.rt_tol)?;
        count(g, "min_points", &mut p.gap.min_points)?;
        num(g, "noise", &mut p.gap.noise)?;
        read_iso(&v, &mut p).map(|()| p)
    }
}

/// The `"baseline"` object (`lambda`, `max_iter`, `exclude_peaks`); absent, `null` or `false` leaves the correction off, `true` or `{}` turns it on with the defaults.
fn read_baseline(v: &Value, p: &mut UntParams) -> Result<()> {
    let b = match v.get("baseline") {
        None | Some(Value::Null) => return Ok(()),
        Some(x) if x.bool() == Some(false) => return Ok(()),
        Some(x) => x,
    };
    let mut q = BaselineParams::default();
    if b.bool() != Some(true) {
        num(b, "lambda", &mut q.lambda)?;
        count(b, "max_iter", &mut q.max_iter)?;
        if let Some(e) = b.get("exclude_peaks") {
            q.exclude_peaks = e.bool().ok_or_else(bad)?;
        }
    }
    if q.lambda <= 0.0 {
        return Err(bad());
    }
    p.baseline = Some(q);
    Ok(())
}

/// The `"resolve"` object: the numbers of [`MinParams`] by their names, `duration` as `[min, max]` and `adjust_edges` as a boolean; `broad` (boolean, default false) switches the broad-band pass of [`UntParams::broad`].
fn read_resolve(r: &Value, m: &mut MinParams) -> Result<()> {
    num(r, "chrom_threshold", &mut m.chrom_threshold)?;
    num(r, "search_rt", &mut m.search_rt)?;
    num(r, "min_ratio", &mut m.min_ratio)?;
    num(r, "min_abs_height", &mut m.min_abs_height)?;
    num(r, "min_rel_height", &mut m.min_rel_height)?;
    count(r, "min_points", &mut m.min_points)?;
    match r.get("adjust_edges") {
        None => {}
        Some(v) => m.adjust_edges = v.bool().ok_or_else(bad)?,
    }
    if let Some(d) = r.get("duration") {
        match (d.at(0).and_then(Value::f64), d.at(1).and_then(Value::f64)) {
            (Some(lo), Some(hi)) if lo.is_finite() && hi >= lo => m.duration = (lo, hi),
            _ => {
                if let Some(hi) = d.f64() {
                    if hi.is_finite() && hi >= m.duration.0 {
                        m.duration.1 = hi;
                    } else {
                        return Err(bad());
                    }
                } else {
                    return Err(bad());
                }
            }
        }
    }
    if let Some(hi) = r.get("max_duration").and_then(Value::f64) {
        if hi.is_finite() && hi >= m.duration.0 {
            m.duration.1 = hi;
        } else {
            return Err(bad());
        }
    }
    Ok(())
}

impl From<&super::resolve::Feature> for Feature {
    /// A resolved peak as a feature of the list: apex RT, limits and height as they are, area in intensity x minutes.
    fn from(f: &super::resolve::Feature) -> Feature {
        Feature {
            mz: f.mz,
            rt: f.rt_apex,
            rt_start: f.rt_start,
            rt_end: f.rt_end,
            height: f.height,
            area: f.area,
            filled: false,
            broad: f.broad,
            apex: f.apex_scan as u32,
            scan_lo: f.scans.0 as u32,
            scan_hi: f.scans.1 as u32,
            npts: f.npts.min(u16::MAX as usize) as u16,
            snr: f.snr as f32,
            fwhm: f.fwhm_scans as f32,
        }
    }
}

/// The `"iso"` object (`tol_da`, `tol_ppm`, `rt_tol`, `max_charge`, `monotonic`, `representative`: `"most_intense"` or `"lowest_mz"`) and the `"ident"`
/// object (`rt_tol`, `win`: ppm if positive, Da if negative).
fn read_iso(v: &Value, p: &mut UntParams) -> Result<()> {
    if matches!(v.get("iso"), Some(x) if x.is_null() || x.bool() == Some(false)) {
        p.iso = None;
    }
    let Some((ip, dp)) = p.iso.as_mut() else {
        return Ok(());
    };
    let null = Value::Null;
    let (i, d) = (
        v.get("iso").unwrap_or(&null),
        v.get("ident").unwrap_or(&null),
    );
    num(i, "tol_da", &mut ip.tol_da)?;
    num(i, "tol_ppm", &mut ip.tol_ppm)?;
    num(i, "rt_tol", &mut ip.rt_tol)?;
    let mut z = f64::from(ip.max_charge);
    num(i, "max_charge", &mut z)?;
    ip.max_charge = z.clamp(1.0, 6.0) as u8;
    if let Some(m) = i.get("monotonic") {
        ip.monotonic = m.bool().ok_or_else(bad)?;
    }
    match i.get("representative").map(Value::str) {
        None => {}
        Some(Some("most_intense")) => ip.representative = Representative::MostIntense,
        Some(Some("lowest_mz")) => ip.representative = Representative::LowestMz,
        Some(_) => return Err(bad()),
    }
    num(d, "rt_tol", &mut dp.rt_tol)?;
    if let Some(w) = d.get("win") {
        dp.win = w
            .f64()
            .filter(|x| x.is_finite() && *x != 0.0)
            .ok_or_else(bad)?;
    }
    Ok(())
}

/// What the store keeps of a feature: RT, limits and height in `f32`.
fn stored(mut f: Feature) -> Feature {
    f.rt = f.rt as f32 as f64;
    f.rt_start = f.rt_start as f32 as f64;
    f.rt_end = f.rt_end as f32 as f64;
    f.height = f.height as f32 as f64;
    f
}

/// ADAP + resolver on the MS1 scans of one file: its list of features, sorted by m/z (then RT). Memory: the points of one slab of m/z at a time.
pub fn run_file<R: ReadAt>(ix: &mut Index<R>, p: &UntParams) -> Result<Vec<Feature>> {
    let Some(si) = ix.section_of(1, p.polarity) else {
        return Ok(Vec::new());
    };
    // the RT of mzmine is a float: the resolver compares distances on that axis
    let axis: Vec<f64> = ix
        .arrays(si)?
        .rt
        .iter()
        .map(|&rt| rt as f32 as f64)
        .collect();
    let mut kept = Kept(Some((Vec::new(), 0)));
    let with_gt = common_mode_params(ix, si, &axis, p, &mut kept)?;
    let p = with_gt.as_ref().unwrap_or(p);
    let mut out = Vec::new();
    let mut failed = None;
    let mut buf = TraceBuf::new(axis.len());
    let mut resolve_slab = |chroms: Vec<AdapChrom>| {
        for c in &chroms {
            buf.load(c);
            let res = match &p.baseline {
                Some(b) => {
                    let mut y = b.apply(&axis, &buf.t.y);
                    std::mem::swap(&mut buf.t.y, &mut y);
                    let r = resolve_with_broad(&buf.t, &axis, &p.resolve, &p.broad);
                    std::mem::swap(&mut buf.t.y, &mut y);
                    r
                }
                None => resolve_with_broad(&buf.t, &axis, &p.resolve, &p.broad),
            };
            match res {
                Ok(fs) => out.extend(fs.iter().map(|f| stored(f.into()))),
                Err(e) => failed = failed.take().or(Some(e)),
            }
        }
    };
    match kept.0 {
        Some((slabs, _)) => slabs.into_iter().for_each(&mut resolve_slab),
        None => each_slab(ix, si, axis.len(), p, resolve_slab)?,
    }
    if let Some(e) = failed {
        return Err(e);
    }
    out.sortc_by(|a, b| a.mz.total_cmp(&b.mz).then(a.rt.total_cmp(&b.rt)));
    Ok(out)
}

/// Join + gap filling + table of a series: `lists[k]` and `ixs[k]` belong to file `k`; `files` are their names.
pub fn run_series<R: ReadAt>(
    lists: &[Vec<Feature>],
    ixs: &mut [Index<R>],
    files: &[String],
    p: &UntParams,
) -> Result<Aligned> {
    let mut rows = align(lists, &p.join);
    let gap = GapParams {
        polarity: p.polarity,
        ..p.gap
    };
    fill(&mut rows, ixs, &gap)?;
    let mut t = to_aligned(&rows, files);
    if let Some((ip, dp)) = &p.iso {
        let groups = group(&t, ip);
        let named = ident(&t, &groups, p.polarity.unwrap_or(1), dp);
        let (id, charge) = groups.per_row(t.len());
        t.iso = crate::table::IsoBlock {
            group: id,
            charge,
            family: named.family,
        };
        t.ident = named.ident;
    }
    Ok(t)
}

/// The features of a file as the table of the store (`.mzfeatf`): area in counts x seconds, the scans, point count and (broad bands) SNR and width that the resolver knows; the columns that mzmine does not give stay at their neutral value.
pub fn to_file_features(list: &[Feature]) -> FileFeatures {
    let n = list.len();
    FileFeatures {
        mz: list.iter().map(|f| f.mz).collect(),
        rt: list.iter().map(|f| f.rt as f32).collect(),
        left: list.iter().map(|f| f.rt_start as f32).collect(),
        right: list.iter().map(|f| f.rt_end as f32).collect(),
        area: list.iter().map(|f| f.area * MIN_TO_S).collect(),
        height: list.iter().map(|f| f.height as f32).collect(),
        apex: list.iter().map(|f| f.apex).collect(),
        scan_lo: list.iter().map(|f| f.scan_lo).collect(),
        scan_hi: list.iter().map(|f| f.scan_hi).collect(),
        npts: list.iter().map(|f| f.npts).collect(),
        flags: vec![0; n],
        iso_group: vec![0; n],
        charge: vec![0; n],
        shape: vec![0.0; n],
        snr: list.iter().map(|f| f.snr).collect(),
        fwhm: list.iter().map(|f| f.fwhm).collect(),
        asym: vec![0.0; n],
        gaps: vec![0; n],
        qreason: list
            .iter()
            .map(|f| if f.broad { REASON_BROAD } else { 0 })
            .collect(),
        ms2_n: vec![0; n],
        purity: vec![-1.0; n],
        area_win: vec![0.0; n],
        family_id: vec![0; n],
        role: vec![0; n],
        alt: vec![0; n],
        ..FileFeatures::default()
    }
}

/// The list of features back from the table of the store (the inverse of `to_file_features`).
pub fn from_file_features(f: &FileFeatures) -> Vec<Feature> {
    (0..f.len())
        .map(|i| Feature {
            mz: f.mz[i],
            rt: f.rt[i] as f64,
            rt_start: f.left[i] as f64,
            rt_end: f.right[i] as f64,
            height: f.height[i] as f64,
            area: f.area[i] / MIN_TO_S,
            filled: false,
            broad: f.qreason[i] & REASON_BROAD != 0,
            apex: f.apex[i],
            scan_lo: f.scan_lo[i],
            scan_hi: f.scan_hi[i],
            npts: f.npts[i],
            snr: f.snr[i],
            fwhm: f.fwhm[i],
        })
        .collect()
}
