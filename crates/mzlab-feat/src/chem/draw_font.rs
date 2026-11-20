//! The stroke font of the PNG of `mol draw`: capitals, digits and the few signs of an atom label as polylines on a 4 x 6 grid (y up), so the
//! rasteriser needs no font file. Lowercase letters are the capitals at 0.78 size (small capitals). The SVG uses real fonts instead.

use mzlab_mol::trig::V;

/// Width of a character cell in units of the font size (the same advance the SVG layout assumes).
pub(super) const ADVANCE: f64 = 0.66;
/// Height of a capital in units of the font size.
pub(super) const CAP: f64 = 0.72;

/// Glyphs: points `xy` (two digits) joined into strokes, strokes separated by `/`.
const GLYPHS: [(char, &str); 46] = [
    ('A', "00 26 40/12 32"),
    ('B', "00 06/06 35 34 23 03/23 42 41 30 00"),
    ('C', "41 30 10 01 05 16 36 45"),
    ('D', "00 06 36 45 41 30 00"),
    ('E', "40 00 06 46/03 33"),
    ('F', "00 06 46/03 33"),
    ('G', "45 36 16 05 01 10 30 41 43 23"),
    ('H', "00 06/40 46/03 43"),
    ('I', "10 30/20 26/16 36"),
    ('J', "46 41 30 10 01"),
    ('K', "00 06/46 03 40"),
    ('L', "06 00 40"),
    ('M', "00 06 23 46 40"),
    ('N', "00 06 40 46"),
    ('O', "10 01 05 16 36 45 41 30 10"),
    ('P', "00 06 36 45 44 33 03"),
    ('Q', "10 01 05 16 36 45 41 30 10/22 50"),
    ('R', "00 06 36 45 44 33 03/23 40"),
    ('S', "41 30 10 01 02 13 33 44 45 36 16 05"),
    ('T', "06 46/26 20"),
    ('U', "06 01 10 30 41 46"),
    ('V', "06 20 46"),
    ('W', "06 10 23 30 46"),
    ('X', "00 46/06 40"),
    ('Y', "06 23 46/23 20"),
    ('Z', "06 46 00 40"),
    ('0', "10 01 05 16 36 45 41 30 10"),
    ('1', "12 26 20/10 30"),
    ('2', "05 16 36 45 44 00 40"),
    ('3', "05 16 36 45 44 33 23/33 42 41 30 10 01"),
    ('4', "30 36 03 43"),
    ('5', "46 06 03 33 42 41 30 10 01"),
    ('6', "45 36 16 05 01 10 30 41 42 33 13 02"),
    ('7', "06 46 20"),
    ('8', "13 04 05 16 36 45 44 33 13 02 01 10 30 41 42 33"),
    ('9', "01 10 30 41 45 36 16 05 04 13 33 44"),
    ('+', "03 43/21 25"),
    ('-', "03 43"),
    ('*', "03 43/12 34/14 32"),
    ('(', "36 15 11 30"),
    (')', "16 35 31 10"),
    ('?', "05 16 36 45 44 33 23 22/20 20"),
    (' ', ""),
    ('.', "10 20"),
    (',', "20 10"),
    ('/', "00 46"),
];

/// Stroke width of text of this size.
pub(super) fn stroke_width(size: f64) -> f64 {
    (0.11 * size).max(0.9)
}

/// The segments of `text` with its first cell at `x` on the baseline `y` (y down), as pairs of end points.
pub(super) fn strokes(text: &str, x: f64, y: f64, size: f64) -> Vec<(V, V)> {
    let mut out = Vec::new();
    let mut left = x;
    for ch in text.chars() {
        let small = ch.is_ascii_lowercase();
        let up = ch.to_ascii_uppercase();
        let k = if small { 0.78 } else { 1.0 };
        let g = GLYPHS.iter().find(|g| g.0 == up).map_or("", |g| g.1);
        // the glyph is centred in its cell; small capitals sit on the baseline
        let (gw, gh) = (0.5 * size * k, CAP * size * k);
        let x0 = left + (ADVANCE * size - gw) / 2.0;
        for stroke in g.split('/') {
            let mut prev: Option<V> = None;
            for pt in stroke.split(' ').filter(|p| p.len() == 2) {
                let b = pt.as_bytes();
                let (gx, gy) = (f64::from(b[0] - b'0'), f64::from(b[1] - b'0'));
                let p = [x0 + gx * gw / 4.0, y - gy * gh / 6.0];
                if let Some(q) = prev {
                    out.push((q, p));
                }
                prev = Some(p);
            }
        }
        left += ADVANCE * size;
    }
    out
}
