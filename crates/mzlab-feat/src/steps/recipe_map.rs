//! The one mapping between the parameters of the feature descriptors (`features.detect`, `features.align`, `features.gapfill`, `features.ms2_link`) and
//! the fields of `Recipe`: `apply` writes values onto a recipe, `values` reads them back. Used by the native runner of plans (`mzlab-cli`) and by
//! `tests/steps.rs`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

use crate::axis::MassAxis;
use crate::centwave::{CentWave, Peak};
use crate::recipe::{Mode, Recipe};

fn bad(step: &str, name: &str) -> Error {
    Error::new("err.flow.param.type")
        .with("instance", step)
        .with("param", name)
}

/// Writes `params` (descriptor names, any order; `algorithm` and `axis` are applied before the rest; `algorithm` is `centwave` or `masstrace` for `features.detect`) of `step` onto `r`. An unknown name is `err.flow.param.unknown`.
pub fn apply(r: &mut Recipe, step: &str, params: &[(String, Value)]) -> Result<()> {
    let mut sorted: Vec<&(String, Value)> = params.iter().collect();
    mzlab_core::sortc::slice_by_key(&mut sorted, |(k, _)| {
        !matches!(k.as_str(), "algorithm" | "axis")
    });
    for (k, v) in sorted {
        set(r, step, k, v)?;
    }
    Ok(())
}

/// The peak model of `features.detect`: the algorithm, and the parameters that only the wavelet model has.
/// Detect parameters that belong to the peak model.
const PEAK_KEYS: [&str; 6] = [
    "algorithm",
    "ppm",
    "peak_width_s",
    "width_s",
    "snr",
    "broad_peaks",
];

/// `[min, max]` minutes (`Some(None)` for null = all rows); `None` when the value is not a range.
fn rt_range(v: &Value) -> Option<Option<(f64, f64)>> {
    if matches!(v, Value::Null) {
        return Some(None);
    }
    let (a, b) = (v.at(0)?.f64()?, v.at(1)?.f64()?);
    (a.is_finite() && b.is_finite() && b >= a).then_some(Some((a, b)))
}

fn set_peak(r: &mut Recipe, step: &str, k: &str, v: &Value) -> Result<()> {
    if k == "algorithm" {
        r.detect.peak = match v.str() {
            Some("centwave") => Peak::CentWave(CentWave::default()),
            Some("masstrace") => Peak::Minima,
            _ => return Err(bad(step, k)),
        };
        return Ok(());
    }
    let Peak::CentWave(c) = &mut r.detect.peak else {
        return Err(Error::new("err.flow.param.unknown").with("param", k));
    };
    if k == "broad_peaks" {
        c.broad.enabled = v.bool().ok_or_else(|| bad(step, k))?;
        return Ok(());
    }
    let num = v.f64().ok_or_else(|| bad(step, k));
    match k {
        "ppm" => c.ppm = num?,
        "snr" => c.snr = num?,
        "peak_width_s" | "width_s" => {
            match (v.at(0).and_then(Value::f64), v.at(1).and_then(Value::f64)) {
                (Some(a), Some(b)) if a.is_finite() && b.is_finite() && b >= a => {
                    c.width_s = (a, b)
                }
                _ => {
                    if let Some(hi) = v.f64() {
                        if hi.is_finite() && hi >= c.width_s.0 {
                            c.width_s.1 = hi;
                        } else {
                            return Err(bad(step, k));
                        }
                    } else {
                        return Err(bad(step, k));
                    }
                }
            }
        }
        _ => return Err(bad(step, k)),
    }
    Ok(())
}

fn set(r: &mut Recipe, step: &str, k: &str, v: &Value) -> Result<()> {
    let num = || v.f64().ok_or_else(|| bad(step, k));
    let int = || num().map(|x| x as usize);
    match (step, k) {
        ("features.detect", "polarity") => {
            r.detect.polarity = match num()? as i32 {
                0 => None,
                p => Some(p as i8),
            }
        }
        ("features.detect", "floor") => r.detect.floor = num()?,
        ("features.detect", "min_height") => r.detect.min_height = num()?,
        ("features.detect", "axis") => {
            r.detect.axis = match (v.str(), r.detect.axis) {
                (Some("ppm"), _) => MassAxis::Ppm,
                (Some("Da"), MassAxis::Da { defect }) => MassAxis::Da { defect },
                (Some("Da"), MassAxis::Ppm) => MassAxis::Da { defect: 0.0 },
                _ => return Err(bad(step, k)),
            }
        }
        // the defect only exists on the unit axis
        ("features.detect", "mass_defect") => {
            if let MassAxis::Da { defect } = &mut r.detect.axis {
                *defect = num()?;
            }
        }
        ("features.detect", "rt_range") => r.rt_range = rt_range(v).ok_or_else(|| bad(step, k))?,
        ("features.detect", "budget") => r.detect.budget = int()?,
        ("features.detect", "min_points") => r.detect.min_points = num()? as u32,
        ("features.detect", "max_gap") => r.detect.max_gap = num()? as u32,
        ("features.detect", "long_trace") => r.detect.long_trace = num()? as u32,
        ("features.detect", "merge_overlap") => r.detect.merge_overlap = num()?,
        ("features.detect", _) if PEAK_KEYS.contains(&k) => set_peak(r, step, k, v)?,
        ("features.align", "mode") => {
            r.mode = match v.str() {
                Some("family") => Mode::Family,
                Some("ion") => Mode::Ion,
                _ => return Err(bad(step, k)),
            }
        }
        ("features.align", "ppm") => r.align.ion.ppm = num()?,
        ("features.align", "rt_gap") => r.align.ion.rt_gap = num()?,
        ("features.align", "max_ppm_span") => r.align.ion.max_ppm_span = num()?,
        ("features.align", "drop_artefacts") => {
            r.align.ion.drop_artefacts = v.bool().ok_or_else(|| bad(step, k))?
        }
        ("features.align", "split_gap") => r.align.ion.split_gap = Some(num()?),
        ("features.align", "rt_warp") => {
            r.align.warp.method = v
                .str()
                .and_then(crate::rtwarp::Method::parse)
                .ok_or_else(|| bad(step, k))?
        }
        ("features.align", "rt_coarse_gap") => r.align.warp.coarse_gap = num()?,
        ("features.align", "rt_span") => r.align.warp.span = num()?,
        ("features.align", "rt_inlier") => r.align.warp.inlier = num()?,
        ("features.align", "link_rt") => r.align.link_rt = num()?,
        ("features.align", "max_span") => r.align.max_span = num()?,
        ("features.align", "min_dependent") => r.align.min_dependent = int()? as u32,
        ("features.align", "weak_files") => r.weak_files = int()?,
        ("features.gapfill", "floor") => r.fill.floor = num()?,
        ("features.gapfill", "ppm") => r.fill.ppm = num()?,
        ("features.gapfill", "rt_tol") => r.fill.rt_tol = num()?,
        ("features.gapfill", "min_points") => r.fill.min_points = int()?,
        ("features.gapfill", "max_gap") => r.fill.max_gap = int()?,
        ("features.gapfill", "long_trace") => r.fill.long_trace = int()?,
        ("features.ms2_link", "default_half_window") => r.steps.ms2.default_half_window = num()?,
        ("features.ms2_link", "tol_ppm") => r.steps.ms2.tol_ppm = num()?,
        ("features.ms2_link", "weak") => r.steps.ms2.weak = num()? as f32,
        _ => {
            return Err(Error::new("err.flow.param.unknown")
                .with("instance", step)
                .with("param", k))
        }
    }
    Ok(())
}

/// The values `r` holds for every parameter of `step` that `apply` knows (sorted by name; `mass_defect` only on the unit axis).
pub fn values(r: &Recipe, step: &str) -> Vec<(String, Value)> {
    let n = |x: f64| Value::Num(x);
    let mut v: Vec<(&str, Value)> = match step {
        "features.detect" => {
            let mut v = vec![
                ("polarity", n(r.detect.polarity.map_or(0.0, f64::from))),
                ("floor", n(r.detect.floor)),
                ("min_height", n(r.detect.min_height)),
                ("budget", n(r.detect.budget as f64)),
                ("min_points", n(f64::from(r.detect.min_points))),
                ("max_gap", n(f64::from(r.detect.max_gap))),
                ("long_trace", n(f64::from(r.detect.long_trace))),
                ("merge_overlap", n(r.detect.merge_overlap)),
            ];
            if let Peak::CentWave(c) = &r.detect.peak {
                let (a, b) = c.width_s;
                v.push(("ppm", n(c.ppm)));
                v.push(("peak_width_s", Value::Arr(vec![n(a), n(b)])));
                v.push(("snr", n(c.snr)));
                v.push(("broad_peaks", Value::Bool(c.broad.enabled)));
            }
            if let Some((a, b)) = r.rt_range {
                v.push(("rt_range", Value::Arr(vec![n(a), n(b)])));
            }
            match r.detect.axis {
                MassAxis::Ppm => v.push(("axis", Value::Str("ppm".into()))),
                MassAxis::Da { defect } => {
                    v.push(("axis", Value::Str("Da".into())));
                    v.push(("mass_defect", n(defect)));
                }
            }
            v
        }
        "features.align" => vec![
            (
                "mode",
                Value::Str(if r.mode == Mode::Ion { "ion" } else { "family" }.into()),
            ),
            ("ppm", n(r.align.ion.ppm)),
            ("rt_gap", n(r.align.ion.rt_gap)),
            ("max_ppm_span", n(r.align.ion.max_ppm_span)),
            ("drop_artefacts", Value::Bool(r.align.ion.drop_artefacts)),
            ("split_gap", r.align.ion.split_gap.map_or(Value::Null, n)),
            ("rt_warp", Value::Str(r.align.warp.method.name().into())),
            ("rt_coarse_gap", n(r.align.warp.coarse_gap)),
            ("rt_span", n(r.align.warp.span)),
            ("rt_inlier", n(r.align.warp.inlier)),
            ("link_rt", n(r.align.link_rt)),
            ("max_span", n(r.align.max_span)),
            ("min_dependent", n(f64::from(r.align.min_dependent))),
            ("weak_files", n(r.weak_files as f64)),
        ],
        "features.gapfill" => vec![
            ("floor", n(r.fill.floor)),
            ("ppm", n(r.fill.ppm)),
            ("rt_tol", n(r.fill.rt_tol)),
            ("min_points", n(r.fill.min_points as f64)),
            ("max_gap", n(r.fill.max_gap as f64)),
            ("long_trace", n(r.fill.long_trace as f64)),
        ],
        "features.ms2_link" => vec![
            ("default_half_window", n(r.steps.ms2.default_half_window)),
            ("tol_ppm", n(r.steps.ms2.tol_ppm)),
            ("weak", n(f64::from(r.steps.ms2.weak))),
        ],
        _ => Vec::new(),
    };
    mzlab_core::sortc::slice_by(&mut v, |a, b| a.0.cmp(b.0));
    v.into_iter().map(|(k, x)| (k.to_string(), x)).collect()
}
