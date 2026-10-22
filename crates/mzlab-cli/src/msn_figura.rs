//! The static figure (SVG) of a fragmentation graph `msngrafo/1`: tree or radial view, drawn from the key `vista`. Native only, never in a .wasm.
//! It repeats the LABEL RULE at the top of `mzlab/web/msngrafo-vista.js` (the interactive drawer of the page, the Flow and the stand-alone HTML), so a
//! test compares the texts of both. Colours are written in the file (Okabe-Ito, dark ink on white): the SVG lives alone. Used by `msn_html`, `msn_graph`
//! (`--figura`, `figure`) and the step `msn.figure@1`.

use std::collections::{HashMap, HashSet};

use mzlab_core::error::{Error, Result};

#[path = "msn_figura_disegno.rs"]
mod disegno;
#[path = "msn_figura_scena.rs"]
mod scena;
#[cfg(test)]
#[path = "msn_figura_tests.rs"]
mod tests;

use disegno::*;
use scena::*;

/// The two views of the graph.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Vista {
    Albero,
    Radiale,
}

impl Vista {
    /// `albero`/`tree` and `radiale`/`radial`.
    pub fn da_nome(n: &str) -> Option<Vista> {
        match n {
            "albero" | "tree" => Some(Vista::Albero),
            "radiale" | "radial" => Some(Vista::Radiale),
            _ => None,
        }
    }
}

// ---- layout ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Box_ {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

fn tocca(a: &Box_, b: &Box_) -> bool {
    a.x < b.x + b.w + 6.0 && b.x < a.x + a.w + 6.0 && a.y < b.y + b.h + 6.0 && b.y < a.y + a.h + 6.0
}

/// The ions reached from the roots (a node with no arc into it, or of level 1) along the arcs; in the order of the graph.
fn visibili(sc: &Scena) -> Vec<usize> {
    let idx: HashMap<&str, usize> = sc
        .ioni
        .iter()
        .enumerate()
        .map(|(k, i)| (i.id.as_str(), k))
        .collect();
    let valid: Vec<&Arco> = sc
        .archi
        .iter()
        .filter(|a| idx.contains_key(a.da.as_str()) && idx.contains_key(a.a.as_str()))
        .collect();
    let has_in: HashSet<&str> = valid.iter().map(|a| a.a.as_str()).collect();
    let mut on = vec![false; sc.ioni.len()];
    let mut todo = Vec::new();
    for (k, i) in sc.ioni.iter().enumerate() {
        if (!has_in.contains(i.id.as_str()) || i.livello == 1) && sc.pos.contains_key(&i.id) {
            on[k] = true;
            todo.push(k);
        }
    }
    while let Some(k) = todo.pop() {
        for a in valid.iter().filter(|a| a.da == sc.ioni[k].id) {
            let t = idx[a.a.as_str()];
            if !on[t] && sc.pos.contains_key(&a.a) {
                on[t] = true;
                todo.push(t);
            }
        }
    }
    (0..sc.ioni.len()).filter(|&k| on[k]).collect()
}

struct Disposizione {
    caselle: HashMap<String, Box_>,
    w: f64,
    h: f64,
    centro: (f64, f64),
    anelli: Vec<(f64, f64)>,
}

fn altezza(m: &Scatola, extra: f64) -> f64 {
    36.0 + LH * (m.righe.len().max(1) as f64)
        + if m.piu.is_empty() { 0.0 } else { LH }
        + extra
        + 6.0
}

/// The tree view: one column per level, rows of the height of their tallest box.
fn dispone_albero(
    sc: &Scena,
    vis: &[usize],
    size: &HashMap<&str, (f64, f64)>,
) -> HashMap<String, Box_> {
    let mut caselle = HashMap::new();
    let mut rows: Vec<i64> = vis.iter().map(|&k| sc.pos[&sc.ioni[k].id].riga).collect();
    rows.sort_unstable();
    rows.dedup();
    let mut row_h = vec![0.0f64; rows.len()];
    for &k in vis {
        let r = rows
            .iter()
            .position(|&x| x == sc.pos[&sc.ioni[k].id].riga)
            .unwrap_or(0);
        row_h[r] = row_h[r].max(size[sc.ioni[k].id.as_str()].1);
    }
    let mut top = vec![20.0];
    for (k, h) in row_h.iter().enumerate() {
        top.push(top[k] + h + GAP_Y);
    }
    for &k in vis {
        let (i, p) = (&sc.ioni[k], &sc.pos[&sc.ioni[k].id]);
        let r = rows.iter().position(|&x| x == p.riga).unwrap_or(0);
        let (w, h) = size[i.id.as_str()];
        caselle.insert(
            i.id.clone(),
            Box_ {
                x: 20.0 + p.col as f64 * (W + GAP_X),
                y: top[r],
                w,
                h,
            },
        );
    }
    caselle
}

/// The boxes of the ions `here` of ring `r` at radius `rad`, each at its angle (the root alone sits at the centre).
fn su_anello(
    sc: &Scena,
    here: &[usize],
    size: &HashMap<&str, (f64, f64)>,
    r: i64,
    rad: f64,
) -> Vec<(String, Box_)> {
    here.iter()
        .map(|&k| {
            let (i, p) = (&sc.ioni[k], &sc.pos[&sc.ioni[k].id]);
            let a = p.angolo * std::f64::consts::PI / 180.0;
            let (w, h) = size[i.id.as_str()];
            let (cx, cy) = if r == 0 {
                (0.0, 0.0)
            } else {
                (rad * a.sin(), -rad * a.cos())
            };
            (
                i.id.clone(),
                Box_ {
                    x: cx - w / 2.0,
                    y: cy - h / 2.0,
                    w,
                    h,
                },
            )
        })
        .collect()
}

/// The radial view: one ring per step, the rings widened until no box touches another. Returns the boxes, the centre and the ring radii.
fn dispone_radiale(
    sc: &Scena,
    vis: &[usize],
    size: &HashMap<&str, (f64, f64)>,
) -> (HashMap<String, Box_>, (f64, f64), Vec<(f64, f64)>) {
    let mut caselle = HashMap::new();
    let mut anelli = Vec::new();
    let max_ring = vis
        .iter()
        .map(|&k| sc.pos[&sc.ioni[k].id].anello)
        .max()
        .unwrap_or(0);
    let mut at: HashMap<String, Box_> = HashMap::new();
    let mut placed: Vec<String> = Vec::new();
    let mut radius = 0.0f64;
    for r in 0..=max_ring {
        let here: Vec<usize> = vis
            .iter()
            .copied()
            .filter(|&k| sc.pos[&sc.ioni[k].id].anello == r)
            .collect();
        if here.is_empty() {
            continue;
        }
        let put = |rad: f64| su_anello(sc, &here, size, r, rad);
        let clash = |list: &[(String, Box_)]| {
            list.iter().enumerate().any(|(a, p)| {
                list[a + 1..].iter().any(|q| tocca(&p.1, &q.1))
                    || placed.iter().any(|id| tocca(&p.1, &at[id]))
            })
        };
        radius = if r == 0 {
            0.0
        } else {
            (radius + 40.0).max(120.0)
        };
        let mut list = put(radius);
        let mut k = 0;
        while clash(&list) && k < 4000 {
            radius += 10.0;
            list = put(radius);
            k += 1;
        }
        if r > 0 {
            let b = list[0].1;
            anelli.push((r as f64, (b.x + b.w / 2.0).hypot(b.y + b.h / 2.0).max(0.0)));
        }
        for (id, b) in list {
            at.insert(id.clone(), b);
            placed.push(id);
        }
    }
    let x0 = at.values().map(|b| b.x).fold(f64::INFINITY, f64::min) - 20.0;
    let y0 = at.values().map(|b| b.y).fold(f64::INFINITY, f64::min) - 20.0;
    for (id, b) in at {
        caselle.insert(
            id,
            Box_ {
                x: b.x - x0,
                y: b.y - y0,
                ..b
            },
        );
    }
    let centro = (-x0, -y0);
    // the rings were measured from the middle of the figure before the shift: the radius does not change with it
    (caselle, centro, anelli)
}

fn dispone(
    sc: &Scena,
    vis: &[usize],
    vista: Vista,
    extra: &dyn Fn(&Ion, &Pos) -> f64,
) -> Disposizione {
    let compatta = vista == Vista::Radiale;
    let size: HashMap<&str, (f64, f64)> = vis
        .iter()
        .map(|&k| {
            let (i, p) = (&sc.ioni[k], &sc.pos[&sc.ioni[k].id]);
            (
                i.id.as_str(),
                (
                    if compatta { WR } else { W },
                    altezza(&scatola(sc, i, p, compatta), extra(i, p)),
                ),
            )
        })
        .collect();
    let (caselle, centro, anelli) = if vista == Vista::Albero {
        (dispone_albero(sc, vis, &size), (0.0, 0.0), Vec::new())
    } else {
        dispone_radiale(sc, vis, &size)
    };
    let (mut w, mut h) = (0.0f64, 0.0f64);
    for b in caselle.values() {
        w = w.max(b.x + b.w + 30.0);
        h = h.max(b.y + b.h + 30.0);
    }
    Disposizione {
        caselle,
        w: w.max(60.0),
        h: h.max(60.0),
        centro,
        anelli,
    }
}

/// The molecule of the root as a small stand-alone SVG (150 × 80), or None when the graph has no molfile with 2D coordinates.
pub fn molecola_svg(graph: &str) -> Result<Option<String>> {
    let sc = scena(graph)?;
    Ok(sc.molfile.as_deref().and_then(molecola).map(|i| {
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {DW} {DH}\">{i}</svg>")
    }))
}

/// The figure of the graph `graph` (an `msngrafo/1` text; the key `vista` is added when missing) as a stand-alone SVG.
pub fn svg(graph: &str, vista: Vista) -> Result<String> {
    let sc = scena(graph)?;
    if sc.ioni.is_empty() {
        return Err(Error::new("err.msnfigura.empty"));
    }
    let vis_ix = visibili(&sc);
    let mol = molecola_radice(&sc);
    let extra = |_: &Ion, p: &Pos| match (&mol, p.da.is_none()) {
        (Mol_::Disegno(_), true) => DH + 18.0,
        (Mol_::Niente, true) => LH,
        _ => 0.0,
    };
    let d = dispone(&sc, &vis_ix, vista, &extra);
    let vis: HashSet<&str> = vis_ix.iter().map(|&k| sc.ioni[k].id.as_str()).collect();
    let narcs = sc
        .archi
        .iter()
        .filter(|a| vis.contains(a.da.as_str()) && vis.contains(a.a.as_str()))
        .count();
    let (modo, nome) = if vista == Vista::Radiale {
        ("radial", "Fragmentation graph, radial view")
    } else {
        ("tree", "Fragmentation graph, tree view")
    };
    let title = format!("{nome}: {} ions, {narcs} losses", vis.len());
    let desc = "Boxes are ions (m/z, relative intensity, MS levels, candidate formulas as proposals, not identifications); arrows are neutral losses. Solid blue arrows are the best path from the precursor, dashed grey arrows the other observed losses. A double border marks an ion reached from several precursors.";
    let mut o = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" role=\"img\" aria-labelledby=\"t d\" data-mode=\"{modo}\"><title id=\"t\">{}</title><desc id=\"d\">{}</desc>",
        esc(&title),
        esc(desc),
        w = num(d.w),
        h = num(d.h)
    );
    o += &format!("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/><defs><marker id=\"h-b\" viewBox=\"0 0 8 8\" refX=\"8\" refY=\"4\" markerUnits=\"userSpaceOnUse\" markerWidth=\"9\" markerHeight=\"9\" orient=\"auto\"><path d=\"M0,0 L8,4 L0,8 z\" fill=\"{BEST}\"/></marker><marker id=\"h-a\" viewBox=\"0 0 8 8\" refX=\"8\" refY=\"4\" markerUnits=\"userSpaceOnUse\" markerWidth=\"9\" markerHeight=\"9\" orient=\"auto\"><path d=\"M0,0 L8,4 L0,8 z\" fill=\"{LINE}\"/></marker></defs>");
    if vista == Vista::Radiale {
        for (_, rad) in &d.anelli {
            o += &format!("<circle cx=\"{}\" cy=\"{}\" r=\"{rad:.1}\" fill=\"none\" stroke=\"#b3b3b3\" stroke-dasharray=\"2 4\"/>", num(d.centro.0), num(d.centro.1));
        }
    }
    o += &archi_svg(&sc, &d, vista, &vis);
    for &k in &vis_ix {
        let (i, p) = (&sc.ioni[k], &sc.pos[&sc.ioni[k].id]);
        o += &nodo_svg(&sc, i, p, &d.caselle[&i.id], vista, &mol);
    }
    Ok(o + "</svg>\n")
}

/// The texts of the figure, in drawing order, for the parity test with `msngrafo-vista.js`: one line per ion (`node`, id, m/z line, levels line, formula lines) and
/// per loss (`arc`, from, to, first line, second line, rule).
pub fn testi(graph: &str, vista: Vista) -> Result<Vec<String>> {
    let sc = scena(graph)?;
    let vis_ix = visibili(&sc);
    let vis: HashSet<&str> = vis_ix.iter().map(|&k| sc.ioni[k].id.as_str()).collect();
    let mut out = Vec::new();
    for &k in &vis_ix {
        let (i, p) = (&sc.ioni[k], &sc.pos[&sc.ioni[k].id]);
        let m = scatola(&sc, i, p, vista == Vista::Radiale);
        let rel = m.rel.map_or(String::new(), |r| format!("  ·  {r:.1} %"));
        let mut l = vec![
            format!("node {}", i.id),
            format!("m/z {:.4}{rel}", i.mz),
            livelli_testo(p),
        ];
        for r in &m.righe {
            let tail = if r.prove.is_empty() {
                String::new()
            } else {
                format!("  {}", r.prove.join(" "))
            };
            l.push(format!("{}  {}{tail}", piano(&r.parti), r.ppm));
        }
        if !m.piu.is_empty() {
            l.push(m.piu.clone());
        }
        out.push(l.join(" | "));
    }
    for a in sc
        .archi
        .iter()
        .filter(|a| vis.contains(a.da.as_str()) && vis.contains(a.a.as_str()))
    {
        let best = sc.best.contains(&format!("{}>{}", a.da, a.a));
        if vista == Vista::Radiale && !best {
            continue;
        }
        let (first, second) = perdita_testo(a);
        out.push(format!(
            "arc {}>{} | {} | {} | {}",
            a.da,
            a.a,
            piano(&first),
            second.as_deref().map_or(String::new(), piano),
            a.regola
        ));
    }
    Ok(out)
}
