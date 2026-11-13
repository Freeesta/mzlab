//! The table of the integrations the user drew by hand on transition / SIM traces: one row per integration window (sample × channel × peak), grouped
//! by channel so a column of areas pastes into a sheet, an optional `sum` row of the areas per channel. The engine measures every window again from
//! the traces, so the site (`mzlab-wasm` `manual_table`), the app (`rpc.rs` op `manint`) and `mzlab-cli manual-table` write the same bytes.
//! Baselines as the Data view draws them: `zero`, `line` (`baseline::peak_area`) and `edges` (the page's line between the 3-point means of the edges).
//! A window with `splits` gives one row per part (`valley::parts`, drop lines on the window's line, `overlap` = `drop`, `pv` = Hp/Hv).

use crate::baseline::{peak_area, Baseline};
use crate::error::{Error, Result};
use crate::json::{parse, Value};
use crate::numfmt::sci;
use crate::peak1d::find_peak;
use crate::peakq_lr::near_idx;
use crate::valley;

/// The baselines a window can name (`baseline::MODES`).
pub const BASELINES: [&str; 3] = ["zero", "edges", "line"];
/// The header of the table.
pub const HEAD: &str =
    "channel,name,sample,start_min,end_min,apex_rt_min,area,height,snr,baseline,overlap,pv,label\n";
/// Smoothing and points across the peak of the S/N (`peak1d::find_peak` around the window, the defaults of `chrom.transitions@2`).
const SN_SMOOTH: usize = 3;
const SN_POINTS: usize = 3;
/// The longest name the user gives a window (characters), as the page's input allows.
pub const LABEL_MAX: usize = 40;

/// One channel of a file: its place in the file's list, Q1 / Q3 (absent when the reader gives none), name, times (min) and intensities.
struct Trace {
    pos: usize,
    q1: Option<f64>,
    q3: Option<f64>,
    name: String,
    t: Vec<f64>,
    y: Vec<f64>,
}

/// One measured window.
#[derive(Clone, Copy)]
struct Row {
    file: usize,
    a: f64,
    b: f64,
    apex: f64,
    area: f64,
    height: f64,
    snr: Option<f64>,
    base: &'static str,
    /// A part of a window split at its valleys, with its Hp/Hv.
    drop: bool,
    pv: Option<f64>,
    /// The place of the window in the request, for its name (`label_of_window`).
    w: usize,
}

/// The group of a channel across files: Q1 and Q3 at 0.01 (or the place in the list without Q1) and the name the method gives it.
type Key = (i64, i64, String);
/// A sample: its name without `.mzML` and its channels.
type Sample = (String, Vec<Trace>);
/// A channel group with its label and name.
type Channel = (Key, String, String);

fn nums(v: Option<&Value>) -> Vec<f64> {
    v.map_or(Vec::new(), |a| {
        a.arr().iter().map(|x| x.f64().unwrap_or(0.0)).collect()
    })
}

/// The channels of one `/api/mrm` transition list.
fn traces(list: &Value) -> Vec<Trace> {
    list.arr()
        .iter()
        .enumerate()
        .map(|(pos, t)| {
            let (rt, y) = (nums(t.get("rt")), nums(t.get("y")));
            let n = rt.len().min(y.len());
            Trace {
                pos,
                q1: t.get("q1").and_then(Value::f64),
                q3: t.get("q3").and_then(Value::f64),
                name: t.get("name").and_then(Value::str).unwrap_or("").to_string(),
                t: rt[..n].to_vec(),
                y: y[..n].to_vec(),
            }
        })
        .collect()
}

fn key_of(tr: &Trace) -> Key {
    let c = |v: f64| (v * 100.0).round() as i64;
    (
        tr.q1.map_or(-1 - tr.pos as i64, c),
        tr.q3.map_or(-1, c),
        tr.name.clone(),
    )
}

/// The channel as the page names it: the m/z of a SIM ion, «q1>q3» of a transition, `ion N` without Q1.
fn label_of(tr: &Trace) -> String {
    match (tr.q1, tr.q3) {
        (Some(q1), Some(q3)) => format!("{q1}>{q3}"),
        (Some(q1), None) => format!("{q1}"),
        _ => format!("ion {}", tr.pos + 1),
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

/// A CSV field: quoted when it holds a comma, a quote or a line break.
fn field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The page's end point of the line between the edges: the mean of the point and its two neighbours.
fn edge3(y: &[f64], i: usize) -> f64 {
    let (lo, hi) = (i.saturating_sub(1), (i + 1).min(y.len() - 1));
    y[lo..=hi].iter().sum::<f64>() / (hi - lo + 1) as f64
}

/// `[area (counts × s), height, apex RT]` against the line between the edges, as the Data view measures a window with `edges`: trapezoids minus the
/// line, area and height clipped at 0.
fn edges_area(t: &[f64], y: &[f64], a: f64, b: f64) -> [f64; 3] {
    let (lo, hi) = (near_idx(t, a.min(b)), near_idx(t, a.max(b)));
    if hi <= lo {
        return [0.0, 0.0, t[lo]];
    }
    let mut ar = 0.0;
    for j in lo + 1..=hi {
        ar += (y[j] + y[j - 1]) / 2.0 * (t[j] - t[j - 1]) * 60.0;
    }
    let at = (lo..=hi).fold(lo, |m, j| if y[j] > y[m] { j } else { m });
    let (ya, yb) = (edge3(y, lo), edge3(y, hi));
    let line = (ya + yb) / 2.0 * (t[hi] - t[lo]) * 60.0;
    let hb = ya + (yb - ya) * (t[at] - t[lo]) / (t[hi] - t[lo]);
    [(ar - line).max(0.0), (y[at] - hb).max(0.0), t[at]]
}

/// The baseline of a window: `baseline` (absent = `zero`) and, for `line`, `line` = `[t0, y0, t1, y1]`.
fn baseline_of(w: &Value) -> Result<Baseline> {
    let name = w.get("baseline").and_then(Value::str).unwrap_or("zero");
    let bad = || Error::new("err.manint.baseline").with("baseline", name);
    match name {
        "zero" => Ok(Baseline::Zero),
        "edges" => Ok(Baseline::Edges),
        "line" => match nums(w.get("line"))[..] {
            [t0, y0, t1, y1] if [t0, y0, t1, y1].iter().all(|v| v.is_finite()) => {
                Ok(Baseline::Line { t0, y0, t1, y1 })
            }
            _ => Err(bad()),
        },
        _ => Err(bad()),
    }
}

/// The samples of `files` (name without `.mzML`, channels) and the channels in the order they first appear, with their label and name.
fn samples_and_keys(files: &str) -> Result<(Vec<Sample>, Vec<Channel>)> {
    let list = parse(files.as_bytes())?;
    let samples: Vec<Sample> = list
        .arr()
        .iter()
        .map(|f| {
            let name = f.get("name").and_then(Value::str).unwrap_or("");
            (
                sample_of(name).to_string(),
                traces(f.get("transitions").unwrap_or(&Value::Null)),
            )
        })
        .collect();
    let mut keys: Vec<Channel> = Vec::new();
    for (_, trs) in &samples {
        for tr in trs {
            let k = key_of(tr);
            if !keys.iter().any(|(x, _, _)| *x == k) {
                keys.push((k, label_of(tr), tr.name.clone()));
            }
        }
    }
    Ok((samples, keys))
}

/// The name the user gave a window: no control characters, trimmed, at most `LABEL_MAX` characters.
fn label_of_window(w: &Value) -> String {
    let s = w.get("label").and_then(Value::str).unwrap_or("");
    let s: String = s.chars().filter(|c| !c.is_control()).collect();
    s.trim()
        .chars()
        .take(LABEL_MAX)
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// The channel of a window and its edges (in order): (file, trace, a, b).
fn channel_of<'a>(w: &Value, samples: &'a [Sample]) -> Result<(usize, &'a Trace, f64, f64)> {
    let num = |k: &str| w.get(k).and_then(Value::f64);
    let (fi, ti, a, b) = (num("file"), num("tr"), num("a"), num("b"));
    let tr = fi
        .zip(ti)
        .and_then(|(f, t)| samples.get(f as usize).and_then(|s| s.1.get(t as usize)));
    match (tr, a, b) {
        (Some(tr), Some(a), Some(b)) if tr.t.len() >= 2 && a.is_finite() && b.is_finite() => {
            Ok((fi.unwrap_or(0.0) as usize, tr, a.min(b), a.max(b)))
        }
        _ => Err(Error::new("err.manint.channel")),
    }
}

/// The line the parts of a split window share: `edges` as the line between the 3-point means of the edges, the others as they are.
fn drop_line(base: Baseline, tr: &Trace, lo: usize, hi: usize) -> Baseline {
    match base {
        Baseline::Edges => Baseline::Line {
            t0: tr.t[lo],
            y0: edge3(&tr.y, lo),
            t1: tr.t[hi],
            y1: edge3(&tr.y, hi),
        },
        b => b,
    }
}

/// The S/N of `peak1d::find_peak` around `a..b`.
fn snr_of(tr: &Trace, a: f64, b: f64) -> Option<f64> {
    find_peak(
        &tr.t,
        &tr.y,
        Some((a + b) / 2.0),
        (b - a) / 2.0,
        SN_SMOOTH,
        SN_POINTS,
    )
    .map(|f| f.snr)
    .filter(|v| v.is_finite())
}

/// One window measured on its channel: (the place of the channel in `keys`, a row, or a row per part when `splits` cuts it).
fn measure(i: usize, w: &Value, samples: &[Sample], keys: &[Channel]) -> Result<Vec<(usize, Row)>> {
    let (file, tr, a, b) = channel_of(w, samples)?;
    let base = baseline_of(w)?;
    let ch = keys
        .iter()
        .position(|(k, _, _)| *k == key_of(tr))
        .unwrap_or(0);
    let row =
        |a: f64, b: f64, apex: f64, area: f64, height: f64, pv: Option<f64>, drop: bool| Row {
            file,
            a,
            b,
            apex,
            area,
            height,
            snr: snr_of(tr, a, b),
            base: base.name(),
            drop,
            pv,
            w: i,
        };
    let (lo, hi) = (near_idx(&tr.t, a), near_idx(&tr.t, b));
    let cuts: Vec<usize> = nums(w.get("splits"))
        .iter()
        .filter(|s| s.is_finite())
        .map(|&s| near_idx(&tr.t, s))
        .filter(|&k| k > lo && k < hi)
        .collect();
    if cuts.is_empty() {
        let [area, height, apex] = match peak_area(&tr.t, &tr.y, a, b, base) {
            Some([area, _, height, apex]) => [area, height, apex],
            None => edges_area(&tr.t, &tr.y, a, b),
        };
        return Ok(vec![(ch, row(a, b, apex, area, height, None, false))]);
    }
    let line = drop_line(base, tr, lo, hi);
    let parts = valley::parts(&tr.t, &tr.y, lo, hi, &cuts, |x| line.at(x).unwrap_or(0.0));
    let n = parts.len();
    Ok(parts
        .iter()
        .enumerate()
        .map(|(j, p)| {
            let (pa, pb) = (
                if j == 0 { a } else { tr.t[p.lo] },
                if j + 1 == n { b } else { tr.t[p.hi] },
            );
            (
                ch,
                row(pa, pb, tr.t[p.apex], p.area * 60.0, p.height, p.pv, true),
            )
        })
        .collect())
}

/// The valleys where the window `window` (JSON, as one window of `table_text`) splits at `depth` (`valley::valleys` on the trace minus the line the
/// parts would share): `{"splits":[MIN,...]}`, times of the points of the trace.
pub fn valleys_text(files: &str, window: &str, depth: f64) -> Result<String> {
    let (samples, _) = samples_and_keys(files)?;
    let w = parse(window.as_bytes())?;
    let (_, tr, a, b) = channel_of(&w, &samples)?;
    let (lo, hi) = (near_idx(&tr.t, a), near_idx(&tr.t, b));
    let line = drop_line(baseline_of(&w)?, tr, lo, hi);
    let d: Vec<f64> =
        tr.t.iter()
            .zip(&tr.y)
            .map(|(&t, &y)| y - line.at(t).unwrap_or(0.0))
            .collect();
    let at: Vec<String> = valley::valleys(&d, lo, hi, depth)
        .iter()
        .map(|&k| format!("{}", tr.t[k]))
        .collect();
    Ok(format!("{{\"splits\":[{}]}}", at.join(",")))
}

/// The table for `files` (JSON `[{"name", "transitions"}]`, `transitions` as `/api/mrm` gives them) and `windows` (JSON `[{"file", "tr", "a", "b",
/// "baseline"?, "line"?, "label"?, "splits"?}]`: index of the file, index of its transition, the edges in min, the name the user gave the window, last
/// column, the times of its drop lines).
/// Channels in the order they first appear in the files, the windows of a channel by file then start; `sums` adds after each channel a row `sum` with the sum of its areas. `err.manint.channel` for a window
/// that names no channel or no times, `err.manint.baseline` for an unknown baseline or a `line` without its four numbers.
pub fn table_text(files: &str, windows: &str, sums: bool) -> Result<String> {
    let (samples, keys) = samples_and_keys(files)?;
    let list = parse(windows.as_bytes())?;
    let mut rows = list
        .arr()
        .iter()
        .enumerate()
        .map(|(i, w)| measure(i, w, &samples, &keys))
        .collect::<Result<Vec<_>>>()?
        .concat();
    let labels: Vec<String> = list.arr().iter().map(label_of_window).collect();
    crate::sortc::by(&mut rows, |x, y| {
        (x.0, x.1.file)
            .cmp(&(y.0, y.1.file))
            .then(x.1.a.total_cmp(&y.1.a))
    });
    let mut out = String::from(HEAD);
    for (ch, (_, label, name)) in keys.iter().enumerate() {
        let mine: Vec<&Row> = rows.iter().filter(|r| r.0 == ch).map(|r| &r.1).collect();
        if mine.is_empty() {
            continue;
        }
        let head = format!("{},{}", field(label), field(name));
        for r in &mine {
            out += &format!(
                "{head},{},{:.3},{:.3},{:.3},{},{},{},{},{},{},{}\n",
                field(&samples[r.file].0),
                r.a,
                r.b,
                r.apex,
                sci(r.area, 5),
                sci(r.height, 5),
                r.snr.map_or(String::new(), |v| format!("{v:.1}")),
                r.base,
                if r.drop { "drop" } else { "" },
                r.pv.map_or(String::new(), |v| format!("{v:.1}")),
                field(&labels[r.w])
            );
        }
        if sums {
            out += &format!(
                "{head},sum,,,,{},,,,,,\n",
                sci(mine.iter().map(|r| r.area).sum(), 5)
            );
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "manint_tests.rs"]
mod tests;
