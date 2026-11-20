//! Sine, cosine, arctangent and 2D vector helpers written with +, -, *, / and sqrt only (all exactly rounded by IEEE 754), so the
//! layout gives the same bits in the native build and in the `.wasm` (the platform libm differs in the last digit).

pub type V = [f64; 2];

pub const PI: f64 = std::f64::consts::PI;
pub const TAU: f64 = 2.0 * PI;

/// `(sin x, cos x)` by Taylor series after reduction to [-pi, pi].
pub fn sincos(x: f64) -> (f64, f64) {
    let k = (x / TAU + 0.5).floor();
    let x = x - k * TAU;
    let x2 = x * x;
    let (mut s, mut c) = (x, 1.0);
    let (mut ts, mut tc) = (x, 1.0);
    for i in 1..=14 {
        let f = (2 * i) as f64;
        tc *= -x2 / ((f - 1.0) * f);
        ts *= -x2 / (f * (f + 1.0));
        c += tc;
        s += ts;
    }
    (s, c)
}

/// Angle of the vector in (-pi, pi]: argument halving to |t| < 0.2, then the arctangent series.
pub fn angle(v: V) -> f64 {
    let (x, y) = (v[0], v[1]);
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }
    let (ax, ay) = (x.abs(), y.abs());
    let (num, den, swap) = if ay <= ax {
        (ay, ax, false)
    } else {
        (ax, ay, true)
    };
    let mut t = num / den;
    for _ in 0..3 {
        t /= 1.0 + (1.0 + t * t).sqrt();
    }
    let t2 = t * t;
    let (mut s, mut p) = (t, t);
    for i in 1..=8 {
        p *= -t2;
        s += p / (2 * i + 1) as f64;
    }
    let mut a = 8.0 * s;
    if swap {
        a = PI / 2.0 - a;
    }
    if x < 0.0 {
        a = PI - a;
    }
    if y < 0.0 {
        -a
    } else {
        a
    }
}

pub fn unit(a: f64) -> V {
    let (s, c) = sincos(a);
    [c, s]
}

pub fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1]]
}

pub fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1]]
}

pub fn scale(a: V, k: f64) -> V {
    [a[0] * k, a[1] * k]
}

pub fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

pub fn cross(a: V, b: V) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

pub fn len(a: V) -> f64 {
    dot(a, a).sqrt()
}

pub fn dist(a: V, b: V) -> f64 {
    len(sub(a, b))
}

/// `a` turned by the angle whose sine and cosine are `sc`.
pub fn rot(a: V, sc: (f64, f64)) -> V {
    [a[0] * sc.1 - a[1] * sc.0, a[0] * sc.0 + a[1] * sc.1]
}

/// `p` mirrored across the line through `a` and `b`.
pub fn reflect(p: V, a: V, b: V) -> V {
    let d = sub(b, a);
    let l2 = dot(d, d);
    if l2 == 0.0 {
        return p;
    }
    let t = dot(sub(p, a), d) / l2;
    let foot = add(a, scale(d, t));
    sub(scale(foot, 2.0), p)
}

/// Distance from `p` to the segment `a`-`b`.
pub fn seg_dist(p: V, a: V, b: V) -> f64 {
    let d = sub(b, a);
    let l2 = dot(d, d);
    let t = if l2 == 0.0 {
        0.0
    } else {
        (dot(sub(p, a), d) / l2).clamp(0.0, 1.0)
    };
    dist(p, add(a, scale(d, t)))
}

/// Whether the open segments `a`-`b` and `c`-`d` cross.
pub fn crosses(a: V, b: V, c: V, d: V) -> bool {
    let (d1, d2) = (cross(sub(b, a), sub(c, a)), cross(sub(b, a), sub(d, a)));
    let (d3, d4) = (cross(sub(d, c), sub(a, c)), cross(sub(d, c), sub(b, c)));
    d1 * d2 < -1e-12 && d3 * d4 < -1e-12
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_match_the_platform() {
        for i in -40..40 {
            let x = i as f64 * 0.37;
            let (s, c) = sincos(x);
            assert!((s - x.sin()).abs() < 1e-13 && (c - x.cos()).abs() < 1e-13);
            let v = [x.cos() * 2.5, x.sin() * 2.5];
            let a = angle(v);
            assert!((a.cos() - x.cos()).abs() < 1e-13 && (a.sin() - x.sin()).abs() < 1e-13);
        }
        assert_eq!(angle([-1.0, 0.0]), PI);
    }
}
