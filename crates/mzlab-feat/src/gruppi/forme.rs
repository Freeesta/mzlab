//! Shape of the time course of a row in one condition (from the means of the treated times): `single`, `rising`, `bell`, `decay`, `flat`.

use super::KinPt;
use mzlab_core::json::round;

/// `max / min` below this is `flat`.
const FLAT_RATIO: f64 = 1.5;

fn argmax(kin: &[KinPt]) -> usize {
    let mut m = 0;
    for (i, p) in kin.iter().enumerate() {
        if p.area > kin[m].area {
            m = i;
        }
    }
    m
}

/// With two times there is no inner maximum: `rising`, `decay` or `flat` by `FLAT_RATIO`.
fn two_points(a: f64, b: f64) -> &'static str {
    if b > FLAT_RATIO * a {
        "rising"
    } else if a > FLAT_RATIO * b {
        "decay"
    } else {
        "flat"
    }
}

/// `single` with one time, `rising`/`decay`/`flat` with two, else the shape over three or more times. A maximum inside the course followed by a fall of less than
/// `drop` of the maximum is a plateau and counts as `rising`.
pub(crate) fn shape(kin: &[KinPt], drop: f64) -> Option<&'static str> {
    match kin.len() {
        0 => return None,
        1 => return Some("single"),
        2 => return Some(two_points(kin[0].area, kin[1].area)),
        _ => {}
    }
    let hi = kin[argmax(kin)].area;
    let lo = kin.iter().map(|p| p.area).fold(f64::INFINITY, f64::min);
    if hi <= 0.0 {
        return None;
    }
    if lo > 0.0 && hi / lo < FLAT_RATIO {
        return Some("flat");
    }
    let m = argmax(kin);
    if m == kin.len() - 1 {
        return Some("rising");
    }
    if m == 0 {
        return Some("decay");
    }
    let after = kin[m + 1..]
        .iter()
        .map(|p| p.area)
        .fold(f64::INFINITY, f64::min);
    Some(if 1.0 - after / hi >= drop {
        "bell"
    } else {
        "rising"
    })
}

/// Time of the maximum mean area.
pub(crate) fn t_max(kin: &[KinPt]) -> Option<f64> {
    (!kin.is_empty()).then(|| kin[argmax(kin)].t)
}

/// First time (interpolated linearly between two points) at which the area reaches half of the maximum; only for `rising` and `bell`.
pub(crate) fn t_half_rise(shape: &str, kin: &[KinPt]) -> Option<f64> {
    if shape != "rising" && shape != "bell" {
        return None;
    }
    let half = 0.5 * kin[argmax(kin)].area;
    let i = kin.iter().position(|p| p.area >= half)?;
    if i == 0 {
        return Some(round(kin[0].t, 6));
    }
    let (a, b) = (&kin[i - 1], &kin[i]);
    let t = a.t + (half - a.area) / (b.area - a.area) * (b.t - a.t);
    Some(round(t, 6))
}
