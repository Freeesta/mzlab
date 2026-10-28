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

/// Median by selection: the same one or two middle values as a full sort in `total_cmp` order.
fn median(mut v: Vec<f64>) -> f64 {
    let n = v.len();
    if n == 0 {
        return f64::NAN;
    }
    let (lower, mid, _) = v.select_nth_unstable_by(n / 2, f64::total_cmp);
    if n % 2 == 1 {
        return *mid;
    }
    let below = lower
        .iter()
        .copied()
        .max_by(f64::total_cmp)
        .unwrap_or(f64::NAN);
    (below + *mid) / 2.0
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

/// The row of the standard, or an error naming that there is none.
pub(crate) fn standard(t: &Aligned, r: &Reference) -> Result<usize> {
    match *r {
        Reference::Id(id) => t.row_of_id(id),
        Reference::Nearest {
            mz,
            rt,
            ppm,
            rt_tol,
        } => {
            let tol = mz * ppm * 1e-6;
            // Both offsets count, each in units of its tolerance: a candidate on the edge of the m/z window with the closest RT does not beat the
            // standard on the exact mass a little farther in RT. The first of equal distances wins.
            let scaled = |d: f64, tol: f64| if tol > 0.0 { d / tol } else { 0.0 };
            let dist = |i: usize| {
                let (dm, dr) = (
                    scaled((t.mz[i] - mz).abs(), tol),
                    scaled((t.rt[i] - rt).abs(), rt_tol),
                );
                dm * dm + dr * dr
            };
            (0..t.len())
                .filter(|&i| (t.mz[i] - mz).abs() <= tol && (t.rt[i] - rt).abs() <= rt_tol)
                .min_by(|&a, &b| dist(a).total_cmp(&dist(b)))
        }
    }
    .ok_or_else(|| Error::new("error.normNoStandard"))
}

/// The statistic of every file (NaN where the file has no cell that counts) for a method that reads the whole file.
fn per_file(t: &Aligned, method: &Method, istd: Option<usize>) -> Vec<f64> {
    let cells = |k: usize| (0..t.len()).filter_map(move |r| measured(t, r, k));
    (0..t.n_files)
        .map(|k| match (method, istd) {
            (_, Some(row)) => measured(t, row, k).unwrap_or(f64::NAN),
            (Method::Sum, _) => {
                // streamed in row order, like the sum of the collected cells
                let mut n = 0usize;
                let sum: f64 = cells(k).inspect(|_| n += 1).sum();
                if n > 0 {
                    sum
                } else {
                    f64::NAN
                }
            }
            (Method::Median, _) => median(cells(k).collect()),
            _ => f64::NAN,
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
                    // an id is a whole number that fits a row id: `as u32` would clamp a negative or huge one to a real row
                    if !(id.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&id)) {
                        return Err(bad());
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aligned::CELL_DETECTED;

    fn median_by_sort(mut v: Vec<f64>) -> f64 {
        crate::sortc::by(&mut v, f64::total_cmp);
        match v.len() {
            0 => f64::NAN,
            n if n % 2 == 1 => v[n / 2],
            n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
        }
    }

    #[test]
    fn selection_gives_the_median_of_a_full_sort() {
        let mut seed = 3u64;
        let mut r = |n: u64| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) % n
        };
        let specials = [f64::NAN, -0.0, 0.0, f64::INFINITY, f64::NEG_INFINITY];
        for case in 0..5000 {
            let n = r(12) as usize;
            let v: Vec<f64> = (0..n)
                .map(|_| {
                    if case % 3 == 0 && r(6) == 0 {
                        specials[r(5) as usize]
                    } else {
                        r(7) as f64 * 0.5 - 1.0
                    }
                })
                .collect();
            assert_eq!(
                median_by_sort(v.clone()).to_bits(),
                median(v).to_bits(),
                "case {case}"
            );
        }
    }

    fn table(rows: &[(f64, f64)]) -> Aligned {
        let mut t = Aligned::with_capacity(1, rows.len());
        for &(mz, rt) in rows {
            t.mz.push(mz);
            t.rt.push(rt);
            t.area.push(1e6);
            t.height.push(1e5);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state.push(CELL_DETECTED);
        }
        t
    }

    #[test]
    fn the_nearest_standard_counts_the_mass_error_as_well_as_the_rt() {
        // an interferent 4.9 ppm off, 0.001 min from the RT given, and the standard 0.1 ppm off, 0.1 min away
        let (mz, rt) = (300.0, 5.0);
        let t = table(&[
            (mz * (1.0 + 4.9e-6), rt + 0.001),
            (mz * (1.0 + 0.1e-6), rt + 0.1),
        ]);
        let by_rt_only =
            (0..t.len()).min_by(|&a, &b| (t.rt[a] - rt).abs().total_cmp(&(t.rt[b] - rt).abs()));
        assert_eq!(by_rt_only, Some(0));
        let near = Reference::Nearest {
            mz,
            rt,
            ppm: 5.0,
            rt_tol: 0.2,
        };
        assert_eq!(standard(&t, &near).unwrap(), 1);
        // outside the window of either axis a row is not a candidate; equal distances keep the first row
        let t = table(&[(mz, rt + 0.3), (mz, rt + 0.1), (mz, rt - 0.1)]);
        assert_eq!(standard(&t, &near).unwrap(), 1);
        let none = Reference::Nearest {
            mz: 400.0,
            rt,
            ppm: 5.0,
            rt_tol: 0.2,
        };
        assert!(standard(&t, &none).is_err());
        // zero tolerances accept only an exact match instead of dividing by zero
        let exact = Reference::Nearest {
            mz,
            rt: rt + 0.1,
            ppm: 0.0,
            rt_tol: 0.0,
        };
        assert_eq!(standard(&t, &exact).unwrap(), 1);
    }

    #[test]
    fn the_sum_streams_in_row_order_and_keeps_nan_for_a_file_without_cells() {
        let mut t = table(&[(100.0, 1.0), (101.0, 2.0), (102.0, 3.0)]);
        t.area = vec![0.1, 1e16, -1.0];
        t.state[2] = CELL_ABSENT;
        assert_eq!(
            per_file(&t, &Method::Sum, None)[0].to_bits(),
            (0.1f64 + 1e16).to_bits()
        );
        t.state = vec![CELL_ABSENT; 3];
        assert!(per_file(&t, &Method::Sum, None)[0].is_nan());
        assert!(per_file(&t, &Method::Median, None)[0].is_nan());
    }

    #[test]
    fn an_istd_id_must_be_a_whole_row_number() {
        let parse = |s: &str| Method::from_json(&mzlab_core::json::parse(s.as_bytes()).unwrap());
        assert_eq!(
            parse(r#"{"metodo":"istd","istd":{"id":3}}"#).unwrap(),
            Method::Istd(Reference::Id(3))
        );
        for bad_id in ["-1", "2.5", "4294967296", "1e30"] {
            assert!(
                parse(&format!(r#"{{"metodo":"istd","istd":{{"id":{bad_id}}}}}"#)).is_err(),
                "{bad_id}"
            );
        }
    }
}
