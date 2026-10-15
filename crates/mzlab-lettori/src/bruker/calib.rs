//! The m/z calibration of a frame (`MzCalibration`, ModelType 1): the time of flight of a TOF index and the quadratic-in-sqrt law that turns it into m/z (docs/agenti/bruker.md).

/// One row of `MzCalibration`.
#[derive(Debug, Clone, Default)]
pub struct MzCal {
    pub modello: i64,
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
    pub fn fattore(&self, t1: f64, t2: f64) -> f64 {
        let f =
            1.0 - 1e-6 * (self.coef[0] * (t1 - self.temp[0]) + self.coef[1] * (t2 - self.temp[1]));
        if f.is_finite() {
            f
        } else {
            1.0
        }
    }

    /// Whether the constants make a usable law (a hostile file may bring zeros or NaN).
    pub fn valida(&self) -> bool {
        self.modello == 1
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
    fn tof_e_mz_si_invertono() {
        let c = MzCal {
            modello: 1,
            timebase: 0.125,
            delay: 18502.75,
            c: [280.6435408329972, 150581.44261048065, 0.0, 0.0, 0.0],
            ..MzCal::default()
        };
        assert!(c.valida());
        let m = c.mz(269196.13333333336);
        assert!((m - 405.164).abs() < 1e-3, "{m}");
        assert!(c.mz(2.0e5) < c.mz(2.1e5));
    }
}
