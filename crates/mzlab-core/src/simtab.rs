//! The SIM table of the Data view: every SIM channel (a transition of `/api/mrm` with no Q3) of every file integrated with default settings. `peaks` is
//! the long table (one row per resolved peak: isomers at one m/z are separate peaks); the other outputs pivot it, one row per sample and one column per
//! channel (layout `samples`) or the reverse (`channels`). Input = the `/api/mrm` transitions of each file, so the site (`mzlab-wasm` `sim_table`), the app
//! (`rpc.rs` op `simtab`) and `mzlab-cli sim-table` write the same bytes. Peaks = `peak1d::peaks` over the whole trace. docs/agenti/transizioni.md.

use crate::error::{Error, Result};
use crate::json::{parse, Value};
use crate::peak1d::{peaks, Found};

/// Smoothing (points, 1-2-1), points across a peak and S/N of the default integration: the defaults of `chrom.transitions@1`.
pub const SMOOTH: usize = 3;
pub const MIN_POINTS: usize = 3;
pub const SNR_MIN: f64 = 3.0;
/// Peaks kept per channel and sample (isomers at one m/z).
pub const MAX_PEAKS: usize = 10;
/// The outputs: the long table, then the pivots (sum of the peak areas, area / height / apex RT of the largest peak, number of peaks).
pub const OUTPUTS: [&str; 6] = [
    "area_sum",
    "area_max",
    "height_max",
    "apex_rt",
    "n_peaks",
    "peaks",
];

/// One SIM channel of a file: transition id, Q1 (absent when the id has none), name, times (min) and intensities.
struct Trace {
    id: i64,
    q1: Option<f64>,
    name: String,
    t: Vec<f64>,
    y: Vec<f64>,
}

/// The column a channel belongs to: Q1 at 0.01 (or the transition id), the name, and its rank among the channels of one file with the same two.
type Key = (i64, bool, String, usize);
/// A sample: its name and the peaks of each of its channels.
type Sample = (String, Vec<(Key, Vec<Found>)>);

fn nums(v: Option<&Value>) -> Vec<f64> {
    v.map_or(Vec::new(), |a| {
        a.arr().iter().map(|x| x.f64().unwrap_or(0.0)).collect()
    })
}

/// The SIM channels of one `/api/mrm` transition list.
fn traces(list: &Value) -> Vec<Trace> {
    list.arr()
        .iter()
        .filter(|t| t.get("q3").and_then(Value::f64).is_none())
        .map(|t| {
            let (rt, y) = (nums(t.get("rt")), nums(t.get("y")));
            let n = rt.len().min(y.len());
            Trace {
                id: t.get("id").and_then(Value::f64).unwrap_or(0.0) as i64,
                q1: t.get("q1").and_then(Value::f64),
                name: t.get("name").and_then(Value::str).unwrap_or("").to_string(),
                t: rt[..n].to_vec(),
                y: y[..n].to_vec(),
            }
        })
        .collect()
}

/// The peaks of a channel in RT order.
fn found(tr: &Trace) -> Vec<Found> {
    let mut v = peaks(&tr.t, &tr.y, SMOOTH, MIN_POINTS, SNR_MIN, MAX_PEAKS);
    crate::sortc::by(&mut v, |a, b| a.apex_rt.total_cmp(&b.apex_rt));
    v
}

/// A CSV field: quoted when it holds a comma, a quote or a line break.
fn field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The name of a sample: its file name without `.mzML`.
fn sample_of(name: &str) -> &str {
    let n = name.len();
    if n > 5 && name.is_char_boundary(n - 5) && name[n - 5..].eq_ignore_ascii_case(".mzml") {
        &name[..n - 5]
    } else {
        name
    }
}

/// A channel column: its Q1 m/z as read (`transition=N` without one), `#k` for the k-th channel of a file with the same m/z and name.
struct Column {
    key: Key,
    q1: Option<f64>,
    name: String,
    label: String,
}

/// The peaks of channel `key` in a file's `cells`; `None` = no such channel.
fn get<'a>(cells: &'a [(Key, Vec<Found>)], key: &Key) -> Option<&'a Vec<Found>> {
    cells.iter().find(|(k, _)| k == key).map(|(_, p)| p)
}

/// One pivot cell of a channel's peaks; `None` = the file has no such channel.
fn cell(p: Option<&Vec<Found>>, output: &str) -> String {
    let Some(p) = p else { return String::new() };
    let top = p.iter().max_by(|a, b| a.area.total_cmp(&b.area));
    match (output, top) {
        ("area_sum", _) => format!("{:.1}", p.iter().fold(0.0, |s, f| s + f.area)),
        ("n_peaks", _) => p.len().to_string(),
        (_, None) => String::new(),
        ("area_max", Some(f)) => format!("{:.1}", f.area),
        ("height_max", Some(f)) => format!("{:.1}", f.height),
        (_, Some(f)) => format!("{:.3}", f.apex_rt),
    }
}

/// The table for `files` (JSON `[{"name", "transitions"}]`, `transitions` as `/api/mrm` gives them) in their order (the injection order the user
/// loaded). `output` is one of `OUTPUTS`; `layout` `samples` (default: one row per sample) or `channels` (one row per channel). Columns in the order
/// the channels first appear. `err.simtab.noChannel` when no file has a SIM channel, `err.simtab.output` for an unknown output or layout.
pub fn table_text(files: &str, output: &str, layout: &str) -> Result<String> {
    if !OUTPUTS.contains(&output) || !matches!(layout, "samples" | "channels") {
        return Err(Error::new("err.simtab.output")
            .with("output", output)
            .with("layout", layout));
    }
    let list = parse(files.as_bytes())?;
    let mut samples: Vec<Sample> = Vec::new();
    let mut cols: Vec<Column> = Vec::new();
    for f in list.arr() {
        let name = f.get("name").and_then(Value::str).unwrap_or("");
        let mut cells: Vec<(Key, Vec<Found>)> = Vec::new();
        for tr in traces(f.get("transitions").unwrap_or(&Value::Null)) {
            let (q, has) = tr
                .q1
                .map_or((tr.id, false), |q| ((q * 100.0).round() as i64, true));
            let rank = cells
                .iter()
                .filter(|((k, h, n, _), _)| *k == q && *h == has && *n == tr.name)
                .count();
            let key: Key = (q, has, tr.name.clone(), rank);
            if !cols.iter().any(|c| c.key == key) {
                let base = tr
                    .q1
                    .map_or(format!("transition={}", tr.id), |q| format!("{q}"));
                cols.push(Column {
                    key: key.clone(),
                    q1: tr.q1,
                    name: tr.name.clone(),
                    label: if rank > 0 {
                        format!("{base} #{}", rank + 1)
                    } else {
                        base
                    },
                });
            }
            cells.push((key, found(&tr)));
        }
        samples.push((sample_of(name).to_string(), cells));
    }
    if cols.is_empty() {
        return Err(Error::new("err.simtab.noChannel"));
    }
    let mut out = String::new();
    if output == "peaks" {
        out += "sample,q1,name,peak,apex_rt,area,height,snr,fwhm_s\n";
        for (s, cells) in &samples {
            for c in &cols {
                let Some(p) = get(cells, &c.key) else {
                    continue;
                };
                let head = format!(
                    "{},{},{}",
                    field(s),
                    c.q1.map_or(String::new(), |q| format!("{q:.4}")),
                    field(&c.name)
                );
                if p.is_empty() {
                    out += &format!("{head},0,,,,,\n");
                }
                for (i, f) in p.iter().enumerate() {
                    out += &format!(
                        "{head},{},{:.3},{:.1},{:.1},{:.1},{:.1}\n",
                        i + 1,
                        f.apex_rt,
                        f.area,
                        f.height,
                        f.snr,
                        f.fwhm
                    );
                }
            }
        }
    } else if layout == "samples" {
        out += "sample";
        for c in &cols {
            out += ",";
            out += &field(&c.label);
        }
        out += "\n";
        for (s, cells) in &samples {
            out += &field(s);
            for c in &cols {
                out += ",";
                out += &cell(get(cells, &c.key), output);
            }
            out += "\n";
        }
    } else {
        out += "channel,name";
        for (s, _) in &samples {
            out += ",";
            out += &field(s);
        }
        out += "\n";
        for c in &cols {
            out += &format!("{},{}", field(&c.label), field(&c.name));
            for (_, cells) in &samples {
                out += ",";
                out += &cell(get(cells, &c.key), output);
            }
            out += "\n";
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "simtab_tests.rs"]
mod tests;
