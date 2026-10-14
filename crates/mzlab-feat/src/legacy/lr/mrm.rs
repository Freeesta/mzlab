//! Integration of the MRM files of a low-resolution series (`TP_Mine/py/tpmine/mrm.py::analyse` without the calibration line): the area, time and
//! S/N of every transition in every file around the time of the quantifier, and the ratio of every qualifier to the quantifier checked against the
//! median of the standards. The program gives no concentrations.

use mzlab_core::idxread::ReadAt;
use mzlab_core::json::{round, Out};
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::Result;

use super::peak1d::{find_peak, Found};
use super::plan::FileLr;

/// Minimum S/N of a peak that counts as detected.
const SNR_MIN: f64 = 3.0;
const SMOOTH: usize = 5;
const MIN_POINTS: usize = 5;
/// Tolerance of a qualifier ratio against the standards (fraction).
pub const RATIO_TOL: f64 = 0.30;

/// One SRM chromatogram of a file.
#[derive(Debug, Clone, Default)]
pub struct Trace {
    pub q1: f64,
    pub q3: f64,
    pub ce: Option<f64>,
    pub name: String,
    pub rt: Vec<f64>,
    pub y: Vec<f64>,
}

/// One MRM file with its role and time.
#[derive(Debug, Clone, Default)]
pub struct MrmFile {
    pub name: String,
    pub label: String,
    pub kind_type: String,
    pub time: Option<f64>,
    pub traces: Vec<Trace>,
}

/// The SRM traces of the mzML `m` of file `f`; `scans_end` is where its last scan ends (0 without scans). The site and the app read the file the same way.
pub fn file_from_mzml<R: ReadAt>(f: &FileLr, m: &mut Mzml<R>, scans_end: u64) -> Result<MrmFile> {
    let mut traces = Vec::new();
    for c in m
        .chromatograms(scans_end)?
        .iter()
        .filter(|c| c.kind == "srm")
    {
        let (Some(q1), Some(q3)) = (c.q1, c.q3) else {
            continue;
        };
        let (rt, y) = m.chromatogram(c)?;
        traces.push(Trace {
            q1,
            q3,
            ce: c.ce,
            name: c.name.clone(),
            rt,
            y,
        });
    }
    Ok(MrmFile {
        name: f.name.clone(),
        label: f.label.clone(),
        kind_type: f.kind_type.clone(),
        time: f.time,
        traces,
    })
}

#[derive(Debug, Clone)]
struct Ion {
    area: f64,
    height: f64,
    rt: Option<f64>,
    snr: f64,
    detected: bool,
    left: Option<f64>,
    right: Option<f64>,
}

fn key(t: &Trace) -> String {
    format!("{}>{}", g(t.q1), g(t.q3))
}

/// `f"{v:g}"` for an m/z.
fn g(v: f64) -> String {
    super::labels::g(round(v, 4))
}

fn ion_of(f: Option<Found>) -> Ion {
    let det = f.is_some_and(|p| p.ok && p.snr >= SNR_MIN);
    let pick = |v: fn(&Found) -> f64| f.filter(|_| det).map_or(0.0, |p| v(&p));
    Ion {
        area: pick(|p| p.area),
        height: pick(|p| p.height),
        rt: f.filter(|_| det).map(|p| p.apex_rt),
        snr: f.map_or(0.0, |p| p.snr),
        detected: det,
        left: f.map(|p| p.left),
        right: f.map(|p| p.right),
    }
}

/// The transitions (quantifier first: those named `quant...`, then by Q1 and Q3) and the reference time of the quantifier.
fn plan(files: &[MrmFile], rt_tol: f64) -> Option<(Vec<Trace>, Option<f64>)> {
    let mut head: Vec<Trace> = Vec::new();
    for t in files.iter().flat_map(|f| &f.traces) {
        if !head.iter().any(|h| h.q1 == t.q1 && h.q3 == t.q3) {
            head.push(Trace {
                rt: vec![],
                y: vec![],
                ..t.clone()
            });
        }
    }
    if head.is_empty() {
        return None;
    }
    let quant = |t: &Trace| t.name.trim().to_lowercase().starts_with("quant");
    head.sort_by(|a, b| {
        (!quant(a), a.q1, a.q3)
            .partial_cmp(&(!quant(b), b.q1, b.q3))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let q = &head[0];
    let best = files
        .iter()
        .filter_map(|f| f.traces.iter().find(|t| t.q1 == q.q1 && t.q3 == q.q3))
        .filter_map(|t| find_peak(&t.rt, &t.y, None, rt_tol, SMOOTH, MIN_POINTS))
        .fold(None::<Found>, |b, p| match b {
            Some(x) if x.height >= p.height => Some(x),
            _ => Some(p),
        });
    Some((head, best.map(|p| p.apex_rt)))
}

fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    mzlab_core::sortc::by(v, |a, b| a.total_cmp(b));
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

fn ion_json(o: &mut Out, i: &Ion) {
    o.raw("{\"area\":")
        .num(round(i.area, 4))
        .raw(",\"height\":")
        .num(round(i.height, 4));
    o.raw(",\"rt\":")
        .opt(i.rt.map(|v| round(v, 4)))
        .raw(",\"snr\":")
        .num(round(i.snr, 4));
    o.raw(",\"detected\":")
        .raw(if i.detected { "true" } else { "false" });
    o.raw(",\"left\":")
        .opt(i.left.map(|v| round(v, 4)))
        .raw(",\"right\":")
        .opt(i.right.map(|v| round(v, 4)));
    o.raw("}");
}

/// The JSON of the MRM tables, or `None` when no file has a transition. `rt_tol` in minutes.
pub fn analyse(files: &[MrmFile], rt_tol: f64) -> Option<String> {
    let (head, ref_rt) = plan(files, rt_tol)?;
    let names: Vec<String> = head.iter().map(key).collect();
    let rows: Vec<Vec<Ion>> = files
        .iter()
        .map(|f| {
            head.iter()
                .map(|h| {
                    let tr = f.traces.iter().find(|t| t.q1 == h.q1 && t.q3 == h.q3);
                    ion_of(tr.zip(ref_rt).and_then(|(t, c)| {
                        find_peak(&t.rt, &t.y, Some(c), rt_tol, SMOOTH, MIN_POINTS)
                    }))
                })
                .collect()
        })
        .collect();
    let ratio =
        |r: &[Ion], n: usize| (r[0].area > 0.0 && r[n].detected).then(|| r[n].area / r[0].area);
    let std: Vec<usize> = (0..files.len())
        .filter(|&k| files[k].kind_type == "standard" && rows[k][0].detected)
        .collect();
    let refs: Vec<Option<f64>> = (1..names.len())
        .map(|n| {
            median(
                &mut std
                    .iter()
                    .filter_map(|&k| ratio(&rows[k], n))
                    .collect::<Vec<f64>>(),
            )
        })
        .collect();
    Some(emit(&head, &names, ref_rt, files, &rows, &refs, &ratio))
}

/// Serialise the per-transition integration table.
#[allow(clippy::too_many_arguments)]
fn emit(
    head: &[Trace],
    names: &[String],
    ref_rt: Option<f64>,
    files: &[MrmFile],
    rows: &[Vec<Ion>],
    refs: &[Option<f64>],
    ratio: &dyn Fn(&[Ion], usize) -> Option<f64>,
) -> String {
    let mut o = Out::default();
    o.raw("{\"transitions\":[");
    for (i, h) in head.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"q1\":")
            .num(h.q1)
            .raw(",\"q3\":")
            .num(h.q3);
        o.raw(",\"ce\":")
            .opt(h.ce)
            .raw(",\"role\":")
            .str(h.name.trim())
            .raw(",\"name\":")
            .str(&names[i])
            .raw("}");
    }
    o.raw("],\"quantifier\":")
        .str(&names[0])
        .raw(",\"ref_rt\":")
        .opt(ref_rt.map(|v| round(v, 4)));
    o.raw(",\"rows\":[");
    for (k, f) in files.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"file\":")
            .str(&f.name)
            .raw(",\"label\":")
            .str(&f.label);
        o.raw(",\"type\":")
            .str(&f.kind_type)
            .raw(",\"time\":")
            .opt(f.time)
            .raw(",\"ion\":{");
        for (i, n) in names.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(n).raw(":");
            ion_json(&mut o, &rows[k][i]);
        }
        o.raw("},\"ratios\":{");
        let mut ok = Vec::new();
        for n in 1..names.len() {
            let v = ratio(&rows[k], n);
            o.raw(if n > 1 { "," } else { "" })
                .str(&names[n])
                .raw(":")
                .opt(v.map(|x| round(x, 4)));
            ok.push(
                refs[n - 1]
                    .zip(v)
                    .map(|(r, v)| (v - r).abs() / r <= RATIO_TOL),
            );
        }
        let all = if ok.is_empty() || ok.iter().any(Option::is_none) {
            "null"
        } else if ok.iter().all(|x| *x == Some(true)) {
            "true"
        } else {
            "false"
        };
        o.raw("},\"ratio_ok\":").raw(all).raw("}");
    }
    o.raw("],\"ratio_ref\":{");
    for (n, r) in refs.iter().enumerate() {
        o.raw(if n > 0 { "," } else { "" })
            .str(&names[n + 1])
            .raw(":")
            .opt(r.map(|x| round(x, 4)));
    }
    o.raw("},\"ratio_tol\":")
        .num(RATIO_TOL)
        .raw(",\"calibration\":null}");
    o.0
}
