//! The SIM table of the Data view: every SIM channel (a transition of `/api/mrm` with no Q3) of every file integrated with default settings, one row per
//! channel and one column per sample (`area` or `apex_rt`). Input = the `/api/mrm` transitions of each file, so the site (`mzlab-wasm` `sim_table`), the
//! app (`rpc.rs` op `simtab`) and `mzlab-cli sim-table` integrate the same numbers and write the same bytes. Peak = `peak1d::find_peak` over the whole
//! trace (the highest one), kept when it has `MIN_POINTS` points and S/N ≥ `SNR_MIN`; an empty cell otherwise. docs/agenti/transizioni.md.

use crate::error::{Error, Result};
use crate::json::{parse, Value};
use crate::peak1d::{find_peak, Found};

/// Smoothing (points), points across the peak and S/N of the default integration: the defaults of `chrom.transitions@1`.
pub const SMOOTH: usize = 3;
pub const MIN_POINTS: usize = 3;
pub const SNR_MIN: f64 = 3.0;

/// One SIM channel of a file: Q1, name, times (min) and intensities.
struct Trace {
    q1: f64,
    name: String,
    t: Vec<f64>,
    y: Vec<f64>,
}

/// The row a channel belongs to: Q1 at 0.01, the name, and its rank among the channels of one file with the same two (an ion monitored twice).
type Key = (i64, String, usize);

fn nums(v: Option<&Value>) -> Vec<f64> {
    v.map_or(Vec::new(), |a| a.arr().iter().map(|x| x.f64().unwrap_or(0.0)).collect())
}

/// The SIM channels of one `/api/mrm` transition list.
fn traces(list: &Value) -> Vec<Trace> {
    list.arr()
        .iter()
        .filter(|t| t.get("q3").and_then(Value::f64).is_none())
        .filter_map(|t| {
            let q1 = t.get("q1").and_then(Value::f64)?;
            let (rt, y) = (nums(t.get("rt")), nums(t.get("y")));
            let n = rt.len().min(y.len());
            Some(Trace {
                q1,
                name: t.get("name").and_then(Value::str).unwrap_or("").to_string(),
                t: rt[..n].to_vec(),
                y: y[..n].to_vec(),
            })
        })
        .collect()
}

/// The highest peak of the whole trace, when it counts.
fn peak(tr: &Trace) -> Option<Found> {
    let (a, b) = (*tr.t.first()?, *tr.t.last()?);
    find_peak(&tr.t, &tr.y, Some((a + b) / 2.0), (b - a) / 2.0, SMOOTH, MIN_POINTS)
        .filter(|f| f.ok && f.snr >= SNR_MIN)
}

/// A CSV field: quoted when it holds a comma, a quote or a line break.
fn field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The column of a sample: its file name without `.mzML`.
fn sample_of(name: &str) -> &str {
    let n = name.len();
    if n > 5 && name.is_char_boundary(n - 5) && name[n - 5..].eq_ignore_ascii_case(".mzml") {
        &name[..n - 5]
    } else {
        name
    }
}

/// The table for `files` (JSON `[{"name", "transitions"}]`, `transitions` as `/api/mrm` gives them) in their order; `output` is `area` (default) or
/// `apex_rt`. Rows in the order the channels first appear. `err.simtab.noChannel` when no file has a SIM channel.
pub fn table_text(files: &str, output: &str) -> Result<String> {
    let list = parse(files.as_bytes())?;
    let apex = output == "apex_rt";
    let mut samples: Vec<(String, Vec<(Key, Option<Found>)>)> = Vec::new();
    let mut rows: Vec<(Key, f64, String)> = Vec::new();
    for f in list.arr() {
        let name = f.get("name").and_then(Value::str).unwrap_or("");
        let mut cells = Vec::new();
        for tr in traces(f.get("transitions").unwrap_or(&Value::Null)) {
            let q = (tr.q1 * 100.0).round() as i64;
            let rank = cells.iter().filter(|((k, n, _), _): &&(Key, _)| *k == q && *n == tr.name).count();
            let key: Key = (q, tr.name.clone(), rank);
            if !rows.iter().any(|(k, _, _)| *k == key) {
                rows.push((key.clone(), tr.q1, tr.name.clone()));
            }
            cells.push((key, peak(&tr)));
        }
        samples.push((sample_of(name).to_string(), cells));
    }
    if rows.is_empty() {
        return Err(Error::new("err.simtab.noChannel"));
    }
    let mut out = String::from("q1,name");
    for (s, _) in &samples {
        out += ",";
        out += &field(s);
    }
    out += "\n";
    for (key, q1, name) in &rows {
        out += &format!("{q1:.4},{}", field(name));
        for (_, cells) in &samples {
            let f = cells.iter().find(|(k, _)| k == key).and_then(|(_, f)| f.as_ref());
            out += &match f {
                Some(f) if apex => format!(",{:.3}", f.apex_rt),
                Some(f) => format!(",{:.1}", f.area),
                None => ",".to_string(),
            };
        }
        out += "\n";
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(q1: f64, name: &str, apex: f64, h: f64) -> String {
        let (mut t, mut y) = (Vec::new(), Vec::new());
        for i in 0..200 {
            let x = i as f64 * 0.05;
            t.push(format!("{x:.4}"));
            y.push(format!("{:.1}", 50.0 + (i % 5) as f64 + h * (-0.5 * ((x - apex) / 0.08).powi(2)).exp()));
        }
        format!(
            "{{\"id\":0,\"q1\":{q1},\"q3\":null,\"ce\":null,\"name\":\"{name}\",\"dwell\":50,\"rt\":[{}],\"y\":[{}]}}",
            t.join(","),
            y.join(",")
        )
    }

    #[test]
    fn rows_are_channels_and_columns_samples() {
        let a = format!("[{},{}]", trace(237.1, "A", 2.5, 8e4), trace(253.1, "B", 4.0, 5e4));
        let b = format!("[{},{}]", trace(253.1, "B", 4.1, 2e4), trace(180.1, "C", 6.0, 1e4));
        let files = format!("[{{\"name\":\"s1.mzML\",\"transitions\":{a}}},{{\"name\":\"s, 2.mzml\",\"transitions\":{b}}}]");
        let area = table_text(&files, "area").unwrap();
        let lines: Vec<&str> = area.lines().collect();
        assert_eq!(lines[0], "q1,name,s1,\"s, 2\"");
        assert_eq!(lines.len(), 4);
        assert!(lines[1].starts_with("237.1000,A,") && lines[1].ends_with(','), "{area}");
        assert!(lines[3].starts_with("180.1000,C,,"), "{area}");
        let rt = table_text(&files, "apex_rt").unwrap();
        assert_eq!(rt.lines().nth(2).unwrap(), "253.1000,B,4.000,4.100");
    }

    #[test]
    fn flat_trace_and_mrm_rows() {
        let flat = "{\"id\":0,\"q1\":100,\"q3\":null,\"name\":\"\",\"rt\":[0,1,2,3,4],\"y\":[5,5,5,5,5]}";
        let mrm = "{\"id\":1,\"q1\":100,\"q3\":50,\"name\":\"\",\"rt\":[0,1,2],\"y\":[1,9,1]}";
        let t = table_text(&format!("[{{\"name\":\"x\",\"transitions\":[{flat},{mrm}]}}]"), "area").unwrap();
        assert_eq!(t, "q1,name,x\n100.0000,,\n");
        let none = table_text(&format!("[{{\"name\":\"x\",\"transitions\":[{mrm}]}}]"), "area");
        assert_eq!(none.unwrap_err().key, "err.simtab.noChannel");
    }
}
