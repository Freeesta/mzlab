//! Parent that saturates the ionisation: the area stays within +-`flat` of its median over at least `min_flat` consecutive times and then falls by
//! more than `drop` times. That is a non-linear response (source saturation), not a rate: the decay constant cannot be read from these areas.
//! Port of `kinetics.parent_saturation`; times in increasing order, dark / t0 included.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saturation {
    pub saturated: bool,
    /// Index range `[first, end)` of the plateau.
    pub plateau: (usize, usize),
    pub level: f64,
    pub drop: f64,
}

const NONE: Saturation = Saturation {
    saturated: false,
    plateau: (0, 0),
    level: 0.0,
    drop: 0.0,
};

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    crate::sortc::by(&mut s, f64::total_cmp);
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

pub fn saturation(a: &[f64], flat: f64, drop: f64, min_flat: usize) -> Saturation {
    let n = a.len();
    for i in 0..(n + 1).saturating_sub(min_flat) {
        for j in (i + min_flat..=n).rev() {
            let med = median(&a[i..j]);
            if med > 0.0 && a[i..j].iter().all(|v| (v / med - 1.0).abs() <= flat) {
                let tail_min = a[j..].iter().copied().fold(f64::INFINITY, f64::min);
                if j < n && med / tail_min.max(1e-300) > drop {
                    return Saturation {
                        saturated: true,
                        plateau: (i, j),
                        level: med,
                        drop: med / tail_min.max(1e-300),
                    };
                }
                break;
            }
        }
    }
    NONE
}

/// With the defaults of the Python reference: +-25 %, a fall over 10 times, at least 3 times.
pub fn parent_saturation(a: &[f64]) -> Saturation {
    saturation(a, 0.25, 10.0, 3)
}
