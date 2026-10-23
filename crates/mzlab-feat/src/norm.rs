//! Normalisation of the areas, a view over the aligned table: one factor per file, applied when the cells are read. The `.mzfeat` keeps the areas as
//! they were measured; the method and its parameters are what the `normalizza` operation of the `.mzflow` records.
//!
//! A cell counts when its state is not `CELL_ABSENT` and its area is a finite number above zero. Blanks and dark files are not part of the target
//! the factors aim at (they are scaled, but they do not decide the scale).

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};

use crate::aligned::{Aligned, CELL_ABSENT};
use crate::samples::Sample;

/// The internal standard: a row by its `id`, or the row nearest to an ion.
#[derive(Debug, Clone, PartialEq)]
pub enum Reference {
    Id(u32),
    Nearest {
        mz: f64,
        rt: f64,
        ppm: f64,
        rt_tol: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Method {
    None,
    /// Total area of the file.
    Sum,
    /// Median of the areas of the file.
    Median,
    /// Area of one row (the internal standard) in the file.
    Istd(Reference),
}

pub(crate) fn bad() -> Error {
    Error::new("error.badView")
}

/// Whether the file takes part in the target of the scale.
pub(crate) fn decides(s: &Sample) -> bool {
    !matches!(s.kind.as_str(), "blank" | "dark")
}

/// The area of the cell when the file has a peak there.
pub fn measured(t: &Aligned, row: usize, file: usize) -> Option<f64> {
    let c = row * t.n_files + file;
    let a = t.area[c];
    (t.state[c] != CELL_ABSENT && a.is_finite() && a > 0.0).then_some(a)
}

fn median(mut v: Vec<f64>) -> f64 {
    crate::sortc::by(&mut v, f64::total_cmp);
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

/// The row of the standard, or an error naming that there is none.
fn standard(t: &Aligned, r: &Reference) -> Result<usize> {
    match *r {
        Reference::Id(id) => t.row_of_id(id),
        Reference::Nearest {
            mz,
            rt,
            ppm,
            rt_tol,
        } => {
            let tol = mz * ppm * 1e-6;
            (0..t.len())
                .filter(|&i| (t.mz[i] - mz).abs() <= tol && (t.rt[i] - rt).abs() <= rt_tol)
                .min_by(|&a, &b| (t.rt[a] - rt).abs().total_cmp(&(t.rt[b] - rt).abs()))
        }
    }
    .ok_or_else(|| Error::new("error.normNoStandard"))
}

/// The statistic of every file (NaN where the file has no cell that counts) for a method that reads the whole file.
fn per_file(t: &Aligned, method: &Method, istd: Option<usize>) -> Vec<f64> {
    (0..t.n_files)
        .map(|k| {
            let cells: Vec<f64> = (0..t.len()).filter_map(|r| measured(t, r, k)).collect();
            match (method, istd) {
                (_, Some(row)) => measured(t, row, k).unwrap_or(f64::NAN),
                (Method::Sum, _) if !cells.is_empty() => cells.iter().sum(),
                (Method::Median, _) => median(cells),
                _ => f64::NAN,
            }
        })
        .collect()
}

/// One factor per file: normalised area = area * factor. 1 for every file with `Method::None`; NaN for a file the method cannot scale (no cell that
/// counts, or no peak of the standard).
pub fn factors(t: &Aligned, samples: &[Sample], method: &Method) -> Result<Vec<f64>> {
    if samples.len() != t.n_files {
        return Err(bad());
    }
    let istd = match method {
        Method::Istd(r) => Some(standard(t, r)?),
        _ => None,
    };
    if *method == Method::None {
        return Ok(vec![1.0; t.n_files]);
    }
    let stat = per_file(t, method, istd);
    let counted: Vec<f64> = (0..t.n_files)
        .filter(|&k| decides(&samples[k]) && stat[k].is_finite())
        .map(|k| stat[k])
        .collect();
    let target = match method {
        Method::Sum => mean(&counted),
        _ => median(counted),
    };
    Ok(stat
        .iter()
        .map(|&s| {
            if s.is_finite() && s > 0.0 {
                target / s
            } else {
                f64::NAN
            }
        })
        .collect())
}

/// The normalised area of a cell, `None` where the file has no peak.
pub fn normalised(t: &Aligned, factors: &[f64], row: usize, file: usize) -> Option<f64> {
    measured(t, row, file).map(|a| a * factors[file])
}

impl Method {
    /// The parameters of the `normalizza` operation: `{"metodo":"nessuna"|"somma"|"mediana"|"istd", "istd":{...}}`.
    pub fn to_json(&self) -> String {
        let mut j = Out::default();
        j.raw("{\"metodo\":");
        match self {
            Method::None => j.str("nessuna"),
            Method::Sum => j.str("somma"),
            Method::Median => j.str("mediana"),
            Method::Istd(r) => {
                j.str("istd").raw(",\"istd\":");
                match r {
                    Reference::Id(id) => j.raw("{\"id\":").int(i64::from(*id)),
                    Reference::Nearest {
                        mz,
                        rt,
                        ppm,
                        rt_tol,
                    } => j
                        .raw("{\"mz\":")
                        .num(*mz)
                        .raw(",\"rt\":")
                        .num(*rt)
                        .raw(",\"ppm\":")
                        .num(*ppm)
                        .raw(",\"rt_tol\":")
                        .num(*rt_tol),
                }
                .raw("}")
            }
        };
        j.raw("}");
        j.0
    }

    pub fn from_json(v: &Value) -> Result<Method> {
        let num = |o: &Value, k: &str| o.get(k).and_then(Value::f64);
        match v.get("metodo").and_then(Value::str) {
            Some("nessuna") => Ok(Method::None),
            Some("somma") => Ok(Method::Sum),
            Some("mediana") => Ok(Method::Median),
            Some("istd") => {
                let o = v.get("istd").ok_or_else(bad)?;
                if let Some(id) = num(o, "id") {
                    return Ok(Method::Istd(Reference::Id(id as u32)));
                }
                match (num(o, "mz"), num(o, "rt")) {
                    (Some(mz), Some(rt)) => Ok(Method::Istd(Reference::Nearest {
                        mz,
                        rt,
                        ppm: num(o, "ppm").unwrap_or(5.0),
                        rt_tol: num(o, "rt_tol").unwrap_or(0.2),
                    })),
                    _ => Err(bad()),
                }
            }
            _ => Err(bad()),
        }
    }
}
