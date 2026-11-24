//! The table of the integrations the user drew by hand on transition / SIM traces: one row per integration window (sample × channel × peak), grouped
//! by channel so a column of areas pastes into a sheet, an optional `sum` row of the areas per channel. The columns are the ones on screen (the method's
//! name of the channel is not written: the channel is its m/z); the apex RT and the area sit right after the name the user gave (apex first). The engine measures every window again from
//! the traces, so the site (`mzlab-wasm` `manual_table`), the app (`rpc.rs` op `manint`) and `mzlab-cli peak table` write the same bytes.
//! Baselines as the Data view draws them: `zero`, `line` (`baseline::peak_area`) and `edges` (the page's line between the 3-point means of the edges).
//! `time_min` (right after the sample) is the time point of the sample in minutes: the file's `time` when the list gives one (`null` = none, as a user
//! who emptied the cell), else the one the name carries (`samples::role_and_minutes`, none for a blank); empty when unknown.
//! Traces the page supplies itself (a chromatogram, an XIC, a DAD trace) may carry `panel`, `label`, `unit` and, per file, `conc` / `conc_unit`, and a window
//! `corr`: then the five columns `EXT_HEAD` follow `origin` (otherwise the bytes are the ones above, unchanged).
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
    "channel,sample,time_min,label,apex_rt_min,area,start_min,end_min,height,snr,baseline,overlap,pv,origin,sample_id,file,shift_min\n";
/// Smoothing and points across the peak of the S/N (`peak1d::find_peak` around the window, the defaults of `chrom.transitions@2`).
const SN_SMOOTH: usize = 3;
const SN_POINTS: usize = 3;
/// The S/N under which a window applied to another sample finds no peak there: the `snr_min` of `chrom.transitions@2`.
pub const FOLLOW_SN: f64 = 3.0;
/// The longest name the user gives a window (characters), as the page's input allows.
pub const LABEL_MAX: usize = 40;

/// One channel of a file: its place in the file's list, Q1 / Q3 (absent when the reader gives none), name, times (min) and intensities. `dup`: how
/// many earlier channels of the file round to the same Q1 / Q3 and name, so that two transitions closer than 0.005 Da stay two channels.
struct Trace {
    pos: usize,
    dup: usize,
    q1: Option<f64>,
    q3: Option<f64>,
    name: String,
    /// Set by a page-supplied trace: the panel, the channel's own label, the unit of its area, the concentration of the sample and its unit.
    panel: Option<String>,
    label: Option<String>,
    unit: String,
    conc: Option<f64>,
    conc_unit: String,
    t: Vec<f64>,
    y: Vec<f64>,
}

/// One measured window.
#[derive(Clone, Copy)]
struct Row {
    tr: usize,
    below: Option<f64>,
    /// The baseline as `[t0, y0, t1, y1]` (`None`: `edges`) and the noise zone the window names, for the optional figure columns.
    bl: Option<[f64; 4]>,
    zone: Option<[f64; 2]>,
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
    /// The name of the row (the window's, or the one given for this sample alone), a place in the names of the table.
    label: usize,
    /// How the row came to be: `manual` (drawn there), `applied` (the window of another sample, same edges), `shifted` (the same window moved so the apex
    /// found there falls in it), `no_peak` (applied, but no peak at the apex search: hidden nothing, blocked nothing).
    origin: &'static str,
    /// The start of the window all the rows of a peak come from: rows are ordered by channel, this, then sample.
    gkey: f64,
    /// The shift (min) the apex search found against the source's apex, for a window applied to another sample (empty otherwise): also the one
    /// that `max_shift` refused (`too_far`).
    found: Option<f64>,
}

/// The group of a channel across files: Q1 and Q3 at 0.01 (or the place in the list without Q1), the name the method gives it and `Trace::dup`.
type Key = (i64, i64, String, usize);
/// A sample: its name without `.mzML`, its channels, its time point in minutes and the file name as given (the `file` column).
type Sample = (String, Vec<Trace>, Option<f64>, String);
/// A channel group with its label and name.
type Channel = (Key, String, String);

fn nums(v: Option<&Value>) -> Vec<f64> {
    v.map_or(Vec::new(), |a| {
        a.arr().iter().map(|x| x.f64().unwrap_or(0.0)).collect()
    })
}

/// The channels of one `/api/mrm` transition list.
fn traces(list: &Value, conc: Option<f64>, conc_unit: &str) -> Vec<Trace> {
    let mut out: Vec<Trace> = Vec::new();
    for (pos, t) in list.arr().iter().enumerate() {
        let (rt, y) = (nums(t.get("rt")), nums(t.get("y")));
        let n = rt.len().min(y.len());
        let mut tr = Trace {
            pos,
            dup: 0,
            q1: t.get("q1").and_then(Value::f64),
            q3: t.get("q3").and_then(Value::f64),
            name: t.get("name").and_then(Value::str).unwrap_or("").to_string(),
            panel: t.get("panel").and_then(Value::str).map(str::to_string),
            label: t.get("label").and_then(Value::str).map(str::to_string),
            unit: t.get("unit").and_then(Value::str).unwrap_or("").to_string(),
            conc,
            conc_unit: conc_unit.to_string(),
            t: rt[..n].to_vec(),
            y: y[..n].to_vec(),
        };
        tr.dup = out.iter().filter(|o| key_of(o) == key_of(&tr)).count();
        out.push(tr);
    }
    out
}

fn key_of(tr: &Trace) -> Key {
    let c = |v: f64| (v * 100.0).round() as i64;
    // a trace with its own label is one channel by that label (and panel), whatever its place in its file's list
    if let Some(l) = &tr.label {
        return (
            i64::MIN,
            0,
            format!("{}\u{1}{l}", tr.panel.as_deref().unwrap_or("")),
            tr.dup,
        );
    }
    (
        tr.q1.map_or(-1 - tr.pos as i64, c),
        tr.q3.map_or(-1, c),
        tr.name.clone(),
        tr.dup,
    )
}

/// The channel as the page names it: the m/z of a SIM ion, «q1>q3» of a transition, `ion N` without Q1.
fn label_of(tr: &Trace) -> String {
    if let Some(l) = &tr.label {
        return l.clone();
    }
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

/// The baseline of a window: `baseline` (absent = `zero`) and, for `line`, `line` = `[t0, y0, t1, y1]`.
fn baseline_of(w: &Value, shift: f64) -> Result<Baseline> {
    let name = w.get("baseline").and_then(Value::str).unwrap_or("zero");
    let bad = || Error::new("err.manint.baseline").with("baseline", name);
    match name {
        "zero" => Ok(Baseline::Zero),
        "edges" => Ok(Baseline::Edges),
        "line" => match nums(w.get("line"))[..] {
            [t0, y0, t1, y1] if [t0, y0, t1, y1].iter().all(|v| v.is_finite()) => {
                Ok(Baseline::Line {
                    t0: t0 + shift,
                    y0,
                    t1: t1 + shift,
                    y1,
                })
            }
            _ => Err(bad()),
        },
        _ => Err(bad()),
    }
}

/// The time point of a sample in minutes: the `time` of the file (a number, or `null` for none), else the one its name carries.
fn time_of(f: &Value, name: &str) -> Option<f64> {
    match f.get("time") {
        Some(t) => t.f64().filter(|m| m.is_finite() && *m >= 0.0),
        None => crate::samples::role_and_minutes(name).1,
    }
}

/// Minutes as written in the table: three decimals at most, no trailing zeros (`90`, `1.5`), empty when unknown.
fn minutes_text(m: Option<f64>) -> String {
    m.map_or(String::new(), |v| {
        let t = format!("{v:.3}");
        t.trim_end_matches('0').trim_end_matches('.').to_string()
    })
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
                traces(
                    f.get("transitions").unwrap_or(&Value::Null),
                    f.get("conc").and_then(Value::f64).filter(|v| v.is_finite()),
                    f.get("conc_unit").and_then(Value::str).unwrap_or(""),
                ),
                time_of(f, name),
                name.to_string(),
            )
        })
        .collect();
    let mut keys: Vec<Channel> = Vec::new();
    for (_, trs, _, _) in &samples {
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
    clean_label(w.get("label").and_then(Value::str).unwrap_or(""))
}

/// The name of a window cleaned as `label_of_window` does.
fn clean_label(s: &str) -> String {
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

/// Where one row's window sits: the sample, the channel's place in its list, and the edges in order, with how it came to be and the start of the
/// window its peak group comes from.
#[derive(Clone, Copy)]
struct Item {
    w: usize,
    file: usize,
    tr: usize,
    a: f64,
    b: f64,
    shift: f64,
    origin: &'static str,
    gkey: f64,
    found: Option<f64>,
}

/// One window measured on its channel: (the place of the channel in `keys`, a row, or a row per part when `splits` cuts it). `w` is the window
/// the item comes from (the source's, for a window applied to another sample: its baseline and name, never its drop lines).
fn measure(
    it: &Item,
    w: &Value,
    label: usize,
    samples: &[Sample],
    keys: &[Channel],
) -> Result<Vec<(usize, Row)>> {
    let (file, tr, a, b) = (it.file, &samples[it.file].1[it.tr], it.a, it.b);
    let base = baseline_of(w, it.shift)?;
    let (bl, zone) = fig::inputs(base, w);
    let ch = keys
        .iter()
        .position(|(k, _, _)| *k == key_of(tr))
        .unwrap_or(0);
    let row =
        |a: f64, b: f64, apex: f64, area: f64, height: f64, pv: Option<f64>, drop: bool| Row {
            tr: it.tr,
            below: None,
            bl,
            zone,
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
            label,
            origin: it.origin,
            gkey: it.gkey,
            found: it.found,
        };
    let (lo, hi) = (near_idx(&tr.t, a), near_idx(&tr.t, b));
    let cuts: Vec<usize> = if it.origin == "manual" {
        nums(w.get("splits"))
            .iter()
            .filter(|s| s.is_finite())
            .map(|&s| near_idx(&tr.t, s))
            .filter(|&k| k > lo && k < hi)
            .collect()
    } else {
        Vec::new()
    };
    if cuts.is_empty() {
        let (area, height, apex, below) = match peak_area(&tr.t, &tr.y, a, b, base) {
            Some([area, below, height, apex]) => (area, height, apex, Some(below)),
            None => {
                let [area, height, apex] = edges_area(&tr.t, &tr.y, a, b);
                (area, height, apex, None)
            }
        };
        let mut r = row(a, b, apex, area, height, None, false);
        r.below = below;
        return Ok(vec![(ch, r)]);
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
    let line = drop_line(baseline_of(&w, 0.0)?, tr, lo, hi);
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
/// column, the times of its drop lines; `follow` windows may carry `max_shift`, min). `windows` may also be `{"windows": [...], "columns": [...],
/// "max_shift": MIN}`: `max_shift` is then the default of every `follow` window that has none of its own.
/// Channels in the order they first appear in the files, the windows of a channel by file then start; `sums` adds after each channel a row `sum` with the sum of its areas. `err.manint.channel` for a window
/// that names no channel or no times, `err.manint.baseline` for an unknown baseline or a `line` without its four numbers.
pub fn table_text(files: &str, windows: &str, sums: bool) -> Result<String> {
    let (samples, keys) = samples_and_keys(files)?;
    let doc = parse(windows.as_bytes())?;
    let (list, cols) = fig::split(&doc)?;
    let items = follow::items_of(list, &samples, follow::max_shift(&doc)?)?;
    let mut rows: Vec<(usize, Row)> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    // the correction note of each window, by the same place as its name
    let mut corrs: Vec<String> = Vec::new();
    for it in &items {
        let w = &list.arr()[it.w];
        // the name of the window, or the one given for this sample alone (`names`: {"<sample>": name})
        let own_name = w
            .get("names")
            .and_then(|n| n.get(&it.file.to_string()))
            .and_then(Value::str);
        let label = own_name.map_or_else(|| label_of_window(w), clean_label);
        labels.push(label);
        corrs.push(clean_label(
            w.get("corr").and_then(Value::str).unwrap_or(""),
        ));
        rows.extend(measure(it, w, labels.len() - 1, &samples, &keys)?);
    }
    crate::sortc::by(&mut rows, |x, y| {
        x.0.cmp(&y.0)
            .then(x.1.gkey.total_cmp(&y.1.gkey))
            .then(x.1.file.cmp(&y.1.file))
            .then(x.1.a.total_cmp(&y.1.a))
    });
    Ok(csv_text(
        &rows, &labels, &corrs, &keys, &samples, sums, cols,
    ))
}

#[path = "manint_edges.rs"]
mod edges;
use edges::{drop_line, edge3, edges_area};
#[path = "manint_csv.rs"]
mod csv;
use csv::csv_text;
pub use csv::EXT_HEAD;
pub use fig::{FIG_HEAD, QUALITY_HEAD, SN_HEAD, STATE_HEAD};
#[path = "manint_fig.rs"]
mod fig;
use fig::Cols;
#[path = "manint_follow.rs"]
mod follow;
pub use follow::follow_text;
#[path = "manint_auto.rs"]
mod auto;
pub use auto::auto_text;

#[cfg(test)]
#[path = "manint_auto_tests.rs"]
mod auto_tests;
#[cfg(test)]
#[path = "manint_ext_tests.rs"]
mod ext_tests;
#[cfg(test)]
#[path = "manint_nopeak_tests.rs"]
mod nopeak_tests;
#[cfg(test)]
#[path = "manint_shift_tests.rs"]
mod shift_tests;
#[cfg(test)]
#[path = "manint_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "manint_time_tests.rs"]
mod time_tests;
