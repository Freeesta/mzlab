//! The PNG of `mol draw`, by a minimal rasteriser of its own: every primitive of the figure is covered with 4 x 4 sample points per pixel and
//! blended onto white (anti-aliased, `+ - * /` and `sqrt` only), the text through the stroke font; the file is RGB, 8 bits, zlib "stored"
//! blocks (no compression, no dependency). The same bytes in the `.wasm`, the app and the CLI.

use super::draw_font::{stroke_width, strokes};
use super::draw_svg::{Figure, Prim};
use mzlab_mol::trig::{dist, V};

const N: usize = 4;

/// Squared distance from `p` to the segment `a`-`b`.
fn seg_d2(p: V, a: V, b: V) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (qx, qy) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    qx * qx + qy * qy
}

/// Even-odd point in polygon.
fn inside(p: V, pts: &[V]) -> bool {
    let mut c = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[j]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            c = !c;
        }
        j = i;
    }
    c
}

/// Paints the shape `hit` (a test on a point) over its bounding box `[x0, y0, x1, y1]` in pixels.
fn paint(img: &mut [u8], w: usize, h: usize, bb: [f64; 4], col: [u8; 3], hit: impl Fn(V) -> bool) {
    let lo = |v: f64, max: usize| (v.floor().max(0.0) as usize).min(max);
    let (x0, x1) = (lo(bb[0], w), lo(bb[2] + 1.0, w));
    let (y0, y1) = (lo(bb[1], h), lo(bb[3] + 1.0, h));
    for y in y0..y1 {
        for x in x0..x1 {
            let mut n = 0;
            for j in 0..N {
                for i in 0..N {
                    let p = [
                        x as f64 + (i as f64 + 0.5) / N as f64,
                        y as f64 + (j as f64 + 0.5) / N as f64,
                    ];
                    n += usize::from(hit(p));
                }
            }
            if n > 0 {
                let a = n as f64 / (N * N) as f64;
                for (k, c) in col.iter().enumerate() {
                    let o = &mut img[(y * w + x) * 3 + k];
                    *o = (f64::from(*o) + (f64::from(*c) - f64::from(*o)) * a + 0.5).floor() as u8;
                }
            }
        }
    }
}

fn capsule(img: &mut [u8], w: usize, h: usize, a: V, b: V, lw: f64, col: [u8; 3]) {
    let r = lw / 2.0;
    let bb = [
        a[0].min(b[0]) - r,
        a[1].min(b[1]) - r,
        a[0].max(b[0]) + r,
        a[1].max(b[1]) + r,
    ];
    paint(img, w, h, bb, col, |p| seg_d2(p, a, b) <= r * r);
}

/// The RGB pixels of a figure (white background), `w * h * 3` bytes.
fn pixels(f: &Figure) -> Vec<u8> {
    let (w, h) = (f.w as usize, f.h as usize);
    let mut img = vec![255u8; w * h * 3];
    let to = |p: V| [p[0] - f.x0, p[1] - f.y0];
    for prim in &f.prims {
        match prim {
            Prim::Line(a, b, lw, c) => capsule(&mut img, w, h, to(*a), to(*b), *lw, *c),
            Prim::Poly(pts, c) => {
                let q: Vec<V> = pts.iter().map(|p| to(*p)).collect();
                let bb = q
                    .iter()
                    .fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, p| {
                        [
                            b[0].min(p[0]),
                            b[1].min(p[1]),
                            b[2].max(p[0]),
                            b[3].max(p[1]),
                        ]
                    });
                paint(&mut img, w, h, bb, *c, |p| inside(p, &q));
            }
            Prim::Disc(c, r, col) => {
                let c = to(*c);
                paint(
                    &mut img,
                    w,
                    h,
                    [c[0] - r, c[1] - r, c[0] + r, c[1] + r],
                    *col,
                    |p| dist(p, c) <= *r,
                );
            }
            Prim::Run(p, size, text, c) => {
                let p = to(*p);
                for (a, b) in strokes(text, p[0], p[1], *size) {
                    capsule(&mut img, w, h, a, b, stroke_width(*size), *c);
                }
            }
        }
    }
    img
}

fn chunk(o: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    o.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    o.extend_from_slice(&body);
    o.extend_from_slice(&mzlab_core::zip::crc32(&body).to_be_bytes());
}

/// A zlib stream of "stored" blocks and its Adler-32.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut z = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65535).peekable();
    if blocks.peek().is_none() {
        z.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    while let Some(b) = blocks.next() {
        z.push(u8::from(blocks.peek().is_none()));
        z.extend_from_slice(&(b.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(b.len() as u16)).to_le_bytes());
        z.extend_from_slice(b);
    }
    let (mut a, mut s) = (1u32, 0u32);
    for &v in raw {
        a = (a + u32::from(v)) % 65521;
        s = (s + a) % 65521;
    }
    z.extend_from_slice(&((s << 16) | a).to_be_bytes());
    z
}

/// The PNG file of a figure.
pub fn png(f: &Figure) -> Vec<u8> {
    let (w, h) = (f.w as usize, f.h as usize);
    let img = pixels(f);
    let mut raw = Vec::with_capacity(h * (w * 3 + 1));
    for row in img.chunks(w * 3) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut o = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut head = Vec::new();
    head.extend_from_slice(&f.w.to_be_bytes());
    head.extend_from_slice(&f.h.to_be_bytes());
    head.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut o, b"IHDR", &head);
    chunk(&mut o, b"IDAT", &zlib_stored(&raw));
    chunk(&mut o, b"IEND", &[]);
    o
}
