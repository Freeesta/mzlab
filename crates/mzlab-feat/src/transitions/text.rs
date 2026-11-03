//! The texts of `chrom.transitions@1`: for each sample and each row of the table, the chromatogram of the row (unit resolution: Q1 and Q3 within `mz_tol`,
//! the same polarity) integrated inside RT ± window (`peaks`, long; `table`, one row per transition with the area and apex RT of each sample), or, for the
//! rows without an RT, the candidate peaks of their channel over all samples sorted by area (`candidates`). Isobaric rows share their candidates: which
//! peak is which isomer is left to the user.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

use super::table::{parse, Row};
use super::traces::{channels, Channel};
use crate::legacy::lr::peak1d::{find_peak, Found};
use crate::sortc::SortC;

/// What the step writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Peaks,
    Table,
    Candidates,
}

/// The parameters of the step (the defaults of its descriptor).
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub output: Output,
    pub mz_tol: f64,
    pub rt_window: f64,
    pub snr_min: f64,
    pub smooth: usize,
    pub min_points: usize,
    pub max_candidates: usize,
    pub group_rt: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            output: Output::Peaks,
            mz_tol: 0.5,
            rt_window: 0.3,
            snr_min: 3.0,
            smooth: 3,
            min_points: 3,
            max_candidates: 5,
            group_rt: 0.1,
        }
    }
}

impl Settings {
    /// The settings of the parameters of a job; an absent one keeps its default.
    pub fn of(params: &[(String, Value)]) -> Settings {
        let d = Settings::default();
        let get = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v);
        let num = |k: &str, v: f64| get(k).and_then(Value::f64).unwrap_or(v);
        let count = |k: &str, v: usize| num(k, v as f64).max(0.0) as usize;
        let output = match get("output").and_then(Value::str) {
            Some("table") => Output::Table,
            Some("candidates") => Output::Candidates,
            _ => Output::Peaks,
        };
        Settings {
            output,
            mz_tol: num("mz_tol", d.mz_tol),
            rt_window: num("rt_window", d.rt_window),
            snr_min: num("snr_min", d.snr_min),
            smooth: count("smooth", d.smooth),
            min_points: count("min_points", d.min_points),
            max_candidates: count("max_candidates", d.max_candidates),
            group_rt: num("group_rt", d.group_rt),
        }
    }
}

/// The channel of `row` in a sample: Q1 (and Q3, or none for a SIM row) within `tol` and the same polarity (a channel that writes none matches); the
/// nearest when several do, the first of the file on a tie.
fn channel_of<'a>(row: &Row, ch: &'a [Channel], tol: f64) -> Option<&'a Channel> {
    let off = |c: &Channel| -> Option<f64> {
        let d1 = (c.q1 - row.q1).abs();
        let d3 = match (row.q3, c.q3) {
            (None, None) => 0.0,
            (Some(a), Some(b)) => (a - b).abs(),
            _ => return None,
        };
        let pol = c.polarity.is_none_or(|p| p == row.polarity);
        (pol && d1 <= tol && d3 <= tol).then_some(d1 + d3)
    };
    let mut best: Option<(f64, &Channel)> = None;
    for c in ch {
        if let Some(d) = off(c).filter(|d| best.is_none_or(|(b, _)| *d < b)) {
            best = Some((d, c));
        }
    }
    best.map(|(_, c)| c)
}

/// The peak of one row in one sample and what it is: `ok`, `edge` (the apex is the first or last point of the window), `no_peak` (none, or under the S/N or
/// the points asked), `no_trace` (no channel), `no_rt` (the row has no RT: see the candidates).
struct Cell {
    found: Option<Found>,
    flag: &'static str,
}

impl Cell {
    fn counted(&self) -> Option<&Found> {
        self.found
            .as_ref()
            .filter(|_| matches!(self.flag, "ok" | "edge"))
    }
}

fn integrate(row: &Row, c: Option<&Channel>, s: &Settings) -> Cell {
    let (Some(c), Some(rt)) = (c, row.rt) else {
        let flag = if c.is_none() { "no_trace" } else { "no_rt" };
        return Cell { found: None, flag };
    };
    let w = row.rt_window.unwrap_or(s.rt_window);
    let found = find_peak(&c.t, &c.y, Some(rt), w, s.smooth, s.min_points);
    let flag = match &found {
        None => "no_peak",
        Some(f) if !f.ok || f.snr < s.snr_min => "no_peak",
        Some(f) => {
            let inside: Vec<f64> =
                c.t.iter()
                    .copied()
                    .filter(|t| (*t - rt).abs() <= w)
                    .collect();
            let edge = inside.len() > 2
                && (inside.first() == Some(&f.apex_rt) || inside.last() == Some(&f.apex_rt));
            if edge {
                "edge"
            } else {
                "ok"
            }
        }
    };
    Cell { found, flag }
}

/// Up to `s.max_candidates` peaks of a channel, highest first: the highest of the trace, then the highest of what is left on each side of it.
fn candidates(c: &Channel, s: &Settings) -> Vec<Found> {
    let (Some(&a), Some(&b)) = (c.t.first(), c.t.last()) else {
        return Vec::new();
    };
    let (mut spans, mut out) = (vec![(a, b)], Vec::new());
    while out.len() < s.max_candidates && !spans.is_empty() {
        let mut best: Option<(usize, Found)> = None;
        for (i, &(lo, hi)) in spans.iter().enumerate() {
            let f = find_peak(
                &c.t,
                &c.y,
                Some((lo + hi) / 2.0),
                (hi - lo) / 2.0,
                s.smooth,
                s.min_points,
            );
            if let Some(f) = f.filter(|f| best.as_ref().is_none_or(|(_, g)| f.height > g.height)) {
                best = Some((i, f));
            }
        }
        let Some((i, f)) = best else {
            break;
        };
        let (lo, hi) = spans.remove(i);
        if !f.ok || f.snr < s.snr_min {
            continue;
        }
        // an apex on the first or last point of its span is the flank of a peak already taken (or cut off by the end of the trace): cut it away, not counted
        let mut inside = c.t.iter().filter(|t| **t >= lo && **t <= hi);
        let ends = (inside.clone().next(), inside.next_back());
        let flank = ends.0 == Some(&f.apex_rt) || ends.1 == Some(&f.apex_rt);
        let (l, r) = (f.left.min(f.apex_rt), f.right.max(f.apex_rt));
        let eps = 1e-9;
        if l - eps > lo {
            spans.push((lo, l - eps));
        }
        if r + eps < hi {
            spans.push((r + eps, hi));
        }
        if !flank {
            out.push(f);
        }
    }
    out
}

/// A CSV field: quoted when it holds a comma, a quote or a line break.
fn field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn opt(v: Option<f64>, digits: usize) -> String {
    v.map_or(String::new(), |x| format!("{x:.digits$}"))
}

/// The columns that describe a row in every output.
fn row_cells(r: &Row) -> String {
    let pol = if r.polarity < 0 { "-" } else { "+" };
    format!(
        "{},{},{:.4},{},{pol},{},{}",
        field(&r.name),
        field(&r.isomer_group),
        r.q1,
        opt(r.q3, 4),
        opt(r.rt, 3),
        opt(r.rt_window, 3)
    )
}

const ROW_HEAD: &str = "name,isomer_group,q1,q3,polarity,rt,rt_window";

fn peaks_text(rows: &[Row], cells: &[(String, Vec<Cell>)]) -> String {
    let mut out = format!("sample,{ROW_HEAD},apex_rt,area,height,snr,fwhm_s,peak\n");
    for (sample, v) in cells {
        for (r, c) in rows.iter().zip(v) {
            let f = c.found.as_ref();
            out += &format!(
                "{},{},{},{},{},{},{},{}\n",
                field(sample),
                row_cells(r),
                opt(f.map(|f| f.apex_rt), 3),
                opt(f.map(|f| f.area), 1),
                opt(f.map(|f| f.height), 1),
                opt(f.map(|f| f.snr), 1),
                opt(f.map(|f| f.fwhm), 1),
                c.flag
            );
        }
    }
    out
}

fn table_text(rows: &[Row], cells: &[(String, Vec<Cell>)]) -> String {
    let mut out = ROW_HEAD.to_string();
    for (sample, _) in cells {
        out += &format!(",{}", field(&format!("area {sample}")));
    }
    for (sample, _) in cells {
        out += &format!(",{}", field(&format!("apex_rt {sample}")));
    }
    out += ",n_peaks,rt_mean,rt_sd\n";
    for (k, r) in rows.iter().enumerate() {
        let found: Vec<Option<&Found>> = cells.iter().map(|(_, v)| v[k].counted()).collect();
        out += &row_cells(r);
        for f in &found {
            out += &format!(",{}", opt(f.map(|f| f.area), 1));
        }
        for f in &found {
            out += &format!(",{}", opt(f.map(|f| f.apex_rt), 3));
        }
        let rts: Vec<f64> = found.iter().flatten().map(|f| f.apex_rt).collect();
        let n = rts.len() as f64;
        let mean = (n > 0.0).then(|| rts.iter().sum::<f64>() / n);
        let sd = mean
            .filter(|_| n > 1.0)
            .map(|m| (rts.iter().map(|t| (t - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt());
        out += &format!(",{},{},{}\n", rts.len(), opt(mean, 3), opt(sd, 3));
    }
    out
}

/// The candidates of the rows without an RT: the peaks of each sample grouped by apex RT (a gap above `group_rt` starts a new group), largest first.
fn candidates_text(rows: &[Row], samples: &[(String, Vec<Channel>)], s: &Settings) -> String {
    let mut out = format!(
        "{ROW_HEAD},candidate,apex_rt,apex_rt_min,apex_rt_max,samples,area_max,area_median\n"
    );
    for r in rows.iter().filter(|r| r.rt.is_none()) {
        let mut all: Vec<(f64, f64, usize)> = Vec::new();
        for (k, (_, ch)) in samples.iter().enumerate() {
            if let Some(c) = channel_of(r, ch, s.mz_tol) {
                all.extend(candidates(c, s).iter().map(|f| (f.apex_rt, f.area, k)));
            }
        }
        all.sortc_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)));
        let mut groups: Vec<Vec<(f64, f64, usize)>> = Vec::new();
        for p in all {
            match groups.last_mut() {
                Some(g) if g.last().is_some_and(|q| p.0 - q.0 <= s.group_rt) => g.push(p),
                _ => groups.push(vec![p]),
            }
        }
        let mut rowsout: Vec<(f64, String)> = groups
            .iter()
            .map(|g| {
                let mut areas: Vec<f64> = g.iter().map(|p| p.1).collect();
                areas.sortc_by(|a, b| a.total_cmp(b));
                let n = areas.len();
                let median = if n % 2 == 1 {
                    areas[n / 2]
                } else {
                    (areas[n / 2 - 1] + areas[n / 2]) / 2.0
                };
                let mut who: Vec<usize> = g.iter().map(|p| p.2).collect();
                who.sortc();
                who.dedup();
                let rt = g.iter().map(|p| p.0).sum::<f64>() / n as f64;
                let max = areas[n - 1];
                let text = format!(
                    "{rt:.3},{:.3},{:.3},{},{max:.1},{median:.1}",
                    g[0].0,
                    g[n - 1].0,
                    who.len()
                );
                (max, text)
            })
            .collect();
        rowsout.sortc_by(|a, b| b.0.total_cmp(&a.0));
        for (i, (_, t)) in rowsout.iter().enumerate() {
            out += &format!("{},{},{t}\n", row_cells(r), i + 1);
        }
    }
    out
}

/// The text of `chrom.transitions@1` for the table `table` (its CSV) over the mzML `files` (name, bytes) in their order.
pub fn transitions_text(
    params: &[(String, Value)],
    table: &str,
    files: &[(&str, &[u8])],
) -> Result<String> {
    let mut samples = Vec::new();
    for (name, bytes) in files {
        samples.push((name.to_string(), channels(bytes)?));
    }
    channels_text(params, table, &samples)
}

/// `transitions_text` over channels already read: (sample name, channels) in the order of the samples.
pub fn channels_text(
    params: &[(String, Value)],
    table: &str,
    samples: &[(String, Vec<Channel>)],
) -> Result<String> {
    let rows = parse(table).map_err(|e| {
        let first = e.first().map_or(String::new(), |r| {
            format!("{}: {} {}", r.line, r.key, r.detail)
        });
        Error::new("err.transitions.table").with("row", first)
    })?;
    if rows.is_empty() {
        return Err(Error::new("err.transitions.empty"));
    }
    let s = Settings::of(params);
    if samples.iter().all(|(_, c)| c.is_empty()) {
        return Err(Error::new("err.transitions.noChannel"));
    }
    if s.output == Output::Candidates {
        return Ok(candidates_text(&rows, samples, &s));
    }
    let cells: Vec<(String, Vec<Cell>)> = samples
        .iter()
        .map(|(n, ch)| {
            let v = rows
                .iter()
                .map(|r| integrate(r, channel_of(r, ch, s.mz_tol), &s))
                .collect();
            (n.clone(), v)
        })
        .collect();
    Ok(match s.output {
        Output::Table => table_text(&rows, &cells),
        _ => peaks_text(&rows, &cells),
    })
}

/// `transitions_text` for the browser (`flowtab`): `params_json` is the step's parameters merged with its defaults, the files are packed as `manifest`
/// (JSON `[{"name","len"}]`, in the order of the plan) over their concatenated bytes `blob`.
pub fn transitions_packed(
    params_json: &str,
    table: &str,
    manifest: &str,
    blob: &[u8],
) -> Result<String> {
    let params = crate::flow_ms2::json_params(params_json)?;
    let bad = || Error::new("err.transitions.manifest");
    let list = mzlab_core::json::parse(manifest.as_bytes())?;
    let (mut at, mut files) = (0usize, Vec::new());
    for f in list.arr() {
        let len = f.get("len").and_then(Value::f64).ok_or_else(bad)? as usize;
        let name = f.get("name").and_then(Value::str).ok_or_else(bad)?;
        files.push((name, blob.get(at..at + len).ok_or_else(bad)?));
        at += len;
    }
    transitions_text(&params, table, &files)
}
