//! Quality control of an aligned table, numbers only: what each file contributes, how well the replicates agree and how stable the standard is. It
//! judges nothing; the thresholds, if any, are the user's. Cells that count are those of `norm::measured`.

use mzlab_core::json::Out;

use crate::aligned::{Aligned, CELL_ABSENT, CELL_DETECTED, CELL_FILLED, CELL_TARGETED};
use crate::norm::{measured, Method, Reference};
use crate::samples::Sample;
use crate::view::View;

/// Counts of the cells of one file.
#[derive(Debug, Clone, PartialEq)]
pub struct FileQc {
    pub detected: usize,
    pub targeted: usize,
    pub filled: usize,
    pub absent: usize,
    /// Sum of the areas that count.
    pub total: f64,
}

/// Agreement of the replicates of one condition (same type, time, concentration and group): the median over the rows with a peak in every replicate
/// of the coefficient of variation (standard deviation over mean, population) of their areas.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupQc {
    pub kind: String,
    pub group: String,
    pub time_min: Option<f64>,
    pub files: Vec<usize>,
    pub rows: usize,
    pub cv_median: f64,
}

pub fn per_file(t: &Aligned) -> Vec<FileQc> {
    (0..t.n_files)
        .map(|k| {
            let mut q = FileQc {
                detected: 0,
                targeted: 0,
                filled: 0,
                absent: 0,
                total: 0.0,
            };
            for r in 0..t.len() {
                match t.state[r * t.n_files + k] {
                    CELL_DETECTED => q.detected += 1,
                    CELL_TARGETED => q.targeted += 1,
                    CELL_FILLED => q.filled += 1,
                    CELL_ABSENT => q.absent += 1,
                    _ => {}
                }
                q.total += measured(t, r, k).unwrap_or(0.0);
            }
            q
        })
        .collect()
}

fn cv(v: &[f64]) -> f64 {
    let m = v.iter().sum::<f64>() / v.len() as f64;
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt() / m
}

fn median(mut v: Vec<f64>) -> f64 {
    crate::sortc::by(&mut v, f64::total_cmp);
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// Conditions with at least two replicates, in the order of their first file. Areas are the ones of the view (normalised) when there is one.
pub fn groups(t: &Aligned, samples: &[Sample], view: Option<&View>) -> Vec<GroupQc> {
    let mut out: Vec<GroupQc> = Vec::new();
    let key = |s: &Sample| {
        (
            s.kind.clone(),
            s.group.clone(),
            s.time_min.map(f64::to_bits),
            s.conc.map(f64::to_bits),
        )
    };
    let mut seen: Vec<_> = Vec::new();
    for s in samples {
        let id = key(s);
        if seen.contains(&id) {
            continue;
        }
        seen.push(id.clone());
        let files: Vec<usize> = (0..samples.len())
            .filter(|&j| key(&samples[j]) == id)
            .collect();
        if files.len() < 2 {
            continue;
        }
        let rows = view.map_or_else(
            || (0..t.len() as u32).collect::<Vec<_>>(),
            |v| v.rows.clone(),
        );
        let cvs: Vec<f64> = rows
            .iter()
            .filter_map(|&r| {
                let vals: Option<Vec<f64>> = files
                    .iter()
                    .map(|&j| {
                        view.map_or_else(
                            || measured(t, r as usize, j),
                            |v| v.area_at(t, r as usize, j),
                        )
                    })
                    .collect();
                vals.map(|v| cv(&v))
            })
            .collect();
        out.push(GroupQc {
            kind: s.kind.clone(),
            group: s.group.clone(),
            time_min: s.time_min,
            files,
            rows: cvs.len(),
            cv_median: median(cvs),
        });
    }
    out
}

/// Coefficient of variation of the area of the standard over the files that decide the scale (None without a standard or with fewer than two peaks).
pub fn standard_cv(t: &Aligned, samples: &[Sample], method: &Method) -> Option<f64> {
    let Method::Istd(r) = method else { return None };
    let row = match r {
        Reference::Id(id) => t.row_of_id(*id),
        Reference::Nearest {
            mz,
            rt,
            ppm,
            rt_tol,
        } => (0..t.len())
            .find(|&i| (t.mz[i] - mz).abs() <= mz * ppm * 1e-6 && (t.rt[i] - rt).abs() <= *rt_tol),
    }?;
    let v: Vec<f64> = (0..t.n_files)
        .filter(|&k| crate::norm::decides(&samples[k]))
        .filter_map(|k| measured(t, row, k))
        .collect();
    (v.len() >= 2).then(|| cv(&v))
}

/// Rows that have a peak in at least one blank: (blank files, rows considered, rows with a peak in a blank). The rows are those of the view, or all of the table.
pub fn blank_share(t: &Aligned, samples: &[Sample], view: Option<&View>) -> (usize, usize, usize) {
    let blanks: Vec<usize> = (0..t.n_files)
        .filter(|&k| samples[k].kind == "blank")
        .collect();
    let rows = view.map_or_else(
        || (0..t.len() as u32).collect::<Vec<_>>(),
        |v| v.rows.clone(),
    );
    let hit = rows
        .iter()
        .filter(|&&r| {
            blanks
                .iter()
                .any(|&k| measured(t, r as usize, k).is_some_and(|a| a > 0.0))
        })
        .count();
    (blanks.len(), rows.len(), hit)
}

/// M+1/M of the rows (of the view, or all) that have one: (rows with it, minimum, median, maximum). None when the table has no summaries of the rows.
pub fn isotope_ratio(t: &Aligned, view: Option<&View>) -> Option<(usize, f64, f64, f64)> {
    if t.meta.is_empty() {
        return None;
    }
    let rows = view.map_or_else(
        || (0..t.len() as u32).collect::<Vec<_>>(),
        |v| v.rows.clone(),
    );
    let v: Vec<f64> = rows
        .iter()
        .map(|&r| f64::from(t.meta.iso_m1[r as usize]))
        .filter(|x| x.is_finite())
        .collect();
    if v.is_empty() {
        return None;
    }
    let (lo, hi) = (
        v.iter().copied().fold(f64::INFINITY, f64::min),
        v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    Some((v.len(), lo, median(v), hi))
}

/// The `qc/1` document: per file the counts and the total, per condition the agreement of the replicates, and the stability of the standard.
pub fn to_json(t: &Aligned, samples: &[Sample], view: Option<&View>) -> String {
    let mut j = Out::default();
    j.raw("{\"schema\":\"qc/1\",\"file\":[");
    for (k, q) in per_file(t).iter().enumerate() {
        j.raw(if k > 0 { ",{\"file\":" } else { "{\"file\":" })
            .str(&samples[k].file);
        j.raw(",\"rilevati\":")
            .int(q.detected as i64)
            .raw(",\"mirati\":")
            .int(q.targeted as i64);
        j.raw(",\"riempiti\":")
            .int(q.filled as i64)
            .raw(",\"assenti\":")
            .int(q.absent as i64);
        j.raw(",\"area_totale\":").num(q.total).raw("}");
    }
    j.raw("],\"gruppi\":[");
    for (i, g) in groups(t, samples, view).iter().enumerate() {
        j.raw(if i > 0 { ",{\"tipo\":" } else { "{\"tipo\":" })
            .str(&g.kind);
        j.raw(",\"gruppo\":")
            .str(&g.group)
            .raw(",\"tempo_min\":")
            .opt(g.time_min);
        j.raw(",\"file\":")
            .int(g.files.len() as i64)
            .raw(",\"righe\":")
            .int(g.rows as i64);
        j.raw(",\"cv_mediano\":")
            .opt(Some(g.cv_median).filter(|x| x.is_finite()))
            .raw("}");
    }
    j.raw("],\"standard_cv\":");
    let method = view.map(|v| v.method.clone());
    j.opt(method.and_then(|m| standard_cv(t, samples, &m)));
    let (nb, n, hit) = blank_share(t, samples, view);
    j.raw(",\"bianchi\":{\"file\":")
        .int(nb as i64)
        .raw(",\"righe\":")
        .int(n as i64)
        .raw(",\"righe_con_picco\":")
        .int(hit as i64)
        .raw("}");
    j.raw(",\"m1_su_m\":");
    match isotope_ratio(t, view) {
        Some((n, lo, med, hi)) => j
            .raw("{\"righe\":")
            .int(n as i64)
            .raw(",\"min\":")
            .num(lo)
            .raw(",\"mediana\":")
            .num(med)
            .raw(",\"max\":")
            .num(hi)
            .raw("}"),
        None => j.raw("null"),
    };
    j.raw(",\"scartate\":[");
    if let Some(v) = view {
        for (i, r) in v.rules.iter().enumerate() {
            let n = v.why.iter().filter(|&&w| w as usize == i + 1).count();
            j.raw(if i > 0 {
                ",{\"regola\":"
            } else {
                "{\"regola\":"
            })
            .str(r.name())
            .raw(",\"righe\":")
            .int(n as i64)
            .raw("}");
        }
    }
    j.raw("]}");
    j.0
}
