// Ported from Attributio src/attributio/review/evidence.py (mirror_plot).
//! The mirror plot of a decision as an SVG written by hand, and the table of what it draws: the experimental spectrum on top with the explained fragments
//! labelled (m/z, sub-formula, ppm), the reference below (shifted by delta m for an analog; the peaks matched through delta m in violet). Every peak drawn is
//! also a row of the CSV that goes with the plot.

use super::proposal::PeakView;

const EXP: &str = "#1f4e79";
const REF: &str = "#c55a11";
const SHIFTED: &str = "#7a3e9d";
const EXP_DIM: &str = "#9fb6d0";
const REF_DIM: &str = "#efc4a3";
const W: f64 = 1000.0;
const H: f64 = 560.0;
const LEFT: f64 = 70.0;
const RIGHT: f64 = 980.0;
const TOP: f64 = 36.0;
const BOTTOM: f64 = 500.0;

/// The reference as it is drawn: its peaks (m/z, % of the base peak below the precursor), the shift and the pairs `(experimental, reference, shifted)`.
pub struct RefDraw {
    pub mz: Vec<f64>,
    pub rel: Vec<f64>,
    pub shift: f64,
    pub pairs: Vec<(usize, usize, bool)>,
}

/// What a plot needs.
pub struct Plot<'a> {
    pub title: String,
    pub peaks: &'a [PeakView],
    pub reference: Option<RefDraw>,
    pub caption: Vec<String>,
}

/// The reference peaks as the page normalises them: the base peak below the precursor is 100, none above 110.
pub fn reference_rel(mz: &[f64], it: &[f64], prec: f64) -> Vec<f64> {
    let max = |v: Vec<f64>| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
    let below: Vec<f64> = mz
        .iter()
        .zip(it)
        .filter(|(m, _)| **m < prec - 0.5)
        .map(|(_, v)| *v)
        .collect();
    let base = if below.is_empty() {
        max(it.to_vec())
    } else {
        max(below)
    };
    it.iter()
        .map(|v| {
            if base > 0.0 {
                (v / base * 100.0).min(110.0)
            } else {
                0.0
            }
        })
        .collect()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn nice_step(range: f64) -> f64 {
    let raw = range / 6.0;
    let mag = 10f64.powf(raw.log10().floor());
    let f = raw / mag;
    mag * if f <= 1.0 {
        1.0
    } else if f <= 2.0 {
        2.0
    } else if f <= 5.0 {
        5.0
    } else {
        10.0
    }
}

struct Frame {
    lo: f64,
    hi: f64,
    ymin: f64,
}

impl Frame {
    fn x(&self, m: f64) -> f64 {
        LEFT + (m - self.lo) / (self.hi - self.lo) * (RIGHT - LEFT)
    }
    fn y(&self, v: f64) -> f64 {
        TOP + (128.0 - v) / (128.0 - self.ymin) * (BOTTOM - TOP)
    }
}

fn frame(p: &Plot) -> Frame {
    let mut all: Vec<f64> = p.peaks.iter().map(|k| k.mz).collect();
    if let Some(r) = &p.reference {
        all.extend(r.mz.iter().map(|m| m + r.shift));
    }
    let lo = all.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = all.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let (lo, hi) = if lo.is_finite() && hi > lo {
        (lo, hi)
    } else {
        (0.0, 1.0)
    };
    let pad = (hi - lo) * 0.04;
    Frame {
        lo: lo - pad,
        hi: hi + pad,
        ymin: if p.reference.is_some() { -118.0 } else { -5.0 },
    }
}

fn axes(fr: &Frame, with_ref: bool, out: &mut String) {
    let step = nice_step(fr.hi - fr.lo);
    let mut m = (fr.lo / step).ceil() * step;
    while m <= fr.hi {
        let x = fr.x(m);
        out.push_str(&format!(
            "<line x1=\"{x:.1}\" y1=\"{BOTTOM}\" x2=\"{x:.1}\" y2=\"{:.1}\" stroke=\"#333\" stroke-width=\"1\"/><text x=\"{x:.1}\" y=\"{:.1}\" font-size=\"13\" text-anchor=\"middle\">{}</text>\n",
            BOTTOM + 5.0,
            BOTTOM + 20.0,
            if step < 1.0 { format!("{m:.2}") } else { format!("{m:.0}") }
        ));
        m += step;
    }
    let ticks: &[f64] = if with_ref {
        &[-100.0, -50.0, 0.0, 50.0, 100.0]
    } else {
        &[0.0, 50.0, 100.0]
    };
    for t in ticks {
        let y = fr.y(*t);
        out.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{y:.1}\" x2=\"{LEFT}\" y2=\"{y:.1}\" stroke=\"#333\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"13\" text-anchor=\"end\">{}</text>\n",
            LEFT - 5.0,
            LEFT - 8.0,
            y + 4.0,
            t.abs()
        ));
    }
    out.push_str(&format!(
        "<line x1=\"{LEFT}\" y1=\"{TOP}\" x2=\"{LEFT}\" y2=\"{BOTTOM}\" stroke=\"#333\"/><line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{RIGHT}\" y2=\"{:.1}\" stroke=\"#333\" stroke-width=\"1.2\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"14\" text-anchor=\"middle\">m/z</text>\n<text transform=\"translate(18 {:.1}) rotate(-90)\" font-size=\"14\" text-anchor=\"middle\">relative intensity (%)</text>\n",
        LEFT, fr.y(0.0), fr.y(0.0), (LEFT + RIGHT) / 2.0, H - 10.0, (TOP + BOTTOM) / 2.0
    ));
}

fn matched_state(p: &Plot) -> (Vec<Option<bool>>, Vec<Option<bool>>) {
    let mut e = vec![None; p.peaks.len()];
    let mut r = vec![None; p.reference.as_ref().map_or(0, |r| r.mz.len())];
    for &(i, j, sh) in p.reference.iter().flat_map(|r| r.pairs.iter()) {
        if i < e.len() {
            e[i] = Some(sh);
        }
        if j < r.len() {
            r[j] = Some(sh);
        }
    }
    (e, r)
}

fn peak_line(fr: &Frame, x: f64, v: f64, color: &str) -> String {
    format!(
        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{color}\" stroke-width=\"2\"/>\n",
        fr.x(x),
        fr.y(0.0),
        fr.x(x),
        fr.y(v)
    )
}

/// The explained fragments, largest first, not overlapping (4 % of the span), at most 12 labels.
fn labels(fr: &Frame, p: &Plot, out: &mut String) {
    let mut order: Vec<usize> = (0..p.peaks.len()).collect();
    order.sort_by(|&a, &b| p.peaks[b].rel.total_cmp(&p.peaks[a].rel));
    let span = if p.peaks.len() > 1 {
        let (lo, hi) = p
            .peaks
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |a, k| {
                (a.0.min(k.mz), a.1.max(k.mz))
            });
        hi - lo
    } else {
        1.0
    };
    let mut used: Vec<f64> = Vec::new();
    for &i in order.iter().take(12) {
        let k = &p.peaks[i];
        let Some(f) = &k.frag else { continue };
        if k.rel < 5.0 || used.iter().any(|u| (k.mz - u).abs() < span * 0.04) {
            continue;
        }
        used.push(k.mz);
        let (x, y) = (fr.x(k.mz), fr.y(k.rel) - 6.0);
        out.push_str(&format!(
            "<text x=\"{x:.1}\" y=\"{:.1}\" font-size=\"10\" text-anchor=\"middle\" fill=\"#2e7d4f\">{:.4}</text><text x=\"{x:.1}\" y=\"{y:.1}\" font-size=\"10\" text-anchor=\"middle\" fill=\"#2e7d4f\">{} {:+.1} ppm</text>\n",
            y - 12.0,
            k.mz,
            esc(&f.formula),
            f.ppm
        ));
    }
}

/// The SVG of the plot.
pub fn svg(p: &Plot) -> String {
    let fr = frame(p);
    let (me, mr) = matched_state(p);
    let any = p.reference.as_ref().is_some_and(|r| !r.pairs.is_empty());
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {W} {H}\" width=\"{W}\" height=\"{H}\" font-family=\"sans-serif\">\n<title>{}</title>\n<rect width=\"{W}\" height=\"{H}\" fill=\"#fff\"/>\n<text x=\"{LEFT}\" y=\"22\" font-size=\"14\">{}</text>\n",
        esc(&p.title),
        esc(&p.title)
    );
    axes(&fr, p.reference.is_some(), &mut s);
    for (i, k) in p.peaks.iter().enumerate() {
        let c = match me[i] {
            Some(true) => SHIFTED,
            Some(false) => EXP,
            None if !any => EXP,
            None => EXP_DIM,
        };
        s.push_str(&peak_line(&fr, k.mz, k.rel, c));
    }
    labels(&fr, p, &mut s);
    if let Some(r) = &p.reference {
        for (j, (&m, &v)) in r.mz.iter().zip(&r.rel).enumerate() {
            let c = match mr[j] {
                Some(true) => SHIFTED,
                Some(false) => REF,
                None => REF_DIM,
            };
            s.push_str(&peak_line(&fr, m + r.shift, -v, c));
        }
    }
    for (n, line) in p.caption.iter().enumerate() {
        let y = BOTTOM - 8.0 - 13.0 * (p.caption.len() - 1 - n) as f64;
        s.push_str(&format!(
            "<text x=\"{:.0}\" y=\"{y:.0}\" font-size=\"11\" fill=\"#333\">{}</text>\n",
            LEFT + 8.0,
            esc(line)
        ));
    }
    s.push_str("</svg>\n");
    s
}

/// The rows of the CSV of the plot: `side;mz;drawn_x;relative_intensity;matched;fragment;ppm`.
pub const PLOT_COLUMNS: [&str; 7] = [
    "side",
    "mz",
    "drawn_x",
    "relative_intensity",
    "matched",
    "fragment",
    "ppm",
];

pub fn rows(p: &Plot) -> Vec<Vec<String>> {
    let (me, mr) = matched_state(p);
    let state = |m: Option<bool>| match m {
        Some(true) => "shifted",
        Some(false) => "matched",
        None => "",
    };
    let mut out = Vec::new();
    for (i, k) in p.peaks.iter().enumerate() {
        out.push(vec![
            "experimental".into(),
            k.mz.to_string(),
            k.mz.to_string(),
            k.rel.to_string(),
            state(me[i]).into(),
            k.frag.as_ref().map_or(String::new(), |f| f.formula.clone()),
            k.frag.as_ref().map_or(String::new(), |f| f.ppm.to_string()),
        ]);
    }
    if let Some(r) = &p.reference {
        for (j, (&m, &v)) in r.mz.iter().zip(&r.rel).enumerate() {
            out.push(vec![
                "reference".into(),
                m.to_string(),
                (m + r.shift).to_string(),
                v.to_string(),
                state(mr[j]).into(),
                String::new(),
                String::new(),
            ]);
        }
    }
    out
}
