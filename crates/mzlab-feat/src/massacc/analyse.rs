//! From the traces of one file to its numbers: the error of each reference in ppm, the robust offset and spread, the fit of the chosen model and what is left after it, the standards and
//! the warnings. Pure arithmetic over `FileMeasure`; used by `massacc::run`.

use mzlab_core::json::Value;

use super::fit::{fit, fit_down, Fit, Model, Obs};
use super::istd::{of_trace, Istd};
use super::measure::{FileMeasure, Trace};
use super::ols::median;
use super::refs::{RefIon, Role};
use super::{FileSpec, Params};

/// The scans of a reference that count as observations of a model with an RT term (the most intense ones).
const RT_SCANS: usize = 64;

/// One reference ion as seen in one file.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub r: usize,
    /// RT of the most intense scan.
    pub rt: f64,
    pub err_ppm: f64,
    pub intensity: f64,
    /// MS1 scans the ion was seen in (inside the search window, over the intensity floor).
    pub scans: usize,
    pub outlier: bool,
}

/// Everything one file gave.
#[derive(Clone, Debug)]
pub struct FileResult {
    pub name: String,
    pub sha256: String,
    pub istd_file: bool,
    pub pol: i8,
    pub n_scans: usize,
    pub searched: usize,
    pub assessed: bool,
    pub offset: Option<f64>,
    pub spread: Option<f64>,
    pub residual: Option<f64>,
    /// The model that was fitted (the chosen one, or the simpler one the references could tell apart).
    pub model: Model,
    pub coef: [f64; 3],
    pub fitted: bool,
    pub points: Vec<Point>,
    pub istds: Vec<Istd>,
    pub warnings: Vec<&'static str>,
    pub tolerance_ppm: Option<f64>,
    pub lock_mass: Option<Value>,
}

/// Error of a trace in ppm: the intensity-weighted m/z against the theoretical one.
fn trace_error(t: &Trace, theo: f64) -> f64 {
    let (sw, sm) = t
        .inten
        .iter()
        .zip(&t.mz)
        .fold((0.0, 0.0), |(a, b), (w, m)| (a + w, b + w * m));
    (sm / sw - theo) / theo * 1e6
}

/// Index of the most intense scan of a trace (the first on a tie).
fn apex(t: &Trace) -> usize {
    (0..t.inten.len())
        .max_by(|&i, &j| t.inten[i].total_cmp(&t.inten[j]).then(j.cmp(&i)))
        .unwrap_or(0)
}

/// The observations of the model with an RT term: the `RT_SCANS` strongest scans of each reference, each reference weighing as much as any other. The second list is the reference of each.
fn scan_obs(traces: &[Trace], refs: &[RefIon]) -> (Vec<Obs>, Vec<usize>) {
    let (mut obs, mut owner) = (Vec::new(), Vec::new());
    for (k, t) in traces.iter().enumerate() {
        let mut order: Vec<usize> = (0..t.rt.len()).collect();
        crate::sortc::by(&mut order, |&i, &j| {
            t.inten[j].total_cmp(&t.inten[i]).then(i.cmp(&j))
        });
        order.truncate(RT_SCANS);
        crate::sortc::ord(&mut order);
        let theo = refs[t.r].mz;
        for &i in &order {
            obs.push(Obs {
                mz: theo,
                rt: t.rt[i],
                err: (t.mz[i] - theo) / theo * 1e6,
                w: 1.0 / order.len() as f64,
            });
            owner.push(k);
        }
    }
    (obs, owner)
}

pub fn analyse_file(spec: &FileSpec, m: &FileMeasure, refs: &[RefIon], p: &Params) -> FileResult {
    let mut res = FileResult {
        name: spec.name.clone(),
        sha256: m.sha256.clone(),
        istd_file: spec.istd,
        pol: m.pol,
        n_scans: m.n_scans,
        searched: m.searched,
        assessed: false,
        offset: None,
        spread: None,
        residual: None,
        model: Model::None,
        coef: [0.0; 3],
        fitted: false,
        points: Vec::new(),
        istds: Vec::new(),
        warnings: Vec::new(),
        tolerance_ppm: None,
        lock_mass: m.lock_mass.clone(),
    };
    let obs: Vec<Obs> = m
        .traces
        .iter()
        .map(|t| {
            let a = apex(t);
            Obs {
                mz: refs[t.r].mz,
                rt: t.rt[a],
                err: trace_error(t, refs[t.r].mz),
                w: 1.0,
            }
        })
        .collect();
    res.points = m
        .traces
        .iter()
        .zip(&obs)
        .map(|(t, o)| Point {
            r: t.r,
            rt: o.rt,
            err_ppm: o.err,
            intensity: t.inten[apex(t)],
            scans: t.rt.len(),
            outlier: false,
        })
        .collect();
    res.istds = m
        .traces
        .iter()
        .filter(|t| match refs[t.r].role {
            Role::Reference => false,
            Role::Istd => spec.istd,
            Role::Standard => true,
        })
        .filter_map(of_trace)
        .collect();
    if obs.len() < p.min_refs.max(1) {
        res.warnings.push("warn.qc.mass.few_refs");
    } else {
        res.assessed = true;
        describe(&mut res, m, refs, &obs, p);
    }
    if res.istds.iter().any(|s| {
        refs[s.r]
            .rt
            .is_some_and(|d| (s.rt - d).abs() > p.warn_is_rt_min)
    }) {
        res.warnings.push("warn.qc.mass.is_rt");
    }
    res
}

/// The offset, the spread, the fit of the model and the residual of an assessed file.
fn describe(res: &mut FileResult, m: &FileMeasure, refs: &[RefIon], obs: &[Obs], p: &Params) {
    let flat = fit(Model::Offset, p.robust, obs);
    let (offset, spread, flat_in) = match &flat {
        Some(f) => (f.coef[0], f.scale, f.inlier.clone()),
        None => {
            let e: Vec<f64> = obs.iter().map(|o| o.err).collect();
            (median(&e), 0.0, vec![true; obs.len()])
        }
    };
    res.offset = Some(offset);
    res.spread = Some(spread);
    let rms = |r: Vec<f64>| (r.iter().map(|x| x * x).sum::<f64>() / r.len().max(1) as f64).sqrt();
    let (mut inlier, mut residual) = (flat_in.clone(), None);
    if p.model != Model::None {
        let fitted: Option<(Fit, Vec<bool>)> = if p.model.uses_rt() {
            let (scans, owner) = scan_obs(&m.traces, refs);
            fit_down(p.model, p.robust, &scans).map(|f| {
                // a reference is an outlier when most of its scans are
                let mut votes = vec![(0usize, 0usize); obs.len()];
                for (k, &o) in owner.iter().enumerate() {
                    votes[o].0 += usize::from(!f.inlier[k]);
                    votes[o].1 += 1;
                }
                let keep = votes.iter().map(|&(bad, all)| 2 * bad <= all).collect();
                (f, keep)
            })
        } else {
            fit_down(p.model, p.robust, obs).map(|f| {
                let keep = f.inlier.clone();
                (f, keep)
            })
        };
        if let Some((f, keep)) = fitted {
            res.model = f.model;
            res.coef = f.coef;
            res.fitted = true;
            let r: Vec<f64> = obs
                .iter()
                .zip(&keep)
                .filter(|&(_, &k)| k)
                .map(|(o, _)| o.err - f.model.predict(&f.coef, o.mz, o.rt))
                .collect();
            residual = Some(rms(r));
            inlier = keep;
        }
    }
    let residual = residual.unwrap_or_else(|| {
        rms(obs
            .iter()
            .zip(&flat_in)
            .filter(|&(_, &k)| k)
            .map(|(o, _)| o.err)
            .collect())
    });
    res.residual = Some(residual);
    for (pt, keep) in res.points.iter_mut().zip(&inlier) {
        pt.outlier = !keep;
    }
    let (bias, sd) = if res.fitted {
        (0.0, residual)
    } else {
        (offset, spread)
    };
    res.tolerance_ppm = Some((bias.abs() + 3.0 * sd).ceil().max(1.0));
    if offset.abs() > p.warn_offset_ppm {
        res.warnings.push("warn.qc.mass.offset");
    }
    if spread > p.warn_spread_ppm {
        res.warnings.push("warn.qc.mass.spread");
    }
}
