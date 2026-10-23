//! The drawing half of the static figure: the molecule of the root, the arcs and the boxes as SVG elements. Child of `msn_figura`, which gives it the layout.

use std::collections::HashSet;

use super::*;

// ---- molecule ---------------------------------------------------------------------------------------------------------

/// The molecule of a molfile as lines and symbols from its 2D coordinates (inner SVG elements in a box of DW × DH), or None when the file has no 2D coordinates.
pub(super) fn molecola(molfile: &str) -> Option<String> {
    let m = mzlab_mol::Mol::from_molfile(molfile).ok()?;
    let xy = &m.coords.as_ref()?.xy;
    if xy.len() != m.atoms.len() || xy.iter().any(|p| p.len() < 2) {
        return None;
    }
    let (x0, x1) = xy.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| {
        (a.0.min(p[0]), a.1.max(p[0]))
    });
    let (y0, y1) = xy.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| {
        (a.0.min(p[1]), a.1.max(p[1]))
    });
    if x1 - x0 < 1e-6 && y1 - y0 < 1e-6 {
        return None;
    }
    let pad = 10.0;
    let k = ((DW - 2.0 * pad) / (x1 - x0).max(1e-6))
        .min((DH - 2.0 * pad) / (y1 - y0).max(1e-6))
        .min(30.0);
    let at = |p: &Vec<f64>| {
        (
            (DW - k * (x1 - x0)) / 2.0 + k * (p[0] - x0),
            (DH - k * (y1 - y0)) / 2.0 + k * (y1 - p[1]),
        )
    };
    let mut out = String::new();
    for b in &m.bonds {
        let (p, q) = (at(&xy[b.a as usize]), at(&xy[b.b as usize]));
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let len = dx.hypot(dy).max(1e-6);
        let (nx, ny) = (-dy / len * 2.2, dx / len * 2.2);
        let offs: &[f64] = match b.o {
            2 => &[-0.5, 0.5],
            3 => &[-1.0, 0.0, 1.0],
            _ => &[0.0],
        };
        for o in offs {
            out += &format!(
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{INK}\" stroke-width=\"1.4\" stroke-linecap=\"round\"/>",
                p.0 + nx * o,
                p.1 + ny * o,
                q.0 + nx * o,
                q.1 + ny * o
            );
        }
    }
    for (a, p) in m.atoms.iter().zip(xy) {
        if a.el == "C" {
            continue;
        }
        let (x, y) = at(p);
        out += &format!(
            "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"6\" fill=\"#ffffff\"/><text x=\"{x:.1}\" y=\"{:.1}\" font-family=\"sans-serif\" font-size=\"11\" font-weight=\"700\" text-anchor=\"middle\" fill=\"{INK}\">{}</text>",
            y + 4.0,
            esc(&a.el)
        );
    }
    Some(out)
}

// ---- drawing ----------------------------------------------------------------------------------------------------------

pub(crate) fn esc(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(super) fn num(v: f64) -> String {
    let t = format!("{v:.2}");
    t.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A `<text>` with its pieces: subscripts and the charge in tspans.
pub(super) fn testo(at: &str, parti: &[Pezzo]) -> String {
    let mut o = format!("<text {at}>");
    for p in parti {
        match p {
            Pezzo::N(t) => o += &esc(t),
            Pezzo::Sub(t) => {
                o += &format!(
                    "<tspan baseline-shift=\"sub\" font-size=\"75%\">{}</tspan>",
                    esc(t)
                )
            }
            Pezzo::Sup(t) => {
                o += &format!(
                    "<tspan baseline-shift=\"super\" font-size=\"75%\">{}</tspan>",
                    esc(t)
                )
            }
        }
    }
    o + "</text>"
}

pub(super) fn bordo(b: &Box_, tx: f64, ty: f64) -> (f64, f64) {
    let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
    let (dx, dy) = (tx - cx, ty - cy);
    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }
    let k = (if dx != 0.0 {
        b.w / 2.0 / dx.abs()
    } else {
        f64::INFINITY
    })
    .min(if dy != 0.0 {
        b.h / 2.0 / dy.abs()
    } else {
        f64::INFINITY
    });
    (cx + dx * k, cy + dy * k)
}

pub(super) fn archi_svg(sc: &Scena, d: &Disposizione, vista: Vista, vis: &HashSet<&str>) -> String {
    let mut arcs: Vec<&Arco> = sc
        .archi
        .iter()
        .filter(|a| vis.contains(a.da.as_str()) && vis.contains(a.a.as_str()))
        .collect();
    arcs.sort_by_key(|a| sc.best.contains(&format!("{}>{}", a.da, a.a)));
    let mut o = String::new();
    for a in arcs {
        let (p, q) = (d.caselle[&a.da], d.caselle[&a.a]);
        let best = sc.best.contains(&format!("{}>{}", a.da, a.a));
        let rel = a.misure.iter().map(|m| m.rel).fold(0.0f64, f64::max);
        let (stroke, width, dash) = if best {
            (BEST, format!("{:.2}", 1.0 + (1.0 + rel).log10() * 1.2), "")
        } else {
            (LINE, "1".into(), " stroke-dasharray=\"4 3\"")
        };
        let (lx, ly);
        if vista == Vista::Albero {
            let (x1, y1, x2, y2) = (p.x + p.w, p.y + p.h / 2.0, q.x, q.y + q.h / 2.0);
            let mx = (x1 + x2) / 2.0;
            o += &format!(
                "<path d=\"M{},{} C{},{} {},{} {},{}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{width}\"{dash} marker-end=\"url(#h-{})\"/>",
                num(x1), num(y1), num(mx), num(y1), num(mx), num(y2), num(x2), num(y2), if best { "b" } else { "a" }
            );
            lx = mx;
            ly = (y1 + y2) / 2.0;
        } else {
            let (s0, e0) = (
                bordo(&p, q.x + q.w / 2.0, q.y + q.h / 2.0),
                bordo(&q, p.x + p.w / 2.0, p.y + p.h / 2.0),
            );
            o += &format!(
                "<path d=\"M{},{} L{},{}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{width}\"{dash} marker-end=\"url(#h-{})\"/>",
                num(s0.0), num(s0.1), num(e0.0), num(e0.1), if best { "b" } else { "a" }
            );
            lx = (s0.0 + e0.0) / 2.0;
            ly = (s0.1 + e0.1) / 2.0;
        }
        if vista == Vista::Radiale && !best {
            continue;
        }
        let (first, second) = perdita_testo(a);
        o += &format!("<g><title>{}</title>", esc(&tip_perdita(a)));
        let halo = format!("font-family=\"sans-serif\" font-size=\"10\" fill=\"{INK}\" text-anchor=\"middle\" stroke=\"#ffffff\" stroke-width=\"3\" paint-order=\"stroke\"");
        o += &testo(
            &format!("x=\"{}\" y=\"{}\" {halo}", num(lx), num(ly - 3.0)),
            &first,
        );
        let mut dy = 9.0;
        if let Some(sec) = second {
            o += &testo(
                &format!("x=\"{}\" y=\"{}\" {halo}", num(lx), num(ly + dy)),
                &sec,
            );
            dy += 11.0;
        }
        if !a.regola.is_empty() {
            o += &format!(
                "<text x=\"{}\" y=\"{}\" {halo}>{}</text>",
                num(lx),
                num(ly + dy),
                esc(&a.regola)
            );
        }
        o += "</g>";
    }
    o
}

pub(super) fn nodo_svg(sc: &Scena, i: &Ion, p: &Pos, b: &Box_, vista: Vista, mol: &Mol_) -> String {
    let m = scatola(sc, i, p, vista == Vista::Radiale);
    let radice = p.da.is_none();
    let mut o = format!(
        "<g transform=\"translate({},{})\" role=\"img\" aria-label=\"{}\"><title>{}</title>",
        num(b.x),
        num(b.y),
        esc(&m.aria.join(", ")),
        esc(&tip_nodo(sc, i, p))
    );
    let dash = if i.isolato {
        ""
    } else {
        " stroke-dasharray=\"5 3\""
    };
    o += &format!("<rect width=\"{}\" height=\"{}\" rx=\"6\" fill=\"#ffffff\" stroke=\"{}\" stroke-width=\"1.5\"{dash}/>", num(b.w), num(b.h), if p.comune { INK } else { LINE });
    if p.comune {
        o += &format!("<rect x=\"3\" y=\"3\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"none\" stroke=\"{LINE}\" stroke-width=\"1.5\"/>", num(b.w - 6.0), num(b.h - 6.0));
        o += &format!(
            "<text x=\"{}\" y=\"17\" font-family=\"sans-serif\" font-size=\"11\" font-weight=\"700\" fill=\"{INK}\" text-anchor=\"end\">×{}<title>Reached from {} different precursors</title></text>",
            num(b.w - 36.0), p.entranti, p.entranti
        );
    }
    let rel = m.rel.map_or(String::new(), |r| format!("  ·  {r:.1} %"));
    o += &format!("<text x=\"8\" y=\"17\" font-family=\"sans-serif\" font-size=\"12\" font-weight=\"700\" fill=\"{INK}\">{}</text>", esc(&format!("m/z {:.4}{rel}", i.mz)));
    o += &format!("<text x=\"8\" y=\"31\" font-family=\"sans-serif\" font-size=\"11\" fill=\"{MUTED}\">{}</text>", esc(&livelli_testo(p)));
    if m.amb {
        o += &format!(
            "<path d=\"M{},8 l5,9 l-10,0 z\" fill=\"{WARN}\" stroke=\"{INK}\"><title>Several candidates</title></path>",
            num(b.w - 16.0)
        );
    }
    let mut y = 48.0;
    for r in &m.righe {
        let mut parti = r.parti.clone();
        let mut tail = format!("  {}", r.ppm);
        if !r.prove.is_empty() {
            tail += &format!("  {}", r.prove.join(" "));
        }
        parti.push(Pezzo::N(tail));
        let (fill, weight) = if r.on {
            (INK, " font-weight=\"700\"")
        } else {
            (MUTED, "")
        };
        o += &testo(&format!("x=\"8\" y=\"{}\" font-family=\"sans-serif\" font-size=\"11\" fill=\"{fill}\"{weight}", num(y)), &parti);
        y += LH;
    }
    if !m.piu.is_empty() {
        o += &format!("<text x=\"8\" y=\"{}\" font-family=\"sans-serif\" font-size=\"11\" fill=\"{MUTED}\">{}</text>", num(y), esc(&m.piu));
        y += LH;
    }
    if m.righe.is_empty() && m.piu.is_empty() {
        y += LH;
    }
    if radice {
        match mol {
            Mol_::Disegno(inner) => {
                o += &format!(
                    "<g transform=\"translate({},{})\" aria-hidden=\"true\">{inner}</g>",
                    num((b.w - DW) / 2.0),
                    num(y - 6.0)
                );
            }
            Mol_::Niente => {
                o += &format!("<text x=\"8\" y=\"{}\" font-family=\"sans-serif\" font-style=\"italic\" font-size=\"10\" fill=\"{MUTED}\">no 2D coordinates</text>", num(y));
            }
            Mol_::Nessuna => {}
        }
    }
    o + "</g>"
}

/// What goes in the box of the root next to the numbers.
pub(super) enum Mol_ {
    Disegno(String),
    Niente,
    Nessuna,
}

pub(super) fn molecola_radice(sc: &Scena) -> Mol_ {
    match &sc.molfile {
        Some(m) => molecola(m).map_or(Mol_::Niente, Mol_::Disegno),
        None if sc.smiles => Mol_::Niente,
        None => Mol_::Nessuna,
    }
}
