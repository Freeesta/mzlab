//! `quant.calibrate@1`: applies the calibration curves the user imported (slope, intercept, linear range per compound, made in a spreadsheet) to the
//! areas of a feature table and writes a long CSV of concentrations with a flag per cell. It never fits a line: the standards are not read.
//! Called by `flow.rs`.

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::json::Value;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};
use mzlab_feat::samples::{self, KINDS};
use mzlab_feat::store::Slice;
use mzlab_feat::store::{origin_from_meta, read_table_from};
use mzlab_feat::table::CELL_ABSENT;

use crate::flow::{io, read_one, FileRunner, Made};
use crate::flow_table::samples_text;

const DEFAULT_MZ_TOL: f64 = 0.5;
const DEFAULT_RT_TOL: f64 = 0.3;

/// The descriptor of `quant.calibrate@1`: feature table, curves CSV and samples in, the concentrations as CSV out.
pub fn quant_step() -> Step {
    Step {
        ins: vec![
            Port::new("table", DataType::Table, "Feature table (.mzfeat)"),
            Port::new("curves", DataType::Report, "Calibration curves imported as CSV: name, mz, rt, slope, intercept, unit, range_min, range_max, dilution"),
            Port::new("samples", DataType::Samples, "Roles of the files (campioni/1 or campioni/2)"),
        ],
        outs: vec![Port::new("concentrations", DataType::Report, "Concentration of each compound in each sample as CSV")],
        params: vec![
            Param::number("mz_tol", "m/z tolerance to match a curve to a row")
                .default_num(DEFAULT_MZ_TOL)
                .min(0.0)
                .unit("Da"),
            Param::number("rt_tol", "RT tolerance to match a curve to a row")
                .default_num(DEFAULT_RT_TOL)
                .min(0.0)
                .unit("min"),
            Param::choice("signal", "Signal the curve was made with", &["area", "height"]).default_str("area"),
            Param::text("kinds", "Sample types to quantify, comma separated").default_str("sample"),
        ],
        runs_in: RunsIn {
            native: true,
            ..RunsIn::default()
        },
        errors: vec![
            "err.flow.quant.curves",
            "err.flow.quant.curveRow",
            "err.flow.quant.duplicate",
            "err.flow.quant.kind",
        ],
        ..Step::core(
            "quant.calibrate",
            "Calibrate",
            "Applies imported calibration curves (signal = slope x concentration + intercept) to the signal of the matched rows of the table; concentrations outside the linear range or below zero are flagged. The line is not fitted here.",
        )
    }
}

struct Curve {
    name: String,
    mz: f64,
    rt: f64,
    slope: f64,
    intercept: f64,
    unit: String,
    lo: Option<f64>,
    hi: Option<f64>,
    dilution: f64,
}

fn fields(line: &str, d: char) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if quoted && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' if quoted || cur.is_empty() => quoted = !quoted,
            c if c == d && !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// The curves of the CSV; the delimiter is the one of the header (`;` and tab read a decimal comma, as a spreadsheet writes it). The columns come in any order;
/// `range_min`, `range_max`, `dilution` and `unit` may be empty.
fn parse_curves(bytes: &[u8]) -> Result<Vec<Curve>> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_start_matches('\u{feff}')))
        .filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'));
    let bad = |detail: String| Error::new("err.flow.quant.curves").with("detail", detail);
    let (_, head) = lines.next().ok_or_else(|| bad("empty file".into()))?;
    let d = [';', '\t', ',']
        .into_iter()
        .find(|d| head.contains(*d))
        .unwrap_or(',');
    let names: Vec<String> = fields(head, d).iter().map(|h| h.to_lowercase()).collect();
    let col = |n: &str| names.iter().position(|h| h == n);
    let need = |n: &str| col(n).ok_or_else(|| bad(format!("no column {n}")));
    let (c_name, c_mz, c_rt, c_slope, c_int) = (
        need("name")?,
        need("mz")?,
        need("rt")?,
        need("slope")?,
        need("intercept")?,
    );
    let (c_unit, c_lo, c_hi, c_dil) = (
        col("unit"),
        col("range_min"),
        col("range_max"),
        col("dilution"),
    );
    let mut out: Vec<Curve> = Vec::new();
    for (n, line) in lines {
        let f = fields(line, d);
        let cell = |c: usize| f.get(c).map(String::as_str).unwrap_or("");
        let row = |what: &str| {
            Error::new("err.flow.quant.curveRow").with("detail", format!("line {n}: {what}"))
        };
        let num = |s: &str| -> Option<f64> {
            let s = if d == ',' {
                s.to_string()
            } else {
                s.replace(',', ".")
            };
            s.parse::<f64>().ok().filter(|v| v.is_finite())
        };
        let req = |c: usize, what: &str| num(cell(c)).ok_or_else(|| row(what));
        let opt = |c: Option<usize>, what: &str| match c.map(cell) {
            None | Some("") => Ok(None),
            Some(s) => num(s).map(Some).ok_or_else(|| row(what)),
        };
        let curve = Curve {
            name: cell(c_name).to_string(),
            mz: req(c_mz, "mz")?,
            rt: req(c_rt, "rt")?,
            slope: req(c_slope, "slope")?,
            intercept: req(c_int, "intercept")?,
            unit: c_unit.map(cell).unwrap_or("").to_string(),
            lo: opt(c_lo, "range_min")?,
            hi: opt(c_hi, "range_max")?,
            dilution: opt(c_dil, "dilution")?.unwrap_or(1.0),
        };
        if curve.name.is_empty() {
            return Err(row("name"));
        }
        if curve.slope == 0.0 {
            return Err(row("slope is zero"));
        }
        if curve.dilution <= 0.0 {
            return Err(row("dilution"));
        }
        if matches!((curve.lo, curve.hi), (Some(a), Some(b)) if a > b) {
            return Err(row("range_min above range_max"));
        }
        if out.iter().any(|c| c.name == curve.name) {
            return Err(Error::new("err.flow.quant.duplicate").with("name", curve.name));
        }
        out.push(curve);
    }
    if out.is_empty() {
        return Err(bad("no curve".into()));
    }
    Ok(out)
}

fn param<'a>(job: &'a Job, name: &str) -> Option<&'a Value> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
}

fn quoted(s: &str) -> String {
    if s.contains([',', ';', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The CSV `compound,mz,rt,sample,signal,concentration,unit,flag` and the warnings. `flag` is `ok`, `not_detected`, `below_range`, `above_range` or `negative`;
/// the range is checked on the concentration read from the curve, the dilution factor is applied after it.
pub(crate) fn quant_text(
    job: &Job,
    table: &[u8],
    curves: &[u8],
    samples: &[u8],
) -> Result<(String, Vec<String>)> {
    let curves = parse_curves(curves)?;
    let mz_tol = param(job, "mz_tol")
        .and_then(|v| v.f64())
        .unwrap_or(DEFAULT_MZ_TOL);
    let rt_tol = param(job, "rt_tol")
        .and_then(|v| v.f64())
        .unwrap_or(DEFAULT_RT_TOL);
    let by_height = param(job, "signal").and_then(|v| v.str()) == Some("height");
    let kinds: Vec<String> = param(job, "kinds")
        .and_then(|v| v.str())
        .unwrap_or("sample")
        .split(',')
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();
    if let Some(k) = kinds.iter().find(|k| !KINDS.contains(&k.as_str())) {
        return Err(Error::new("err.flow.quant.kind").with("kind", k.clone()));
    }
    let (t, meta) = read_table_from(&mut Slice(table), table.len() as u64)?;
    let files = origin_from_meta(&meta).inputs;
    let roles = samples::from_json(samples_text(samples)?.as_bytes())?;
    let mut warnings: Vec<String> = Vec::new();
    // the columns to quantify, in the order of the table
    let cols: Vec<(usize, &str)> = files
        .iter()
        .enumerate()
        .filter_map(|(i, (name, _))| {
            let role = roles.iter().find(|s| &s.file == name)?;
            kinds.contains(&role.kind).then_some((i, name.as_str()))
        })
        .collect();
    if files
        .iter()
        .any(|(n, _)| !roles.iter().any(|s| &s.file == n))
    {
        warnings.push("warn.flow.quant.unlisted".into());
    }
    let mut out = String::from("compound,mz,rt,sample,signal,concentration,unit,flag\n");
    let (mut no_row, mut shared, mut range, mut negative) = (false, false, false, false);
    let mut taken: Vec<usize> = Vec::new();
    for c in &curves {
        // the nearest row inside both tolerances, the first one on a tie
        let mut best: Option<(f64, usize)> = None;
        for row in 0..t.mz.len() {
            let (dm, dr) = ((t.mz[row] - c.mz).abs(), (t.rt[row] - c.rt).abs());
            if dm > mz_tol || dr > rt_tol {
                continue;
            }
            let d = dm / mz_tol.max(1e-9) + dr / rt_tol.max(1e-9);
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, row));
            }
        }
        let Some((_, row)) = best else {
            no_row = true;
            continue;
        };
        if taken.contains(&row) {
            shared = true;
        }
        taken.push(row);
        for &(file, name) in &cols {
            let cell = row * t.n_files + file;
            let head = format!(
                "{},{},{},{},",
                quoted(&c.name),
                t.mz[row],
                t.rt[row],
                quoted(name)
            );
            if t.state[cell] == CELL_ABSENT {
                out += &format!("{head},,{},not_detected\n", quoted(&c.unit));
                continue;
            }
            let signal = if by_height {
                t.height[cell] as f64
            } else {
                t.area[cell]
            };
            let measured = (signal - c.intercept) / c.slope;
            let flag = if measured < 0.0 {
                negative = true;
                "negative"
            } else if c.lo.is_some_and(|lo| measured < lo) {
                range = true;
                "below_range"
            } else if c.hi.is_some_and(|hi| measured > hi) {
                range = true;
                "above_range"
            } else {
                "ok"
            };
            out += &format!(
                "{head}{signal},{:.6e},{},{flag}\n",
                measured * c.dilution,
                quoted(&c.unit)
            );
        }
    }
    for (on, key) in [
        (no_row, "warn.flow.quant.noRow"),
        (shared, "warn.flow.quant.sharedRow"),
        (range, "warn.flow.quant.range"),
        (negative, "warn.flow.quant.negative"),
    ] {
        if on {
            warnings.push(key.into());
        }
    }
    Ok((out, warnings))
}

pub(crate) fn quant_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let (table, curves, samples) = (
        read_one(r, job, "table")?,
        read_one(r, job, "curves")?,
        read_one(r, job, "samples")?,
    );
    let (text, warnings) = quant_text(job, &table.1, &curves.1, &samples.1)?;
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    let mut made = Made::new("concentrations", r.keep(out, DataType::Report)?);
    made.warnings = warnings;
    Ok(made)
}
