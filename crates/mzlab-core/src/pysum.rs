//! Python's built-in `sum()` over floats. Since Python 3.12 it adds with Neumaier's compensation (so `sum([0.1] * 10)` is 1.0), which
//! changes the last bit; where the Python code sums floats with `sum()`, the Rust port must do the same to round to the same decimals.

pub fn pysum<I: IntoIterator<Item = f64>>(it: I) -> f64 {
    let (mut s, mut c) = (0.0f64, 0.0f64);
    for x in it {
        let t = s + x;
        if !t.is_finite() {
            // CPython falls back to plain addition once a value is not finite
            s = t;
            continue;
        }
        if s.abs() >= x.abs() {
            c += (s - t) + x;
        } else {
            c += (x - t) + s;
        }
        s = t;
    }
    if c != 0.0 && c.is_finite() {
        s + c
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::pysum;

    #[test]
    fn compensated() {
        assert_eq!(pysum(std::iter::repeat_n(0.1, 10)), 1.0);
        assert_eq!(pysum([]), 0.0);
    }
}
