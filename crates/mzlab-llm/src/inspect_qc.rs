//! The series sections of `ispezione/1`: `qc/1` (the signals the QC files share, how their RT and area move with the order of injection, and the
//! `rt_gap` rule R-rt-1 of `docs/studi/assistente-dati/03-parametri.md`) and the blank check (signals of a blank that no other file has). Plain
//! arithmetic on the strongest traces the engine already found; the model reads these numbers and never computes them. Used by `inspect`.

use mzlab_core::json::Value;
use mzlab_core::sortc;

use crate::inspect::{median, n};

/// RT window (min) in which a signal of one QC is looked for in the next.
const QC_RT_WINDOW: f64 = 0.5;
/// RT window (min) in which a signal of a blank is looked for in the samples.
const BLANK_RT_WINDOW: f64 = 0.2;
/// m/z tolerance at unit resolution (Da) and at high resolution (ppm).
const TOL_DA: f64 = 0.5;
const TOL_PPM: f64 = 10.0;
/// The most reference signals kept, and signals listed per blank.
const REFERENCES: usize = 10;
const BLANK_SIGNALS: usize = 5;
/// Scale of the median absolute deviation to a standard deviation.
const MAD_SD: f64 = 1.4826;

/// What the QC check needs of a file: its role, injection order and strongest traces `[m/z, RT, area]`.
pub(crate) struct QcFile {
    pub alias: String,
    pub role: String,
    pub order: Option<f64>,
    pub signals: Vec<[f64; 3]>,
    pub fwhm: Option<f64>,
}

fn same_mz(a: f64, b: f64, high: bool) -> bool {
    (a - b).abs() <= if high { a * TOL_PPM * 1e-6 } else { TOL_DA }
}

/// The signal of `from` that matches `(mz, rt)` best (closest in RT), if any.
fn find(from: &[[f64; 3]], mz: f64, rt: f64, window: f64, high: bool) -> Option<[f64; 3]> {
    let mut best: Option<[f64; 3]> = None;
    for s in from {
        let d = (s[1] - rt).abs();
        if same_mz(mz, s[0], high) && d <= window && best.is_none_or(|b| d < (b[1] - rt).abs()) {
            best = Some(*s);
        }
    }
    best
}

fn mad_sd(xs: &[f64]) -> f64 {
    let m = median(&mut xs.to_vec()).unwrap_or(0.0);
    MAD_SD * median(&mut xs.iter().map(|x| (x - m).abs()).collect::<Vec<_>>()).unwrap_or(0.0)
}

/// Slope of the least-squares line through `(x, y)`; `None` when all `x` are equal.
fn slope(x: &[f64], y: &[f64]) -> Option<f64> {
    let k = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / k, y.iter().sum::<f64>() / k);
    let sxx: f64 = x.iter().map(|v| (v - mx) * (v - mx)).sum();
    (sxx > 0.0).then(|| {
        x.iter()
            .zip(y)
            .map(|(a, b)| (a - mx) * (b - my))
            .sum::<f64>()
            / sxx
    })
}

fn obj(items: Vec<(&str, Value)>) -> Value {
    Value::Obj(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// `qc/1` for the QC files (two or more with signals) and the blank check; either is `None` when its inputs are missing.
pub(crate) fn sections(files: &[&QcFile], high: bool) -> (Option<Value>, Option<Value>) {
    (qc(files, high), blanks(files, high))
}

fn qc(files: &[&QcFile], high: bool) -> Option<Value> {
    let mut qcs: Vec<(f64, &QcFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.role == "qc" && !f.signals.is_empty())
        .map(|(k, f)| (f.order.unwrap_or(k as f64 + 1.0), *f))
        .collect();
    if qcs.len() < 2 {
        return None;
    }
    let listed = qcs.iter().any(|(_, f)| f.order.is_none());
    sortc::slice_by(&mut qcs, |a, b| a.0.total_cmp(&b.0));
    let first = qcs[0].1;
    let mut strongest = first.signals.clone();
    sortc::slice_by(&mut strongest, |a, b| b[2].total_cmp(&a[2]));
    // reference signals: those of the first QC found in every other QC; per signal the (RT, area) in each QC
    let mut common: Vec<(f64, Vec<[f64; 3]>)> = Vec::new();
    for s in strongest.iter().take(20) {
        let rows: Option<Vec<[f64; 3]>> = qcs
            .iter()
            .map(|(_, f)| find(&f.signals, s[0], s[1], QC_RT_WINDOW, high))
            .collect();
        if let Some(rows) = rows {
            let mut areas: Vec<f64> = rows.iter().map(|r| r[2]).collect();
            common.push((median(&mut areas).unwrap_or(0.0), rows));
        }
    }
    sortc::slice_by(&mut common, |a, b| b.0.total_cmp(&a.0));
    common.truncate(REFERENCES);
    let order_from = if listed {
        "list_position"
    } else {
        "sample_table"
    };
    let mut head = vec![
        ("schema", Value::Str("qc/1".into())),
        (
            "qc",
            Value::Arr(
                qcs.iter()
                    .map(|(_, f)| Value::Str(f.alias.clone()))
                    .collect(),
            ),
        ),
        (
            "order",
            Value::Arr(qcs.iter().map(|(o, _)| Value::Num(*o)).collect()),
        ),
        ("order_from", Value::Str(order_from.into())),
        ("common_signals", Value::Num(common.len() as f64)),
    ];
    if common.is_empty() {
        return Some(obj(head));
    }
    let orders: Vec<f64> = qcs.iter().map(|(o, _)| *o).collect();
    let last = qcs.len() - 1;
    let (offsets, first_last, robust_sd, rt) = rt_drift(&common, &orders);
    let area = area_drift(&common, last);
    let refs = common.iter().map(|(_, r)| {
        let mut rts: Vec<f64> = r.iter().map(|x| x[1]).collect();
        obj(vec![
            ("mz", n(r[0][0])),
            ("rt", n(median(&mut rts).unwrap_or(0.0))),
        ])
    });
    head.push(("signals", Value::Arr(refs.collect())));
    head.push(("rt_drift_min", obj(rt)));
    head.push(("area_drift", obj(area)));
    proposal(&mut head, &qcs, &offsets, first_last, robust_sd);
    head.push((
        "confidence",
        Value::Str(if qcs.len() >= 3 { "high" } else { "medium" }.into()),
    ));
    Some(obj(head))
}

/// RT shift of the reference signals against the first QC: per QC (median offsets), first to last (median), its robust spread, and the section.
fn rt_drift(
    common: &[(f64, Vec<[f64; 3]>)],
    orders: &[f64],
) -> (Vec<f64>, f64, f64, Vec<(&'static str, Value)>) {
    let last = orders.len() - 1;
    let drift: Vec<f64> = common.iter().map(|(_, r)| r[last][1] - r[0][1]).collect();
    let offsets: Vec<f64> = (0..orders.len())
        .map(|q| {
            median(
                &mut common
                    .iter()
                    .map(|(_, r)| r[q][1] - r[0][1])
                    .collect::<Vec<_>>(),
            )
            .unwrap_or(0.0)
        })
        .collect();
    let first_last = median(&mut drift.clone()).unwrap_or(0.0);
    let robust_sd = mad_sd(&drift);
    let mut rt = vec![("first_last", n(first_last)), ("robust_sd", n(robust_sd))];
    if let Some(sl) = slope(orders, &offsets) {
        rt.insert(1, ("slope_per_inj", n(sl)));
    }
    (offsets, first_last, robust_sd, rt)
}

/// How the area of the reference signals moves from the first QC to the last (median change) and between all of them (median CV).
fn area_drift(common: &[(f64, Vec<[f64; 3]>)], last: usize) -> Vec<(&'static str, Value)> {
    let ratios: Vec<f64> = common
        .iter()
        .filter(|(_, r)| r[0][2] > 0.0)
        .map(|(_, r)| r[last][2] / r[0][2])
        .collect();
    let cvs: Vec<f64> = common
        .iter()
        .map(|(_, r)| {
            let a: Vec<f64> = r.iter().map(|x| x[2]).collect();
            let m = a.iter().sum::<f64>() / a.len() as f64;
            let var = a.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (a.len() as f64 - 1.0);
            if m > 0.0 {
                var.sqrt() / m * 100.0
            } else {
                0.0
            }
        })
        .collect();
    let mut area = Vec::new();
    if let Some(r) = median(&mut ratios.clone()) {
        area.push(("first_last_pct", n((r - 1.0) * 100.0)));
    }
    if let Some(c) = median(&mut cvs.clone()) {
        area.push(("cv_pct", n(c)));
    }
    area
}

/// The `rt_gap` rule R-rt-1 and the `rt_warp` decision, or the missing peak width.
fn proposal(
    head: &mut Vec<(&'static str, Value)>,
    qcs: &[(f64, &QcFile)],
    offsets: &[f64],
    first_last: f64,
    robust_sd: f64,
) {
    let mut widths: Vec<f64> = qcs.iter().filter_map(|(_, f)| f.fwhm).collect();
    match median(&mut widths) {
        Some(w) => {
            head.push(("fwhm_median_min", n(w)));
            // R-rt-1: three times the robust drift, never below the median peak width
            let monotone =
                offsets.windows(2).all(|p| p[1] > p[0]) || offsets.windows(2).all(|p| p[1] < p[0]);
            let warp =
                first_last.abs() > 2.0 * w || (monotone && qcs.len() >= 3 && first_last.abs() > w);
            head.push((
                "proposal",
                obj(vec![
                    ("features.align.rt_gap", n((3.0 * robust_sd).max(w))),
                    ("features.align.rt_warp", Value::Bool(warp)),
                ]),
            ));
        }
        None => head.push((
            "missing",
            Value::Arr(vec![Value::Str("fwhm_median_min".into())]),
        )),
    }
}

/// Per blank, its strongest signals and whether any other file has a signal at the same m/z and RT.
fn blanks(files: &[&QcFile], high: bool) -> Option<Value> {
    let others: Vec<&&QcFile> = files
        .iter()
        .filter(|f| f.role != "blank" && !f.signals.is_empty())
        .collect();
    if others.is_empty() {
        return None;
    }
    let list: Vec<Value> = files
        .iter()
        .filter(|f| f.role == "blank" && !f.signals.is_empty())
        .map(|f| {
            let mut s = f.signals.clone();
            sortc::slice_by(&mut s, |a, b| b[2].total_cmp(&a[2]));
            let rows = s.iter().take(BLANK_SIGNALS).map(|x| {
                let present = others
                    .iter()
                    .any(|o| find(&o.signals, x[0], x[1], BLANK_RT_WINDOW, high).is_some());
                obj(vec![
                    ("mz", n(x[0])),
                    ("rt", n(x[1])),
                    ("area", n(x[2])),
                    ("in_samples", Value::Bool(present)),
                ])
            });
            obj(vec![
                ("alias", Value::Str(f.alias.clone())),
                ("signals", Value::Arr(rows.collect())),
            ])
        })
        .collect();
    (!list.is_empty()).then_some(Value::Arr(list))
}
