//! Plausibility of the values a feature table shows, `plausible/1` (`docs/schemi/plausible-1.json`): non-finite numbers, m/z and RT out of any real run,
//! negative areas, impossible peak widths, rows with no peak in any file and the ppm error of the identity columns. Each check counts the rows it flags and
//! lists up to 20 of them; numbers only, nothing says what a row is. Used by the quality-control step of Tabui (page and app).

use mzlab_core::elements;
use mzlab_core::error::Result;
use mzlab_core::json::Out;

use crate::aligned::{Aligned, CELL_ABSENT, CELL_DETECTED};

pub const SCHEMA: &str = "plausible/1";
/// Example rows listed by a check.
pub const ROWS_CAP: usize = 20;
/// Alignment tolerance, Da, from which the table is unit resolution and a ppm error means nothing.
pub const UNIT_TOL_DA: f64 = 0.1;

/// What the checks read against. The defaults are the bounds of any real LC-MS run; `mz_tol_da` is the tolerance of the alignment (0.005 in HR, 0.3 at
/// unit resolution), which decides whether the ppm error can be read at all; without it the recipe of the table decides.
#[derive(Debug, Clone, PartialEq)]
pub struct Limits {
    /// Lowest and highest m/z; outside this band a value is a unit or parsing mistake.
    pub mz_min: f64,
    pub mz_max: f64,
    /// Longest run, minutes (10 h).
    pub rt_max: f64,
    /// A detected peak wider than this (minutes) is a baseline, not a peak.
    pub width_max: f64,
    /// Largest |ppm| between the m/z of a row and the neutral mass plus the adduct it carries, in a high-resolution table.
    pub ppm_max: f64,
    /// Tolerance of the alignment when the caller knows it; `None` reads the resolution from the recipe stored in the table (`recipe::is_unit_text`).
    pub mz_tol_da: Option<f64>,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            mz_min: 20.0,
            mz_max: 5000.0,
            rt_max: 600.0,
            width_max: 30.0,
            ppm_max: 50.0,
            mz_tol_da: None,
        }
    }
}

impl Limits {
    /// The limits of an options object `{"mz_tol_da","mz_min","mz_max","rt_max","width_max","ppm_max"}`; a missing, non-finite or non-positive key keeps the default,
    /// and an empty text is all the defaults.
    pub fn from_json(text: &str) -> Result<Limits> {
        let mut l = Limits::default();
        if text.trim().is_empty() {
            return Ok(l);
        }
        let j = mzlab_core::json::parse(text.as_bytes())?;
        for (key, slot) in [
            ("mz_min", &mut l.mz_min),
            ("mz_max", &mut l.mz_max),
            ("rt_max", &mut l.rt_max),
            ("width_max", &mut l.width_max),
            ("ppm_max", &mut l.ppm_max),
        ] {
            if let Some(v) = j
                .get(key)
                .and_then(|x| x.f64())
                .filter(|v| v.is_finite() && *v > 0.0)
            {
                *slot = v;
            }
        }
        l.mz_tol_da = j
            .get("mz_tol_da")
            .and_then(|x| x.f64())
            .filter(|v| v.is_finite() && *v > 0.0);
        Ok(l)
    }
}

/// IDs of the checks in the order of the text.
pub const IDS: [&str; 7] = [
    "non_finite",
    "mz_range",
    "rt_range",
    "negative",
    "peak_shape",
    "empty_rows",
    "ppm_error",
];

/// One check: `rows` = how many rows it flags, `ids` = the first ones (table ids, ascending), `warn` false means a note (the data are not wrong, but worth a look).
struct Check {
    id: &'static str,
    rows: usize,
    ids: Vec<u32>,
    warn: bool,
    /// `[min, max]` the check read against, or the limit alone; empty when the check does not have one.
    limit: Vec<f64>,
    /// Why the check could not be made, empty when it was.
    missing: &'static str,
}

impl Check {
    fn new(id: &'static str, warn: bool, limit: &[f64]) -> Check {
        Check {
            id,
            rows: 0,
            ids: Vec::new(),
            warn,
            limit: limit.to_vec(),
            missing: "",
        }
    }

    fn hit(&mut self, t: &Aligned, row: usize) {
        self.rows += 1;
        if self.ids.len() < ROWS_CAP {
            self.ids.push(t.id(row));
        }
    }
}

fn bad_value(v: f64) -> bool {
    !v.is_finite()
}

/// The checks of `t` read against `l`.
fn checks(t: &Aligned, unit: bool, l: &Limits) -> Vec<Check> {
    let mut non_finite = Check::new("non_finite", true, &[]);
    let mut mz_range = Check::new("mz_range", true, &[l.mz_min, l.mz_max]);
    let mut rt_range = Check::new("rt_range", true, &[0.0, l.rt_max]);
    let mut negative = Check::new("negative", true, &[0.0]);
    let mut peak_shape = Check::new("peak_shape", true, &[0.0, l.width_max]);
    let mut empty_rows = Check::new("empty_rows", false, &[]);
    let mut ppm_error = Check::new("ppm_error", true, &[l.ppm_max]);
    let hr = l.mz_tol_da.map_or(!unit, |tol| tol < UNIT_TOL_DA);
    if !hr {
        ppm_error.missing = "accurate_mass";
    } else if t.ident.is_empty() {
        ppm_error.missing = "identity";
    }
    let shifts = elements::adducts();
    for row in 0..t.len() {
        let (mz, rt) = (t.mz[row], t.rt[row]);
        let cells = row * t.n_files..(row + 1) * t.n_files;
        let mut finite = !bad_value(mz) && !bad_value(rt);
        let (mut neg, mut shape, mut seen) = (false, false, false);
        for c in cells {
            let (a, h) = (t.area[c], f64::from(t.height[c]));
            if bad_value(a) || bad_value(h) {
                finite = false;
            }
            if a < 0.0 || h < 0.0 {
                neg = true;
            }
            if t.state[c] != CELL_ABSENT {
                seen = true;
            }
            if t.state[c] == CELL_DETECTED {
                let w = f64::from(t.width[c]);
                // a detected peak always has an apex and a width
                if bad_value(w) || bad_value(f64::from(t.apex_rt[c])) || w <= 0.0 || w > l.width_max
                {
                    shape = true;
                }
            }
        }
        if !finite {
            non_finite.hit(t, row);
        }
        if finite && !(l.mz_min..=l.mz_max).contains(&mz) {
            mz_range.hit(t, row);
        }
        if finite && !(0.0..=l.rt_max).contains(&rt) {
            rt_range.hit(t, row);
        }
        if neg {
            negative.hit(t, row);
        }
        if shape {
            peak_shape.hit(t, row);
        }
        if !seen {
            empty_rows.hit(t, row);
        }
        if ppm_error.missing.is_empty() && finite && t.ident.ion[row] > 0 {
            let theory = t.ident.neutral[row] + shifts[usize::from(t.ident.ion[row]) - 1].1;
            if theory > 0.0 && ((mz - theory) / theory * 1e6).abs() > l.ppm_max {
                ppm_error.hit(t, row);
            }
        }
    }
    vec![
        non_finite, mz_range, rt_range, negative, peak_shape, empty_rows, ppm_error,
    ]
}

/// The `plausible/1` text of `t` read against `l`; `unit` = the recipe of the table is the unit-resolution one. Same table and limits, same bytes.
pub fn to_json(t: &Aligned, unit: bool, l: &Limits) -> String {
    let list = checks(t, unit, l);
    let flag = |c: &Check| {
        if c.rows == 0 {
            "ok"
        } else if c.warn {
            "warn"
        } else {
            "note"
        }
    };
    let mut o = Out::default();
    o.raw("{\"schema\":")
        .str(SCHEMA)
        .raw(",\"rows\":")
        .int(t.len() as i64)
        .raw(",\"summary\":{");
    for (i, f) in ["ok", "note", "warn"].iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(f).raw(":").int(
            list.iter()
                .filter(|c| c.missing.is_empty() && flag(c) == *f)
                .count() as i64,
        );
    }
    o.raw("},\"checks\":[");
    for (i, c) in list.iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .str(c.id);
        o.raw(",\"flag\":")
            .str(if c.missing.is_empty() { flag(c) } else { "na" });
        o.raw(",\"count\":").int(c.rows as i64).raw(",\"limit\":[");
        for (j, v) in c.limit.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).num(*v);
        }
        o.raw("],\"rows\":[");
        for (j, r) in c.ids.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).int(i64::from(*r));
        }
        o.raw("],\"missing\":");
        if c.missing.is_empty() {
            o.raw("null");
        } else {
            o.str(c.missing);
        }
        o.raw("}");
    }
    o.raw("]}");
    o.0
}
