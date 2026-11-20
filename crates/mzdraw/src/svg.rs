//! The display list as an SVG file: the same primitives the page paints, with the palette of the file. Deterministic text (numbers
//! to 2 decimals), no editor overlay.

use crate::doc::Document;
use crate::scene::{self, Overlay, View};
use mzlab_mol::fixed;

/// Pixels per bond length, label size and margin of an exported drawing.
const SCALE: f64 = 30.0;
const FONT: f64 = 14.0;
const MARGIN: f64 = 16.0;

/// File colours by colour index (ink, then the overlay colours that a file never uses, then N, O, S, halogens, P).
const PALETTE: [&str; 10] = [
    "#000000", "#000000", "#000000", "#000000", "#000000", "#2040c0", "#c02020", "#8a6d00",
    "#1f7a1f", "#b05a00",
];

fn num(v: f32) -> String {
    let s = fixed(v as f64, 2);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The whole document as an SVG of its own size (an empty document gives an empty 1x1 drawing).
pub fn to_svg(doc: &Document) -> String {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for a in &doc.atoms {
        (x0, y0, x1, y1) = (x0.min(a.x), y0.min(a.y), x1.max(a.x), y1.max(a.y));
    }
    if doc.atoms.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\" viewBox=\"0 0 1 1\"></svg>\n".into();
    }
    // labels can stick out by about two characters on each side
    let pad = MARGIN + FONT * 1.6;
    let view = View {
        scale: SCALE,
        ox: pad - x0 * SCALE,
        oy: pad + y1 * SCALE,
    };
    let (w, h) = ((x1 - x0) * SCALE + 2.0 * pad, (y1 - y0) * SCALE + 2.0 * pad);
    let sc = scene::build(doc, view, &Overlay::default(), FONT);
    let (w, h) = (num(w as f32), num(h as f32));
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" font-family=\"Arial, Helvetica, sans-serif\" stroke-linecap=\"round\">\n"
    );
    let d = &sc.data;
    let mut i = 0;
    while i < d.len() {
        let op = d[i];
        if op == scene::LINE {
            s.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>\n",
                num(d[i + 1]), num(d[i + 2]), num(d[i + 3]), num(d[i + 4]), PALETTE[d[i + 6] as usize % 10], num(d[i + 5])
            ));
            i += 7;
        } else if op == scene::TEXT {
            let anchor = if d[i + 6] == 0.0 { "middle" } else { "start" };
            s.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\" text-anchor=\"{anchor}\" dominant-baseline=\"central\">{}</text>\n",
                num(d[i + 1]), num(d[i + 2]), num(d[i + 3]), PALETTE[d[i + 4] as usize % 10], escape(&sc.strings[d[i + 5] as usize])
            ));
            i += 7;
        } else if op == scene::RECT {
            i += 6;
        } else {
            i += 5;
        }
    }
    s.push_str("</svg>\n");
    s
}
