//! The picture of `mol draw`: the drawn structure becomes a list of primitives (lines, polygons, discs, text runs) in pixels, written as SVG
//! here and rasterised by `draw_png`. Rules: carbon is a vertex unless it is charged, isotopic, alone or labelled by an edit patch; every other atom shows its symbol
//! with its hydrogens (`OH`, `H2N`, `NH3+`), the hydrogens on the side without bonds (a patch label replaces symbol and hydrogens); bonds stop at the label; a double bond sits inside its ring or
//! on the side of its substituents, centred when it ends in a terminal atom or both sides are alike; wedges are solid, hashed or wavy;
//! CIP marks (R/S/r/s, E/Z) go into the free angle beside their atom or bond; highlights are pale halos under the structure. Arithmetic is
//! `+ - * /` and `sqrt` only, so the bytes are the same in every build.

use super::draw::Drawn;
use super::draw_hl::Hl;
use super::structure_file::base64;
use mzlab_core::json::Out;
use mzlab_mol::trig::V;

pub(super) type Rgb = [u8; 3];
pub(super) const INK: Rgb = [0, 0, 0];

pub(super) enum Prim {
    Line(V, V, f64, Rgb),
    Poly(Vec<V>, Rgb),
    Disc(V, f64, Rgb),
    /// Text: first cell at `(x, baseline)`, font size.
    Run(V, f64, String, Rgb),
}

/// A label with its box `[left, top, right, bottom]` in pixels: an atom symbol (`atom`) or a CIP mark (`cip`).
pub struct Label {
    pub kind: &'static str,
    pub atom: u32,
    pub other: Option<u32>,
    pub text: String,
    pub bx: [f64; 4],
}

pub struct Figure {
    pub w: u32,
    pub h: u32,
    pub x0: f64,
    pub y0: f64,
    pub scale: f64,
    pub(super) prims: Vec<Prim>,
    pub labels: Vec<Label>,
}

pub struct Options {
    pub scale: f64,
    pub cip: bool,
    pub hl: Vec<Hl>,
}

/// A pixel number with at most 2 decimals, shortest form.
pub(super) fn num(v: f64) -> String {
    let s = mzlab_mol::fixed(v, 2);
    let t = s.trim_end_matches('0').trim_end_matches('.');
    if t == "-0" || t.is_empty() {
        "0".into()
    } else {
        t.into()
    }
}

fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Text for an XML element: `& < >` escaped, characters XML 1.0 forbids (controls but tab, CR and LF, U+FFFE and U+FFFF) dropped.
fn escape(t: &str) -> String {
    let mut o = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '&' => o += "&amp;",
            '<' => o += "&lt;",
            '>' => o += "&gt;",
            '\t' | '\n' | '\r' => o.push(c),
            c if c < ' ' || c == '\u{fffe}' || c == '\u{ffff}' => {}
            c => o.push(c),
        }
    }
    o
}

/// The SVG text of a figure; `title` becomes its accessible name.
pub fn svg(f: &Figure, title: &str) -> String {
    let (w, h) = (f.w, f.h);
    let mut o = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{} {} {w} {h}\" font-family=\"Arial, Helvetica, sans-serif\" stroke-linecap=\"round\">\n",
        num(f.x0),
        num(f.y0)
    );
    if !title.is_empty() {
        o += &format!("<title>{}</title>\n", escape(title));
    }
    o += &format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" fill=\"#ffffff\"/>\n",
        num(f.x0),
        num(f.y0)
    );
    for p in &f.prims {
        o += &match p {
            Prim::Line(a, b, lw, c) => format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>\n",
                num(a[0]), num(a[1]), num(b[0]), num(b[1]), hex(*c), num(*lw)
            ),
            Prim::Poly(pts, c) => format!(
                "<polygon points=\"{}\" fill=\"{}\" stroke=\"none\"/>\n",
                pts.iter().map(|p| format!("{},{}", num(p[0]), num(p[1]))).collect::<Vec<_>>().join(" "),
                hex(*c)
            ),
            Prim::Disc(c, r, col) => format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>\n",
                num(c[0]), num(c[1]), num(*r), hex(*col)
            ),
            Prim::Run(p, size, text, c) => format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
                num(p[0]), num(p[1]), num(*size), hex(*c), escape(text)
            ),
        };
    }
    o + "</svg>\n"
}

/// The `draw-image/1` wrapper: the picture (`svg` text or `png_base64`) with its size, scale and the boxes of its labels, every length in pixels
/// of the picture (origin at its top left corner), plus what was chosen on the way.
pub fn wrapped(d: &Drawn, f: &Figure, png: bool, body: &[u8]) -> String {
    let mut o = Out::default();
    o.raw("{\"schema\":\"draw-image/1\",\"spec_kind\":")
        .str(d.kind);
    o.raw(",\"title\":").str(&d.title);
    o.raw(",\"format\":").str(if png { "png" } else { "svg" });
    o.raw(",\"width_px\":").int(i64::from(f.w));
    o.raw(",\"height_px\":").int(i64::from(f.h));
    o.raw(",\"px_per_angstrom\":").raw(&num(f.scale));
    o.raw(",\"labels\":[");
    for (i, l) in f.labels.iter().enumerate() {
        o.raw(if i > 0 { ",{\"kind\":" } else { "{\"kind\":" })
            .str(l.kind);
        o.raw(",\"atom\":").int(i64::from(l.atom));
        if let Some(b) = l.other {
            o.raw(",\"other\":").int(i64::from(b));
        }
        o.raw(",\"text\":").str(&l.text);
        for (k, v) in ["left_px", "top_px", "right_px", "bottom_px"]
            .iter()
            .zip(l.bx)
        {
            let at = if k.starts_with('l') || k.starts_with('r') {
                f.x0
            } else {
                f.y0
            };
            o.raw(",\"").raw(k).raw("\":").raw(&num(v - at));
        }
        o.raw("}");
    }
    o.raw("]");
    for (key, list) in [("choices", &d.choices), ("warnings", &d.warnings)] {
        o.raw(",").str(key).raw(":[");
        for (i, n) in list.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" })
                .raw("{\"key\":")
                .str(n.key);
            o.raw(",\"value\":").str(&n.value).raw("}");
        }
        o.raw("]");
    }
    if png {
        o.raw(",\"png_base64\":").str(&base64(body));
    } else {
        o.raw(",\"svg\":").str(&String::from_utf8_lossy(body));
    }
    o.raw("}");
    o.0
}

#[cfg(test)]
#[path = "draw_svg_tests.rs"]
mod tests;
