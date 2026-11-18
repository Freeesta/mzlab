//! What the clamp of each baseline end of `peakq_lrauto` did, and its result as the JSON that the page (mzlab-wasm) and `mzlab-cli peak-auto-lr`
//! both print, the same bytes. A child module of `peakq_lrauto`, split off to keep that file under the size limit.

use super::{End, LrAuto, Split};

/// What the clamp of a baseline end changed. The line runs only between the two ends of a peak, and an end is never below zero (the zero line of the
/// trace shown, the subtracted background of a subtracted one): a background under zero is raised to it, and an end above the raw trace is lowered onto it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clamp {
    /// The end is the engine's level as found.
    None,
    /// The level was above the raw trace and was lowered onto it.
    Trace,
    /// The level was below zero and was raised to zero.
    Zero,
}

/// A baseline level `v` at a point of raw value `y`: never below zero; never above `y` unless `y` itself is below zero (then the end stays at zero).
pub(super) fn clamp(v: f64, y: f64) -> (f64, Clamp) {
    if v < 0.0 {
        (0.0, Clamp::Zero)
    } else if v > y {
        (y.max(0.0), Clamp::Trace)
    } else {
        (v, Clamp::None)
    }
}

fn clamp_name(c: Clamp) -> &'static str {
    match c {
        Clamp::None => "none",
        Clamp::Trace => "trace",
        Clamp::Zero => "zero",
    }
}

fn end_name(e: End) -> &'static str {
    match e {
        End::Background => "background",
        End::Valley => "valley",
        End::Cap => "cap",
    }
}

/// The result as the JSON the page (`lr_auto_integrate` of mzlab-wasm) and `mzlab-cli peak-auto-lr` both print, the same bytes: the parameters used
/// (`half_auto` true when the half window was chosen here, `baseline` always "engine": this route never reads the page's baseline setting) and every
/// peak (times in min, baseline in counts with what its clamp did, areas in counts x min, never negative, `below` the part cut off under the line).
pub fn lr_auto_json(rt: &[f64], r: &LrAuto) -> String {
    let p = &r.params;
    let head = format!(
        "{{\"params\":{{\"half\":{},\"half_auto\":{},\"valley\":{},\"sn\":{},\"min_pts\":{},\"despike\":{},\"split\":\"{}\",\"baseline\":\"engine\",\"noise\":{}}}",
        r.half, r.half_auto, p.valley, p.snr, p.min_pts, p.despike, if p.split == Split::Drop { "drop" } else { "valley" }, r.noise
    );
    let rows: Vec<String> = r
        .peaks
        .iter()
        .map(|k| {
            format!(
                "{{\"lo\":{},\"hi\":{},\"t0\":{},\"t1\":{},\"rt\":{},\"b0\":{},\"b1\":{},\"area\":{},\"below\":{},\"area_zero\":{},\"height\":{},\"sn\":{},\"end0\":\"{}\",\"end1\":\"{}\",\"clamp0\":\"{}\",\"clamp1\":\"{}\"}}",
                k.lo, k.hi, rt[k.lo], rt[k.hi], rt[k.apex], k.base_lo, k.base_hi, k.area, k.below, k.area_zero, k.height, k.sn, end_name(k.end_lo), end_name(k.end_hi),
                clamp_name(k.clamp_lo), clamp_name(k.clamp_hi)
            )
        })
        .collect();
    format!("{head},\"peaks\":[{}]}}", rows.join(","))
}
