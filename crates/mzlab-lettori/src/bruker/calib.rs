//! The m/z calibration of a frame (`MzCalibration`, ModelType 1): the time of flight of a TOF index and the quadratic-in-sqrt law that turns it into m/z (docs/agenti/bruker.md).

/// One row of `MzCalibration`.
#[derive(Debug, Clone, Default)]
pub struct MzCal {
    pub model: i64,
    /// ns per TOF index.
    pub timebase: f64,
    /// ns before the first sample.
    pub delay: f64,
    pub c: [f64; 5],
    /// Temperatures (T1, T2) of the calibration and their coefficients (ppm of m/z per degree): the frame's temperatures move the m/z.
    pub temp: [f64; 2],
    pub coef: [f64; 2],
}

impl MzCal {
    /// m/z of the (possibly fractional) TOF index `tof`: `t = delay + timebase·tof` and `t = C0 + k·√(m/z + C4) + C2·(m/z)`, `k = 1e6/√C1`, solved for m/z.
    pub fn mz(&self, tof: f64) -> f64 {
        let [c0, c1, c2, _c3, c4] = self.c;
        let t = self.delay + self.timebase * tof;
        let k = 1e6 / c1.sqrt();
        let u = t - c0 + c2 * c4;
        let s = 2.0 * u / (k + (k * k + 4.0 * c2 * u).sqrt());
        s * s - c4
    }

    /// The factor of the m/z of a frame measured at temperatures `t1`, `t2`: `1 - 1e-6·(dC1·(t1 - T1) + dC2·(t2 - T2))`.
    pub fn factor(&self, t1: f64, t2: f64) -> f64 {
        let f =
            1.0 - 1e-6 * (self.coef[0] * (t1 - self.temp[0]) + self.coef[1] * (t2 - self.temp[1]));
        if f.is_finite() {
            f
        } else {
            1.0
        }
    }

    /// Whether the constants make a usable law (a hostile file may bring zeros or NaN).
    pub fn validate(&self) -> bool {
        self.model == 1
            && self.c[1] > 0.0
            && self.c.iter().all(|x| x.is_finite())
            && self.timebase.is_finite()
            && self.delay.is_finite()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tof_and_mz_invert() {
        let c = MzCal {
            model: 1,
            timebase: 0.125,
            delay: 18502.75,
            c: [280.6435408329972, 150581.44261048065, 0.0, 0.0, 0.0],
            ..MzCal::default()
        };
        assert!(c.validate());
        let m = c.mz(269196.13333333336);
        assert!((m - 405.164).abs() < 1e-3, "{m}");
        assert!(c.mz(2.0e5) < c.mz(2.1e5));
    }

    #[test]
    fn the_quadratic_and_offset_terms_invert_the_forward_law() {
        let c = MzCal {
            model: 1,
            timebase: 0.125,
            delay: 18502.75,
            c: [280.0, 150_000.0, 2.0, 0.0, 0.5],
            ..MzCal::default()
        };
        // t = C0 + k·√(m/z + C4) + C2·(m/z) for m/z 400, back to its TOF index
        let k = 1e6 / c.c[1].sqrt();
        let t = c.c[0] + k * (400.0f64 + c.c[4]).sqrt() + c.c[2] * 400.0;
        let m = c.mz((t - c.delay) / c.timebase);
        assert!((m - 400.0).abs() < 1e-6, "{m}");
    }

    #[test]
    fn the_temperature_factor_adds_both_terms() {
        let c = MzCal {
            temp: [25.0, 30.0],
            coef: [2.0, 3.0],
            ..MzCal::default()
        };
        // 1 - 1e-6·(2·(26 - 25) + 3·(32 - 30)) = 1 - 8e-6
        assert!((c.factor(26.0, 32.0) - (1.0 - 8e-6)).abs() < 1e-12);
        assert_eq!(c.factor(25.0, 30.0), 1.0);
        assert_eq!(c.factor(f64::NAN, 30.0), 1.0);
    }
}
