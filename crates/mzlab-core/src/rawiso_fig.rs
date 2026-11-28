//! The figure of `raw isotopes` (`figure=1` of `/api/rawiso`, `--figure PATH` of the CLI): the traces of the bins M, M+1 and M+2 over the window with the peak
//! region shaded, and below them the observed ratios R1 and R2 with their sigma beside the expected ones and the verdict of each. It only draws what the
//! `result` holds, so the numbers of `raw-isotopes/1` stay the evidence. Texts are `note.rawiso.fig.*` keys of `lang/en/figure-raw.js` (the file is read
//! here, so the site, the app and the CLI write the same bytes); colours are Okabe-Ito on white, as the other static figures.

use crate::rawiso::Ask;
use crate::rawiso_found::{Found, Ratio, Test};

const CATALOG: &str = include_str!("../../../mzlab/web/lang/en/figure-raw.js");

const INK: &str = "#1a1a1a";
const MUTED: &str = "#595959";
const GRID: &str = "#d9d9d9";
const REGION: &str = "#cfe3f3";
/// Okabe-Ito: blue (M), orange (M+1), bluish green (M+2).
const TRACE: [&str; 3] = ["#0072B2", "#E69F00", "#009E73"];
const OK: &str = "#009E73";
const BAD: &str = "#D55E00";
const MAYBE: &str = "#7f7f7f";

const W_T: &str = "960";
const LEFT: f64 = 180.0;
const RIGHT: f64 = 930.0;
const PANEL_H: f64 = 84.0;
const TRACES_TOP: f64 = 74.0;
const RATIO_ROW: f64 = 92.0;

/// The text of `key` in the catalogue (one `"key": "text",` per line), its `{name}` filled in one pass; the key itself when it has no text.
#[inline(never)]
fn tx(key: &str, args: &[(&str, String)]) -> String {
    let sign = ["\"", key, "\": \""].concat();
    let Some(row) = CATALOG
        .lines()
        .find_map(|l| l.trim_start().strip_prefix(sign.as_str()))
    else {
        return key.to_string();
    };
    let row = row.trim_end();
    let text = row.strip_suffix(',').unwrap_or(row);
    let text = text.strip_suffix('"').unwrap_or(text);
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find('{') {
        out += &rest[..i];
        let tail = &rest[i..];
        match tail.find('}').map(|j| (&tail[1..j], j)) {
            Some((name, j)) if args.iter().any(|(n, _)| *n == name) => {
                out += args.iter().find(|(n, _)| *n == name).map_or("", |a| &a.1);
                rest = &tail[j + 1..];
            }
            _ => {
                out.push('{');
                rest = &tail[1..];
            }
        }
    }
    out + rest
}

#[inline(never)]
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            _ => o.push(c),
        }
    }
    o
}

/// `v` rounded to `d` decimals, written from integers (no float formatter is linked into the `.wasm`), trailing zeros dropped, never `-0`.
#[inline(never)]
fn fx(v: f64, d: usize) -> String {
    let mut p = 1_u64;
    for _ in 0..d {
        p *= 10;
    }
    let n = (v * p as f64).round() as i64;
    let (ip, fp) = (n.unsigned_abs() / p, n.unsigned_abs() % p);
    let mut s = if n < 0 {
        "-".to_string()
    } else {
        String::new()
    };
    s += &ip.to_string();
    if fp > 0 {
        s += ".";
        let f = fp.to_string();
        s += &"0".repeat(d - f.len());
        s += f.trim_end_matches('0');
    }
    s
}

/// One self-closing element from its attributes (numbers already written by `fx`): one copy of the drawing code instead of a `format!` per shape.
#[inline(never)]
fn el(o: &mut String, tag: &str, at: &[(&str, &str)]) {
    o.push('<');
    o.push_str(tag);
    for (k, v) in at {
        o.push(' ');
        o.push_str(k);
        o.push_str("=\"");
        o.push_str(v);
        o.push('"');
    }
    o.push_str("/>\n");
}

/// A line from (x1, y1) to (x2, y2).
fn seg(o: &mut String, p: [f64; 4], stroke: &str, extra: &[(&str, &str)]) {
    let n = p.map(|v| fx(v, 1));
    let mut at = vec![
        ("x1", n[0].as_str()),
        ("y1", n[1].as_str()),
        ("x2", n[2].as_str()),
        ("y2", n[3].as_str()),
        ("stroke", stroke),
    ];
    at.extend_from_slice(extra);
    el(o, "line", &at);
}

/// A filled rectangle.
fn block(o: &mut String, r: [f64; 4], fill: &str, extra: &[(&str, &str)]) {
    let n = r.map(|v| fx(v, 1));
    let mut at = vec![
        ("x", n[0].as_str()),
        ("y", n[1].as_str()),
        ("width", n[2].as_str()),
        ("height", n[3].as_str()),
        ("fill", fill),
    ];
    at.extend_from_slice(extra);
    el(o, "rect", &at);
}

/// Decimals that show the span of an axis in about three significant figures.
#[inline(never)]
fn decimals(span: f64) -> usize {
    [(10.0, 0), (1.0, 1), (0.1, 2), (0.01, 3), (0.001, 4)]
        .iter()
        .find(|(m, _)| span >= *m)
        .map_or(5, |(_, d)| *d)
}

#[inline(never)]
fn text(x: f64, y: f64, size: u32, fill: &str, anchor: &str, bold: bool, s: &str) -> String {
    let mut o = String::from("<text");
    for (k, v) in [
        ("x", fx(x, 1)),
        ("y", fx(y, 1)),
        ("font-size", size.to_string()),
        ("fill", fill.to_string()),
        ("text-anchor", anchor.to_string()),
    ] {
        o += &[" ", k, "=\"", &v, "\""].concat();
    }
    if bold {
        o += " font-weight=\"700\"";
    }
    o + ">" + &esc(s) + "</text>\n"
}

/// The word and the colour of a verdict; an undetermined one carries its reason.
#[inline(never)]
fn verdict_of(t: &Test) -> (String, &'static str) {
    let word = match t.verdict {
        "consistent" => tx("note.rawiso.fig.verdict.consistent", &[]),
        "inconsistent" => tx("note.rawiso.fig.verdict.inconsistent", &[]),
        _ => {
            let why = match t.reason {
                "no_m" => tx("note.rawiso.fig.reason.no_m", &[]),
                "no_quantum" => tx("note.rawiso.fig.reason.no_quantum", &[]),
                "too_few_counts" => tx("note.rawiso.fig.reason.too_few_counts", &[]),
                "between_limits" => tx("note.rawiso.fig.reason.between_limits", &[]),
                _ => tx("note.rawiso.fig.reason.no_sigma", &[]),
            };
            [
                &tx("note.rawiso.fig.verdict.undetermined", &[]),
                " (",
                &why,
                ")",
            ]
            .concat()
        }
    };
    let colour = match t.verdict {
        "consistent" => OK,
        "inconsistent" => BAD,
        _ => MAYBE,
    };
    (word, colour)
}

/// One trace panel: the points of the scans of the window, scaled to the highest of the panel, the region shaded behind.
#[inline(never)]
fn trace_panel(
    o: &mut String,
    k: usize,
    top: f64,
    win: (f64, f64),
    at: (&[f64], &[f64]),
    label: &str,
    region: Option<(f64, f64)>,
) {
    let (rt, y) = at;
    let x_of = |t: f64| LEFT + (t - win.0) / (win.1 - win.0) * (RIGHT - LEFT);
    let bottom = top + PANEL_H;
    if let Some((a, b)) = region {
        let (xa, xb) = (x_of(a.max(win.0)), x_of(b.min(win.1)));
        block(o, [xa, top, (xb - xa).max(0.0), PANEL_H], REGION, &[]);
    }
    block(
        o,
        [LEFT, top, RIGHT - LEFT, PANEL_H],
        "none",
        &[("stroke", GRID)],
    );
    let pts: Vec<(f64, f64)> = rt
        .iter()
        .zip(y)
        .filter(|(t, _)| **t >= win.0 && **t <= win.1)
        .map(|(t, v)| (*t, *v))
        .collect();
    let top_v = pts.iter().fold(0.0_f64, |m, p| m.max(p.1));
    let line: Vec<String> = pts
        .iter()
        .map(|(t, v)| {
            let h = if top_v > 0.0 { v.max(0.0) / top_v } else { 0.0 };
            [fx(x_of(*t), 1), fx(bottom - 4.0 - h * (PANEL_H - 10.0), 1)].join(",")
        })
        .collect();
    if !line.is_empty() {
        let points = line.join(" ");
        el(
            o,
            "polyline",
            &[
                ("points", &points),
                ("fill", "none"),
                ("stroke", TRACE[k]),
                ("stroke-width", "1.6"),
                ("stroke-linejoin", "round"),
            ],
        );
    }
    *o += &text(LEFT - 8.0, top + 16.0, 12, TRACE[k], "end", true, label);
    *o += &text(
        LEFT - 8.0,
        top + 32.0,
        11,
        MUTED,
        "end",
        false,
        &tx(
            "note.rawiso.fig.peak",
            &[("v", fx(top_v, usize::from(top_v < 100.0) * 2))],
        ),
    );
}

/// The x axis under the last trace: five ticks over the window and its title.
#[inline(never)]
fn time_axis(o: &mut String, win: (f64, f64), y: f64) {
    let d = decimals((win.1 - win.0) / 4.0);
    for i in 0..=4 {
        let t = win.0 + (win.1 - win.0) * f64::from(i) / 4.0;
        let x = LEFT + (RIGHT - LEFT) * f64::from(i) / 4.0;
        seg(o, [x, y, x, y + 5.0], MUTED, &[]);
        *o += &text(x, y + 18.0, 11, INK, "middle", false, &fx(t, d));
    }
    *o += &text(
        (LEFT + RIGHT) / 2.0,
        y + 36.0,
        12,
        INK,
        "middle",
        false,
        &tx("note.rawiso.fig.axis.rt", &[]),
    );
}

/// One ratio row: the axis, the band of the consistent ratios around the expected one, the expected mark, the observed point with its sigma bar and the words.
#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn ratio_row(
    o: &mut String,
    top: f64,
    name: &str,
    obs: Option<Ratio>,
    exp: Option<f64>,
    test: Option<&Test>,
    ask: &Ask,
) {
    *o += &text(LEFT - 8.0, top + 14.0, 12, INK, "end", true, name);
    let Some(r) = obs else {
        *o += &text(
            LEFT,
            top + 14.0,
            12,
            MUTED,
            "start",
            false,
            &tx("note.rawiso.fig.ratio.none", &[]),
        );
        return;
    };
    let s = r.sigma.unwrap_or(0.0);
    let pad = if s > 0.0 {
        0.0
    } else {
        0.1 * r.r.abs().max(exp.unwrap_or(0.0)).max(1e-3)
    };
    let mut lo = r.r - 3.0 * s - pad;
    let mut hi = r.r + 3.0 * s + pad;
    if let Some(e) = exp {
        lo = lo.min(e - ask.p.z_ok * s - pad);
        hi = hi.max(e + ask.p.z_ok * s + pad);
    }
    if hi - lo <= 0.0 {
        hi = lo + 1e-3;
    }
    let x_of = |v: f64| LEFT + (v - lo) / (hi - lo) * (RIGHT - LEFT);
    let (y0, y1) = (top + 24.0, top + 56.0);
    let mid = (y0 + y1) / 2.0;
    if let (Some(e), true) = (exp, s > 0.0) {
        let (a, b) = (
            x_of((e - ask.p.z_ok * s).max(lo)),
            x_of((e + ask.p.z_ok * s).min(hi)),
        );
        block(o, [a, y0, b - a, y1 - y0], REGION, &[]);
    }
    seg(o, [LEFT, y1, RIGHT, y1], MUTED, &[]);
    let d = decimals((hi - lo) / 12.0);
    for i in 0..=4 {
        let v = lo + (hi - lo) * f64::from(i) / 4.0;
        let x = LEFT + (RIGHT - LEFT) * f64::from(i) / 4.0;
        seg(o, [x, y1, x, y1 + 4.0], MUTED, &[]);
        *o += &text(x, y1 + 15.0, 10, MUTED, "middle", false, &fx(v, d));
    }
    if let Some(e) = exp {
        let x = x_of(e);
        seg(
            o,
            [x, y0 - 4.0, x, y1 + 2.0],
            INK,
            &[("stroke-width", "2"), ("stroke-dasharray", "4 3")],
        );
    }
    let colour = test.map_or(TRACE[0], |t| verdict_of(t).1);
    if s > 0.0 {
        let (a, b) = (x_of(r.r - s), x_of(r.r + s));
        seg(o, [a, mid, b, mid], colour, &[("stroke-width", "2.5")]);
        for x in [a, b] {
            seg(
                o,
                [x, mid - 6.0, x, mid + 6.0],
                colour,
                &[("stroke-width", "2.5")],
            );
        }
    }
    el(
        o,
        "circle",
        &[
            ("cx", &fx(x_of(r.r), 1)),
            ("cy", &fx(mid, 1)),
            ("r", "5"),
            ("fill", colour),
            ("stroke", "#ffffff"),
            ("stroke-width", "1.5"),
        ],
    );
    let observed = match r.sigma {
        Some(sg) => tx(
            "note.rawiso.fig.observed",
            &[("r", fx(r.r, 4)), ("s", fx(sg, 4))],
        ),
        None => tx("note.rawiso.fig.observedOnly", &[("r", fx(r.r, 4))]),
    };
    let mut words = vec![observed];
    if let Some(e) = exp {
        words.push(tx("note.rawiso.fig.expected", &[("e", fx(e, 4))]));
    }
    if let Some(z) = test.and_then(|t| t.z) {
        words.push(tx("note.rawiso.fig.z", &[("z", fx(z, 2))]));
    }
    *o += &text(
        LEFT,
        top + 14.0,
        12,
        INK,
        "start",
        false,
        &words.join("  ·  "),
    );
    if let Some(t) = test {
        let (w, c) = verdict_of(t);
        *o += &text(RIGHT, top + 14.0, 12, c, "end", true, &w);
    }
}

/// The figure of one sample: `rt` and `y` are the whole sweep of the three bins (the window is cut here), `f` what was counted.
pub(crate) fn svg(ask: &Ask, f: &Found, rt: &[f64], y: &[Vec<f64>], centres: [f64; 3]) -> String {
    let ratios_top = TRACES_TOP + 3.0 * (PANEL_H + 10.0) + 50.0;
    let h = ratios_top + 2.0 * RATIO_ROW + 30.0;
    let hs = fx(h, 0);
    let mut o = String::new();
    o += &[
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 ",
        W_T,
        " ",
        &hs,
        "\" width=\"",
        W_T,
        "\" height=\"",
        &hs,
        "\" font-family=\"sans-serif\">\n",
    ]
    .concat();
    el(
        &mut o,
        "rect",
        &[("width", W_T), ("height", &hs), ("fill", "#ffffff")],
    );
    let title = tx(
        "note.rawiso.fig.title",
        &[
            ("mz", fx(ask.mz, 4)),
            ("a", fx(ask.rt.0, 2)),
            ("b", fx(ask.rt.1, 2)),
        ],
    );
    o += &["<title>", &esc(&title), "</title>\n"].concat();
    o += &text(LEFT - 8.0, 26.0, 16, INK, "start", true, &title);
    let sub = match &f.expected {
        Some(e) => tx(
            "note.rawiso.fig.formula",
            &[
                ("formula", e.formula.clone()),
                ("adduct", e.adduct.clone()),
                ("mz", fx(e.mz, 4)),
            ],
        ),
        None => tx("note.rawiso.fig.noFormula", &[]),
    };
    o += &text(LEFT - 8.0, 46.0, 12, MUTED, "start", false, &sub);
    let region = f.region.as_ref().map(|r| (r.lo, r.hi));
    let legend = match &f.region {
        Some(r) if r.source == "peak" => tx(
            "note.rawiso.fig.region.peak",
            &[("a", fx(r.lo, 2)), ("b", fx(r.hi, 2))],
        ),
        Some(_) => tx("note.rawiso.fig.region.window", &[]),
        None => tx("note.rawiso.fig.nodata", &[]),
    };
    let lx = fx(RIGHT - 14.0 - 6.0 - 6.0 * legend.chars().count() as f64, 1);
    el(
        &mut o,
        "rect",
        &[
            ("x", &lx),
            ("y", "54"),
            ("width", "14"),
            ("height", "10"),
            ("fill", REGION),
            ("stroke", GRID),
        ],
    );
    o += &text(RIGHT, 63.0, 11, INK, "end", false, &legend);
    let labels = [
        tx("note.rawiso.fig.trace.m", &[("mz", fx(centres[0], 0))]),
        tx("note.rawiso.fig.trace.m1", &[("mz", fx(centres[1], 0))]),
        tx("note.rawiso.fig.trace.m2", &[("mz", fx(centres[2], 0))]),
    ];
    for k in 0..3 {
        let top = TRACES_TOP + k as f64 * (PANEL_H + 10.0);
        trace_panel(&mut o, k, top, ask.rt, (rt, &y[k]), &labels[k], region);
    }
    time_axis(
        &mut o,
        ask.rt,
        TRACES_TOP + 2.0 * (PANEL_H + 10.0) + PANEL_H,
    );
    let exp = |pick: fn(&crate::rawiso_found::Expected) -> f64| f.expected.as_ref().map(pick);
    let t = f.tests.as_ref();
    ratio_row(
        &mut o,
        ratios_top,
        &tx("note.rawiso.fig.ratio.r1", &[]),
        f.r1,
        exp(|e| e.e1),
        t.map(|x| &x.0),
        ask,
    );
    ratio_row(
        &mut o,
        ratios_top + RATIO_ROW,
        &tx("note.rawiso.fig.ratio.r2", &[]),
        f.r2,
        exp(|e| e.e2),
        t.map(|x| &x.1),
        ask,
    );
    let k = fx(ask.p.z_ok, 1);
    let mut foot = vec![tx("note.rawiso.fig.legend", &[])];
    if f.expected.is_some() {
        foot.push(tx("note.rawiso.fig.band", &[("k", k)]));
    }
    o += &text(
        LEFT - 8.0,
        h - 10.0,
        11,
        MUTED,
        "start",
        false,
        &foot.join("  ·  "),
    );
    o + "</svg>\n"
}

#[cfg(test)]
#[path = "rawiso_fig_tests.rs"]
mod tests;
