//! The parameters of `quant.targets@1` (the defaults of its descriptor), read from the resolved parameters of a job, the CLI flags or the JSON the
//! browser sends: one reader for the three, so a value means the same everywhere. A value out of its range is refused, never clamped.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;
use mzlab_core::mzcal::Spec;

/// How the m/z offset (observed - true) of each file is taken.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OffsetMode {
    /// A number of Da for every file (0 = none).
    Fixed(f64),
    /// Measured per file and polarity on the first `standard`/`parent` row of the targets (none: 0, logged).
    Auto,
    /// Measured per file on the ion the text names (`auto:MZ[@RT]`, the `--mz-offset` of `trace xic`).
    Ion(Spec),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    /// Half-window of every XIC, Da.
    pub da: f64,
    /// Lowest S/N of a detected peak.
    pub min_sn: f64,
    /// Fewest points and lowest Gaussian correlation (`peakq::quality`) of an acceptable shape.
    pub min_points: usize,
    pub min_shape: f64,
    pub offset: OffsetMode,
    /// Comma list of case-insensitive globs over the file names: the blanks.
    pub blank_pattern: String,
    pub ms_level: u8,
    /// Savitzky-Golay half window of the detection copy (points, 0 = automatic per trace) and valley depth in % (`peak auto-lr`).
    pub smooth_half: usize,
    pub valley_pct: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            da: 0.5,
            min_sn: 3.0,
            min_points: 5,
            min_shape: 0.5,
            offset: OffsetMode::Fixed(0.0),
            blank_pattern: String::new(),
            ms_level: 1,
            smooth_half: 0,
            valley_pct: 10.0,
        }
    }
}

fn bad(name: &str, v: &str) -> Error {
    Error::new("err.quant.targets.param")
        .with("name", name.to_string())
        .with("value", v.to_string())
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Num(n) => n.to_string(),
        Value::Str(s) => s.trim().to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// The offset text: `0` or a number of Da (|D| <= 0.6), `auto`, or `auto:MZ[@RT]`.
pub fn offset_of(t: &str) -> Result<OffsetMode> {
    let t = t.trim();
    if t.is_empty() {
        return Ok(OffsetMode::Fixed(0.0));
    }
    if t.eq_ignore_ascii_case("auto") {
        return Ok(OffsetMode::Auto);
    }
    Ok(match Spec::parse(t)? {
        Spec::Zero => OffsetMode::Fixed(0.0),
        Spec::Fixed(d) => OffsetMode::Fixed(d),
        s => OffsetMode::Ion(s),
    })
}

impl Params {
    /// The parameters of `params`; an absent one keeps its default, a wrong one is `err.quant.targets.param` (or the `err.mz.offset.*` of the offset).
    pub fn of(params: &[(String, Value)]) -> Result<Params> {
        let mut p = Params::default();
        for (k, v) in params {
            let t = text_of(v);
            let num = || t.parse::<f64>().ok().filter(|x| x.is_finite());
            let count = |min: f64| num().filter(|x| *x >= min && x.fract() == 0.0);
            match k.as_str() {
                "da" => {
                    p.da = num()
                        .filter(|x| *x > 0.0 && *x <= 5.0)
                        .ok_or_else(|| bad(k, &t))?
                }
                "min_sn" => p.min_sn = num().filter(|x| *x >= 0.0).ok_or_else(|| bad(k, &t))?,
                "min_points" => p.min_points = count(1.0).ok_or_else(|| bad(k, &t))? as usize,
                "min_shape" => {
                    p.min_shape = num()
                        .filter(|x| (0.0..=1.0).contains(x))
                        .ok_or_else(|| bad(k, &t))?
                }
                "offset" => p.offset = offset_of(&t)?,
                "blank_pattern" => p.blank_pattern = t,
                "ms_level" => {
                    p.ms_level = count(1.0)
                        .filter(|x| *x <= 10.0)
                        .ok_or_else(|| bad(k, &t))? as u8
                }
                "smooth_half" => {
                    p.smooth_half = count(0.0)
                        .filter(|x| *x <= 30.0)
                        .ok_or_else(|| bad(k, &t))? as usize
                }
                "valley" => {
                    p.valley_pct = num()
                        .filter(|x| (0.0..=100.0).contains(x))
                        .ok_or_else(|| bad(k, &t))?
                }
                _ => return Err(bad(k, &t)),
            }
        }
        Ok(p)
    }

    /// The offset as the text the parameter takes (`0`, `0.24`, `auto`, `auto:MZ@RT`).
    pub fn offset_text(&self) -> String {
        match self.offset {
            OffsetMode::Fixed(d) => format!("{d}"),
            OffsetMode::Auto => "auto".into(),
            OffsetMode::Ion(s) => s.to_string(),
        }
    }

    /// Whether `name` is a blank: it matches one of the globs of `blank_pattern` (case-insensitive, `*` and `?`).
    pub fn is_blank(&self, name: &str) -> bool {
        let low: Vec<char> = name.to_lowercase().chars().collect();
        self.blank_pattern
            .split(',')
            .map(str::trim)
            .filter(|g| !g.is_empty())
            .any(|g| mzlab_core::glob::glob(&g.to_lowercase().chars().collect::<Vec<_>>(), &low))
    }
}
