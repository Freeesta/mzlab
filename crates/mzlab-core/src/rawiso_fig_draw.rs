//! Drawing primitives of the `raw isotopes` figure (`rawiso_fig`): the text of a `note.rawiso.fig.*` key from `lang/en/figure-raw.js`, numbers written from
//! integers, SVG elements from their attributes, and the palette (Okabe-Ito on white, as the other static figures).

use crate::rawiso_found::Test;

pub(crate) const CATALOG: &str = include_str!("../../../mzlab/web/lang/en/figure-raw.js");

pub(crate) const INK: &str = "#1a1a1a";
pub(crate) const MUTED: &str = "#595959";
pub(crate) const GRID: &str = "#d9d9d9";
pub(crate) const REGION: &str = "#cfe3f3";
/// Okabe-Ito: blue (M), orange (M+1), bluish green (M+2).
pub(crate) const TRACE: [&str; 3] = ["#0072B2", "#E69F00", "#009E73"];
pub(crate) const OK: &str = "#009E73";
pub(crate) const BAD: &str = "#D55E00";
pub(crate) const MAYBE: &str = "#7f7f7f";

pub(crate) const W_T: &str = "960";
pub(crate) const LEFT: f64 = 180.0;
pub(crate) const RIGHT: f64 = 930.0;
pub(crate) const PANEL_H: f64 = 84.0;
pub(crate) const TRACES_TOP: f64 = 74.0;
pub(crate) const RATIO_ROW: f64 = 92.0;

/// The text of `key` in the catalogue (one `"key": "text",` per line), its `{name}` filled in one pass; the key itself when it has no text.
#[inline(never)]
pub(crate) fn tx(key: &str, args: &[(&str, String)]) -> String {
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
pub(crate) fn esc(s: &str) -> String {
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
pub(crate) fn fx(v: f64, d: usize) -> String {
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
pub(crate) fn el(o: &mut String, tag: &str, at: &[(&str, &str)]) {
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
pub(crate) fn seg(o: &mut String, p: [f64; 4], stroke: &str, extra: &[(&str, &str)]) {
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
pub(crate) fn block(o: &mut String, r: [f64; 4], fill: &str, extra: &[(&str, &str)]) {
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
pub(crate) fn decimals(span: f64) -> usize {
    [(10.0, 0), (1.0, 1), (0.1, 2), (0.01, 3), (0.001, 4)]
        .iter()
        .find(|(m, _)| span >= *m)
        .map_or(5, |(_, d)| *d)
}

#[inline(never)]
pub(crate) fn text(
    x: f64,
    y: f64,
    size: u32,
    fill: &str,
    anchor: &str,
    bold: bool,
    s: &str,
) -> String {
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
pub(crate) fn verdict_of(t: &Test) -> (String, &'static str) {
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
