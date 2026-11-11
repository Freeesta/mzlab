//! Mass calibration of a Q-TOF run (`MSMassCal.bin`, `DefaultMassCal.xml`), see `lcms.rs`. A scan whose record says it has its own coefficients
//! (`MassCalOffset` > 0) and has no record behind it (file missing, cut short, record emptied) has no calibration (`Miss::NoRecord`), and a `CalibrationID`
//! that names no default calibration is `Miss::NoDefault`: never the default or first coefficients, which would shift its masses.

pub(super) const MASS_CAL_HEAD: usize = 72;
const MASS_CAL_REC: usize = 84;

impl Calibration {
    /// `mass_cal`: `MSMassCal.bin`, `default_cal`: `DefaultMassCal.xml` (BOM allowed); either may be missing.
    pub fn parse(mass_cal: Option<&[u8]>, default_cal: Option<&[u8]>) -> Calibration {
        let defaults = default_cal
            .map(|b| defaults_of(&String::from_utf8_lossy(b)))
            .unwrap_or_default();
        let scans = mass_cal
            .filter(|b| b.len() >= MASS_CAL_HEAD)
            .map(|b| {
                b[MASS_CAL_HEAD..]
                    .chunks_exact(MASS_CAL_REC)
                    .map(|r| {
                        let n = u32::from_le_bytes([r[0], r[1], r[2], r[3]]);
                        (n == 10).then(|| {
                            let mut v = [0f64; 10];
                            for (k, x) in v.iter_mut().enumerate() {
                                let o = 4 + k * 8;
                                *x = f64::from_le_bytes(r[o..o + 8].try_into().unwrap_or([0; 8]));
                            }
                            v
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Calibration { defaults, scans }
    }

    pub fn is_empty(&self) -> bool {
        self.defaults.is_empty()
    }

    /// m/z of the flight time `t` of the scan record number `index`, whose `CalibrationID` is `id` (`None`: a layout without the field, the first
    /// default): the coefficients of its own record in `MSMassCal.bin`, required when `own` (its `MassCalOffset` > 0), else the default ones; the powers
    /// of the polynomial always from the default calibration it names.
    pub(super) fn mz(
        &self,
        id: Option<i32>,
        index: Option<usize>,
        own: bool,
        t: f64,
    ) -> Result<f64, Miss> {
        let def = match id {
            Some(id) => self.defaults.iter().find(|d| d.id == id),
            None => self.defaults.first(),
        }
        .ok_or(Miss::NoDefault)?;
        let rec = index.and_then(|i| self.scans.get(i).copied().flatten());
        if own && rec.is_none() {
            return Err(Miss::NoRecord);
        }
        let (a, t0, tmin, tmax, c): (f64, f64, f64, f64, &[f64]) = match &rec {
            Some(s) => (s[0], s[1], s[2], s[3], &s[4..]),
            None => (def.a, def.t0, def.tmin, def.tmax, &def.c[..]),
        };
        let tt = t.max(tmin).min(tmax);
        let corr: f64 = def
            .powers
            .iter()
            .zip(c)
            .map(|(p, c)| c * tt.powi(*p as i32))
            .sum();
        let x = a * (t - t0);
        Some(x * x - corr)
            .filter(|v| v.is_finite())
            .ok_or(Miss::NotFinite)
    }
}

/// Why a flight time has no m/z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Miss {
    /// `DefaultMassCal.xml` has no calibration with the `CalibrationID` of the scan.
    NoDefault,
    /// The scan says it has its own record (`MassCalOffset` > 0) and `MSMassCal.bin` has none for it (missing, shorter than the run, record emptied).
    NoRecord,
    /// The coefficients give no finite m/z.
    NotFinite,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct DefaultCal {
    id: i32,
    a: f64,
    t0: f64,
    tmin: f64,
    tmax: f64,
    powers: Vec<u32>,
    c: Vec<f64>,
}

/// The mass calibration of a Q-TOF run: the defaults and the coefficients of each scan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Calibration {
    defaults: Vec<DefaultCal>,
    scans: Vec<Option<[f64; 10]>>,
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = s.find(open)? + open.len();
    Some(&s[a..a + s[a..].find(close)?])
}

fn defaults_of(xml: &str) -> Vec<DefaultCal> {
    let mut out = Vec::new();
    for part in xml.split("<DefaultCalibration ").skip(1) {
        let body = part.split("</DefaultCalibration>").next().unwrap_or("");
        let Some(id) = between(body, "DefaultCalibrationID=\"", "\"").and_then(|v| v.parse().ok())
        else {
            continue;
        };
        let mut cal = DefaultCal {
            id,
            ..Default::default()
        };
        for step in body.split("<Step ").skip(1) {
            let formula = between(step, "<CalibrationFormula>", "<").unwrap_or("");
            let flags: u32 = between(step, "<ValueUseFlags>", "<")
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0);
            let values: Vec<f64> = step
                .split("<Value Number=\"")
                .skip(1)
                .filter_map(|v| between(v, ">", "<")?.trim().parse().ok())
                .collect();
            match formula {
                "Traditional" if values.len() >= 2 => {
                    cal.a = values[0];
                    cal.t0 = values[1];
                }
                "Polynomial" if values.len() >= 3 => {
                    cal.tmin = values[0];
                    cal.tmax = values[1];
                    cal.c = values[2..].to_vec();
                    cal.powers = (0..32).filter(|k| flags >> k & 1 == 1).collect();
                }
                _ => {}
            }
        }
        out.push(cal);
    }
    out
}
