//! MRM files (one chromatogram per transition Q1 > Q3, no scans): the same table as the other files, with the transition as the unit.
//!
//! A peak is a peak of the chromatogram of a transition, found by its own resolver: the 3200 reports multiples of one count, mostly zeros, so the
//! chromatogram is averaged over seconds and its counting noise made even (`Stabilise`) before the peaks are measured in standard deviations; its `mz`
//! is the Q1 and its `q3` the Q3. There are no isotopes, no MS2 and no families: the Q3 is already the proof.
//! The series are aligned by transition (Q1, Q3 to 0.01) and RT, and the cells without a peak are integrated on the chromatogram of the transition.
//! The qualifier/quantifier ratios and the long table of the calibration standards are here too; the calibration line is never computed.

use mzlab_core::error::Result;
use mzlab_core::peakq::{quality, Quality};
use mzlab_core::peaks::noise_level;
use mzlab_core::sorgente::Tracce;

mod resolver;
use resolver::{area_above, boxcar, odd_points, resolve, Span, Stabilise};

use crate::export::{fix, Ctx};
use crate::gapfill::{expected_window, trace_area, FillParams};
use crate::recipe::Recipe;
use crate::store::{write_file_to, write_table_to, Origin};
use crate::table::*;
use mzlab_core::budget::WriteAt;

/// The chromatogram of one transition: times in minutes.
#[derive(Debug, Clone, PartialEq)]
pub struct Srm {
    pub q1: f64,
    pub q3: f64,
    pub name: String,
    pub t: Vec<f64>,
    pub y: Vec<f64>,
}

/// The SRM chromatograms of a file read through `Tracce` (an mzML: `TracceMzml`, whose `scans_end` is where its last scan ends, 0 without scans; or a sample of a
/// vendor file), ordered by Q1 then Q3.
pub fn read_srm<T: Tracce>(t: &mut T) -> Result<Vec<Srm>> {
    let mut out: Vec<Srm> = mzlab_core::srm::leggi(t)?
        .into_iter()
        .map(|s| Srm {
            q1: s.q1,
            q3: s.q3,
            name: s.name,
            t: s.t,
            y: s.y,
        })
        .collect();
    out.sort_by(|a, b| a.q1.total_cmp(&b.q1).then(a.q3.total_cmp(&b.q3)));
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct MrmParams {
    /// Rise of a peak over the higher of its valleys, in standard deviations of the noise of the averaged chromatogram (`Stabilise`). The rise is the
    /// difference of two noisy points (x 1.4) picked as the extremes among the thousands of windows of a chromatogram, and with a few counts per window
    /// the Poisson tail is longer than the Gaussian one: 10 keeps that noise out.
    pub snr: f64,
    /// Lowest height of a peak, counts; the noise of the chromatogram sets the real threshold above it.
    pub min_height: f64,
    /// Shortest LC peak at its base, seconds: the chromatogram is smoothed over half of it and a peak spans at least all of it. A peak of liquid
    /// chromatography lasts seconds (a few in UHPLC, tens in HPLC) while an MRM chromatogram has a point every few hundredths of a second: a width in
    /// points would let an isolated count through.
    pub min_width_s: f64,
    /// Peaks of one transition closer than this (minutes) are one row.
    pub rt_gap: f64,
    /// Half width added to the window of the peak when a cell is integrated (minutes).
    pub fill_tol: f64,
    pub fill: bool,
}

impl Default for MrmParams {
    fn default() -> Self {
        MrmParams {
            snr: 10.0,
            min_height: 10.0,
            min_width_s: 3.0,
            rt_gap: 0.15,
            fill_tol: 0.1,
            fill: true,
        }
    }
}

/// The transition key: Q1 and Q3 to the hundredth of a dalton.
fn key(q: f64) -> i64 {
    (q * 100.0).round() as i64
}

/// Median interval between two points, seconds.
fn interval(t: &[f64]) -> f64 {
    let mut d: Vec<f64> = t.windows(2).map(|w| (w[1] - w[0]) * 60.0).collect();
    crate::sortc::by(&mut d, f64::total_cmp);
    d.get(d.len() / 2).copied().unwrap_or(0.0)
}

/// The peaks of every transition of a file, ordered by Q1, Q3 and RT. Area in counts x seconds. The chromatogram is averaged over half the shortest
/// peak (`MrmParams.min_width_s`) and stabilised (`Stabilise`); a peak rises `MrmParams.snr` standard deviations over its valleys and spans at least
/// `min_width_s`. Height and area are taken on the averaged chromatogram (the moving average keeps the area); a peak without area is dropped.
pub fn detect(traces: &[Srm], p: &MrmParams) -> FileFeatures {
    let mut f = FileFeatures {
        dt: traces.first().map_or(0.0, |s| interval(&s.t)),
        ..FileFeatures::default()
    };
    for s in traces {
        let dt = interval(&s.t);
        let n = s.y.len().min(s.t.len());
        let w = odd_points(p.min_width_s / 2.0, dt);
        let ys = boxcar(&s.y[..n], w);
        let st = Stabilise::new(&s.y[..n], w);
        let zs: Vec<f64> = ys.iter().map(|&v| st.z(v)).collect();
        let min_pts = if dt > 0.0 {
            (p.min_width_s / dt).ceil() as usize
        } else {
            0
        }
        .max(3);
        for k in resolve(&zs, p.snr, min_pts) {
            let area = area_above(&s.t, &ys, k.lo, k.hi) * 60.0;
            if ys[k.apex] < p.min_height || area <= 0.0 {
                continue;
            }
            let q = quality(&s.t, &ys, k.lo, k.hi, st.sigma);
            push_peak(&mut f, s, &k, (ys[k.apex], area), &q);
        }
    }
    f
}

/// Appends the peak `k` of the transition `s` to `f`, with its height and its area in counts x seconds.
fn push_peak(f: &mut FileFeatures, s: &Srm, k: &Span, (height, area): (f64, f64), q: &Quality) {
    f.mz.push(s.q1);
    f.q3.push(s.q3);
    f.rt.push(s.t[k.apex] as f32);
    f.left.push(s.t[k.lo] as f32);
    f.right.push(s.t[k.hi] as f32);
    f.apex.push(k.apex as u32);
    f.scan_lo.push(k.lo as u32);
    f.scan_hi.push(k.hi as u32);
    f.area.push(area);
    f.height.push(height as f32);
    f.npts
        .push((k.hi - k.lo + 1).min(usize::from(u16::MAX)) as u16);
    f.flags.push(FLAG_GOOD);
    f.iso_group.push(0);
    f.charge.push(1);
    f.shape.push(q.gaussian as f32);
    f.snr.push(k.rise as f32);
    f.fwhm.push(0.0);
    f.asym.push(q.asymmetry.unwrap_or(0.0) as f32);
    f.gaps.push(0);
    f.qreason.push(0);
    f.ms2_n.push(0);
    f.purity.push(-1.0);
    f.area_win.push(area);
    f.family_id.push(0);
    f.role.push(ROLE_M);
    f.alt.push(0);
}

/// (key of Q1, key of Q3, RT, file, position in the file) of a peak, and the transition key with the (file, position) of the peaks of a row.
type Peak = (i64, i64, f32, usize, usize);
type Cluster = ((i64, i64), Vec<(usize, usize)>);

/// Aligns the files by transition and RT. Cells without a peak have area 0, NaN apex and width, state `CELL_ABSENT`.
pub fn align(files: &[&FileFeatures], p: &MrmParams) -> Aligned {
    let nf = files.len();
    let mut all: Vec<Peak> = Vec::new();
    for (k, f) in files.iter().enumerate() {
        for i in 0..f.len() {
            all.push((key(f.mz[i]), key(f.q3[i]), f.rt[i], k, i));
        }
    }
    all.sort_by(|a, b| {
        (a.0, a.1)
            .cmp(&(b.0, b.1))
            .then(a.2.total_cmp(&b.2))
            .then((a.3, a.4).cmp(&(b.3, b.4)))
    });
    // clusters in order of key then RT: a peak joins the last cluster of its transition when it is within the gap of its RT and the file is not in it yet
    let mut clusters: Vec<Cluster> = Vec::new();
    for &(a, b, rt, k, i) in &all {
        let joins = clusters.last().is_some_and(|(ky, m)| {
            *ky == (a, b)
                && !m.iter().any(|x| x.0 == k)
                && m.iter()
                    .map(|&(kk, ii)| f64::from(files[kk].rt[ii]))
                    .sum::<f64>()
                    / m.len() as f64
                    - f64::from(rt)
                    >= -p.rt_gap
        });
        if joins {
            clusters.last_mut().expect("checked").1.push((k, i));
        } else {
            clusters.push(((a, b), vec![(k, i)]));
        }
    }
    let mut t = Aligned::with_capacity(nf, clusters.len());
    for (_, m) in &clusters {
        let (k0, i0) = m[0];
        t.mz.push(files[k0].mz[i0]);
        t.q3.push(files[k0].q3[i0]);
        t.rt.push(
            m.iter()
                .map(|&(k, i)| f64::from(files[k].rt[i]))
                .sum::<f64>()
                / m.len() as f64,
        );
        for (k, &f) in files.iter().enumerate() {
            match m.iter().find(|x| x.0 == k) {
                Some(&(_, i)) => {
                    t.area.push(f.area[i]);
                    t.height.push(f.height[i]);
                    t.apex_rt.push(f.rt[i]);
                    t.width.push(f.right[i] - f.left[i]);
                    t.state.push(CELL_DETECTED);
                }
                None => {
                    t.area.push(0.0);
                    t.height.push(0.0);
                    t.apex_rt.push(f32::NAN);
                    t.width.push(f32::NAN);
                    t.state.push(CELL_ABSENT);
                }
            }
        }
    }
    t.sort_rows();
    t
}

/// Integrates the cells of `file` without a peak on the chromatogram of their transition (the threshold is three times the noise of the chromatogram).
/// Returns the number of cells filled.
pub fn fill_file(al: &mut Aligned, file: usize, traces: &[Srm], p: &MrmParams) -> usize {
    let nf = al.n_files;
    let base = FillParams {
        rt_tol: p.fill_tol,
        min_points: 4,
        ..FillParams::default()
    };
    let mut filled = 0;
    for row in 0..al.len() {
        if al.state[row * nf + file] != CELL_ABSENT {
            continue;
        }
        let Some(s) = traces
            .iter()
            .find(|s| key(s.q1) == key(al.mz[row]) && key(s.q3) == key(al.q3[row]))
        else {
            continue;
        };
        let Some((c, w)) = expected_window(al, row, file, None, &base) else {
            continue;
        };
        let (i0, i1) = (
            s.t.partition_point(|&r| r < c - w),
            s.t.partition_point(|&r| r <= c + w),
        );
        if i1 <= i0 || s.t.len() < 3 {
            continue;
        }
        let fp = FillParams {
            floor: (3.0 * noise_level(&s.y)).max(f64::MIN_POSITIVE),
            rt_tol: base.rt_tol,
            min_points: base.min_points,
            ..FillParams::default()
        };
        if let Some(area) = trace_area(&s.y, 0, i0, i1, interval(&s.t), &fp).filter(|&a| a > 0.0) {
            al.area[row * nf + file] = area;
            al.state[row * nf + file] = CELL_FILLED;
            filled += 1;
        }
    }
    filled
}

/// A qualifier against the quantifier of its precursor: the ratio of their areas in every file (NaN where one has no area).
#[derive(Debug, Clone, PartialEq)]
pub struct QualRatio {
    pub qualifier: usize,
    pub quantifier: usize,
    pub ratio: Vec<f64>,
}

/// For every Q1 with several transitions, each of the others against the one with the largest median area. Measured, never a verdict.
pub fn qualifier_ratios(t: &Aligned) -> Vec<QualRatio> {
    let nf = t.n_files;
    let median = |r: usize| {
        let mut v: Vec<f64> = (0..nf)
            .map(|k| t.area[r * nf + k])
            .filter(|&a| a > 0.0)
            .collect();
        crate::sortc::by(&mut v, f64::total_cmp);
        v.get(v.len() / 2).copied().unwrap_or(0.0)
    };
    let mut out = Vec::new();
    let mut a = 0;
    while a < t.len() {
        let b = (a..t.len())
            .take_while(|&r| key(t.mz[r]) == key(t.mz[a]))
            .last()
            .map_or(a, |r| r)
            + 1;
        if b - a > 1 {
            let quant = (a..b)
                .max_by(|&x, &y| median(x).total_cmp(&median(y)).then(y.cmp(&x)))
                .unwrap_or(a);
            for r in (a..b).filter(|&r| r != quant) {
                let ratio = (0..nf)
                    .map(|k| {
                        let (n, d) = (t.area[r * nf + k], t.area[quant * nf + k]);
                        if n > 0.0 && d > 0.0 {
                            n / d
                        } else {
                            f64::NAN
                        }
                    })
                    .collect();
                out.push(QualRatio {
                    qualifier: r,
                    quantifier: quant,
                    ratio,
                });
            }
        }
        a = b;
    }
    out
}

/// The long table of the calibration standards, ready for the line (that the programme does not compute): one line for every transition and every file of
/// type `calibration` or `std`, with its concentration and unit, RT, area, height and state.
pub fn csv_calibration(c: &Ctx) -> String {
    let t = c.table;
    let mut out =
        String::from("transition,q1,q3,sample,type,concentration,unit,rt,area,height,state\n");
    for r in c.rows() {
        for (k, s) in c
            .samples
            .iter()
            .enumerate()
            .filter(|(_, s)| s.kind == "calibration" || s.kind == "std")
        {
            let cell = r * t.n_files + k;
            let state = match t.state[cell] {
                CELL_DETECTED => "DETECTED",
                CELL_FILLED => "FILLED",
                _ => "ABSENT",
            };
            let (q1, q3) = (fix(t.mz[r], 2), fix(t.q3.get(r).copied().unwrap_or(0.0), 2));
            let stem = s.file.rsplit_once('.').map_or(s.file.as_str(), |x| x.0);
            let a = c.area(r, k);
            out.push_str(&format!(
                "{q1}>{q3},{q1},{q3},\"{}\",{},{},{},{},{},{},{state}\n",
                stem.replace('"', "\"\""),
                s.kind,
                s.conc.map_or(String::new(), |v| fix(v, 6)),
                // fixed size: a unit label, empty when absent
                s.unit.clone().unwrap_or_default(),
                if t.apex_rt[cell].is_nan() {
                    fix(t.rt[r], 4)
                } else {
                    fix(f64::from(t.apex_rt[cell]), 4)
                },
                a.map_or(String::new(), |v| fix(v, 1)),
                if a.is_some() {
                    fix(f64::from(t.height[cell]), 1)
                } else {
                    String::new()
                },
            ));
        }
    }
    out
}

/// The recipe written in the meta of the files of an MRM series: the unit one (the masses are nominal), with the threshold of the heights.
pub fn recipe(p: &MrmParams) -> Recipe {
    Recipe::lr(0.0, 0.0, p.min_height)
}

/// The `.mzfeatf` of one MRM file written to `sink` (`name` and `sha256` of the source go in its meta; no times, so the bytes repeat).
pub fn file_to<W: WriteAt>(
    traces: &[Srm],
    name: &str,
    sha256: &str,
    p: &MrmParams,
    sink: &mut W,
) -> Result<()> {
    let o = Origin {
        inputs: vec![(name.to_string(), sha256.to_string())],
        ..Origin::default()
    };
    write_file_to(sink, &detect(traces, p), &recipe(p), &o)
}

/// The `.mzfeat` of a series written to `sink`: the tables of the files aligned by transition and RT, then the cells without a peak integrated on the
/// chromatograms of their file (`load(k)` gives the transitions of file `k`).
pub fn table_to<W: WriteAt>(
    files: &[FileFeatures],
    origin: &Origin,
    p: &MrmParams,
    mut load: impl FnMut(usize) -> Result<Vec<Srm>>,
    sink: &mut W,
) -> Result<()> {
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let mut t = align(&refs, p);
    if p.fill {
        for k in 0..files.len() {
            fill_file(&mut t, k, &load(k)?, p);
        }
    }
    write_table_to(sink, &t, &recipe(p), origin, &mut (&[][..]))
}

/// `file_to` into memory (small series and tests).
pub fn file_bytes(traces: &[Srm], name: &str, sha256: &str, p: &MrmParams) -> Vec<u8> {
    let mut v = Vec::new();
    file_to(traces, name, sha256, p, &mut v).expect("a Vec grows");
    v
}

/// `table_to` into memory (small series and tests).
pub fn table_bytes(
    files: &[FileFeatures],
    origin: &Origin,
    p: &MrmParams,
    load: impl FnMut(usize) -> Result<Vec<Srm>>,
) -> Result<Vec<u8>> {
    let mut v = Vec::new();
    table_to(files, origin, p, load, &mut v)?;
    Ok(v)
}
