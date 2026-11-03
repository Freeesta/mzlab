//! The SIM table of the Data view: every SIM channel (a transition of `/api/mrm` with no Q3) of every file integrated with default settings. `peaks` is
//! the long table (one row per resolved peak: isomers at one m/z are separate peaks); the other outputs pivot it, one row per sample and one column per
//! channel (layout `samples`) or the reverse (`channels`). A column is one m/z across files; a channel without Q1 is its place in the list, kept apart
//! per group. Input = the `/api/mrm` transitions of each file and its group, so the site (`mzlab-wasm` `sim_table`), the app (`rpc.rs` op `simtab`) and
//! `mzlab-cli sim-table` write the same bytes. Areas and heights in `numfmt::sci`. docs/agenti/transizioni.md.

use crate::error::{Error, Result};
use crate::json::{parse, Value};
use crate::numfmt::sci;
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

/// One SIM channel of a file: its place in the file's transition list, Q1 (absent when the id has none), name, times (min) and intensities.
struct Trace {
    pos: usize,
    q1: Option<f64>,
    name: String,
    t: Vec<f64>,
    y: Vec<f64>,
}

/// The column a channel belongs to: Q1 at 0.01 (or, without Q1, the place in the list and the group of the file), and its rank among the
/// channels of one file with the same key.
type Key = (i64, bool, String, usize);
/// A sample: its name, its group ("" = none) and the peaks of each of its channels.
type Sample = (String, String, Vec<(Key, Vec<Found>)>);

fn nums(v: Option<&Value>) -> Vec<f64> {
    v.map_or(Vec::new(), |a| {
        a.arr().iter().map(|x| x.f64().unwrap_or(0.0)).collect()
    })
}

/// The SIM channels of one `/api/mrm` transition list.
fn traces(list: &Value) -> Vec<Trace> {
    list.arr()
        .iter()
        .enumerate()
        .filter(|(_, t)| t.get("q3").and_then(Value::f64).is_none())
        .map(|(pos, t)| {
            let (rt, y) = (nums(t.get("rt")), nums(t.get("y")));
            let n = rt.len().min(y.len());
            Trace {
                pos,
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
pub fn sample_of(name: &str) -> &str {
    let n = name.len();
    if n > 5 && name.is_char_boundary(n - 5) && name[n - 5..].eq_ignore_ascii_case(".mzml") {
        &name[..n - 5]
    } else {
        name
    }
}

/// A channel column: its Q1 m/z as read; without one `ion N` (N = place in the list from 1, the page's «ione N»), with ` [group]` when the file has
/// one; `#k` for the k-th channel of a file with the same key.
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
        ("area_sum", _) => sci(p.iter().fold(0.0, |s, f| s + f.area), 5),
        ("n_peaks", _) => p.len().to_string(),
        (_, None) => String::new(),
        ("area_max", Some(f)) => sci(f.area, 5),
        ("height_max", Some(f)) => sci(f.height, 5),
        (_, Some(f)) => format!("{:.3}", f.apex_rt),
    }
}

/// The table for `files` (JSON `[{"name", "transitions", "group"?}]`, `transitions` as `/api/mrm` gives them). Rows: the groups in the order
/// they first appear, then the files without one, each in the order given (the injection order the user loaded); a `group` column after `sample`
/// when a file has one. `output` is one of `OUTPUTS`; `layout` `samples` (default: one row per sample) or `channels` (one row per channel, no group).
/// Columns in the order the channels first appear. `err.simtab.noChannel` when no file has a SIM channel, `err.simtab.output` for an unknown output or layout.
pub fn table_text(files: &str, output: &str, layout: &str) -> Result<String> {
    if !OUTPUTS.contains(&output) || !matches!(layout, "samples" | "channels") {
        return Err(Error::new("err.simtab.output")
            .with("output", output)
            .with("layout", layout));
    }
    let list = parse(files.as_bytes())?;
    let mut order: Vec<&str> = Vec::new();
    for f in list.arr() {
        let g = f.get("group").and_then(Value::str).unwrap_or("");
        if !g.is_empty() && !order.contains(&g) {
            order.push(g);
        }
    }
    let rank_of = |g: &str| order.iter().position(|x| *x == g).unwrap_or(order.len());
    let mut files: Vec<&Value> = list.arr().iter().collect();
    files.sort_by_key(|f| rank_of(f.get("group").and_then(Value::str).unwrap_or("")));
    let mut samples: Vec<Sample> = Vec::new();
    let mut cols: Vec<Column> = Vec::new();
    for f in files {
        let name = f.get("name").and_then(Value::str).unwrap_or("");
        let group = f.get("group").and_then(Value::str).unwrap_or("");
        let mut cells: Vec<(Key, Vec<Found>)> = Vec::new();
        for tr in traces(f.get("transitions").unwrap_or(&Value::Null)) {
            let (q, has, g) = tr.q1.map_or((tr.pos as i64, false, group), |q| {
                ((q * 100.0).round() as i64, true, "")
            });
            let rank = cells
                .iter()
                .filter(|((k, h, gg, _), _)| *k == q && *h == has && gg == g)
                .count();
            let key: Key = (q, has, g.to_string(), rank);
            if !cols.iter().any(|c| c.key == key) {
                let base = match tr.q1 {
                    Some(q) => format!("{q}"),
                    None if g.is_empty() => format!("ion {}", tr.pos + 1),
                    None => format!("ion {} [{g}]", tr.pos + 1),
                };
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
        samples.push((sample_of(name).to_string(), group.to_string(), cells));
    }
    if cols.is_empty() {
        return Err(Error::new("err.simtab.noChannel"));
    }
    let grouped = !order.is_empty();
    Ok(if output == "peaks" {
        peaks_text(&samples, &cols, grouped)
    } else if layout == "samples" {
        by_sample(&samples, &cols, output, grouped)
    } else {
        by_channel(&samples, &cols, output)
    })
}

/// The long table: one row per peak, a channel without peaks = peak 0.
fn peaks_text(samples: &[Sample], cols: &[Column], grouped: bool) -> String {
    let mut out = String::new();
    out += if grouped { "sample,group," } else { "sample," };
    out += "q1,name,peak,apex_rt,area,height,snr,fwhm_s\n";
    for (s, g, cells) in samples {
        for c in cols {
            let Some(p) = get(cells, &c.key) else {
                continue;
            };
            let head = format!(
                "{}{},{},{}",
                field(s),
                if grouped {
                    format!(",{}", field(g))
                } else {
                    String::new()
                },
                c.q1.map_or(String::new(), |q| format!("{q:.4}")),
                field(&c.name)
            );
            if p.is_empty() {
                out += &format!("{head},0,,,,,\n");
            }
            for (i, f) in p.iter().enumerate() {
                out += &format!(
                    "{head},{},{:.3},{},{},{:.1},{:.1}\n",
                    i + 1,
                    f.apex_rt,
                    sci(f.area, 5),
                    sci(f.height, 5),
                    f.snr,
                    f.fwhm
                );
            }
        }
    }
    out
}

/// The pivot with one row per sample and one column per channel.
fn by_sample(samples: &[Sample], cols: &[Column], output: &str, grouped: bool) -> String {
    let mut out = String::new();
    out += if grouped { "sample,group" } else { "sample" };
    for c in cols {
        out += ",";
        out += &field(&c.label);
    }
    out += "\n";
    for (s, g, cells) in samples {
        out += &field(s);
        if grouped {
            out += ",";
            out += &field(g);
        }
        for c in cols {
            out += ",";
            out += &cell(get(cells, &c.key), output);
        }
        out += "\n";
    }
    out
}

/// The pivot with one row per channel and one column per sample.
fn by_channel(samples: &[Sample], cols: &[Column], output: &str) -> String {
    let mut out = String::new();
    out += "channel,name";
    for (s, _, _) in samples {
        out += ",";
        out += &field(s);
    }
    out += "\n";
    for c in cols {
        out += &format!("{},{}", field(&c.label), field(&c.name));
        for (_, _, cells) in samples {
            out += ",";
            out += &cell(get(cells, &c.key), output);
        }
        out += "\n";
    }
    out
}

#[cfg(test)]
#[path = "simtab_tests.rs"]
mod tests;
