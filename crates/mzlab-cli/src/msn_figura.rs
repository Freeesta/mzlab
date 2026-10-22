//! The static figure (SVG) of a fragmentation graph `msngrafo/1`: tree or radial view, drawn from the key `vista`. Native only, never in a .wasm.
//! It repeats the LABEL RULE at the top of `mzlab/web/msngrafo-vista.js` (the interactive drawer of the page, the Flow and the stand-alone HTML), so a
//! test compares the texts of both. Colours are written in the file (Okabe-Ito, dark ink on white): the SVG lives alone. Used by `msn_html`, `msn_graph`
//! (`--figura`, `figure`) and the step `msn.figure@1`.

use std::collections::{HashMap, HashSet};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

const W: f64 = 176.0;
const WR: f64 = 150.0;
const LH: f64 = 14.0;
const GAP_X: f64 = 130.0;
const GAP_Y: f64 = 20.0;
const MAXC: usize = 4;
const DW: f64 = 150.0;
const DH: f64 = 80.0;

const INK: &str = "#1a1a1a";
const MUTED: &str = "#4d4d4d";
const LINE: &str = "#595959";
const BEST: &str = "#0072B2";
const WARN: &str = "#E69F00";

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

// ---- model -----------------------------------------------------------------------------------------------------------

struct Cand {
    formula: String,
    ppm: f64,
    prove: Vec<String>,
}

struct Ion {
    id: String,
    livello: i64,
    mz: f64,
    isolato: bool,
    cands: Vec<Cand>,
    scelta: Option<String>,
    has_formula: bool,
}

struct Pos {
    livelli: Vec<i64>,
    rel: Option<f64>,
    da: Option<String>,
    entranti: i64,
    comune: bool,
    col: i64,
    riga: i64,
    anello: i64,
    angolo: f64,
}

struct Misura {
    livello: i64,
    att: Option<String>,
    ce: Option<f64>,
    rel: f64,
}

struct Arco {
    da: String,
    a: String,
    misure: Vec<Misura>,
    formula: Option<String>,
    da_massa: f64,
    compat: Vec<String>,
    regola: String,
}

/// Everything the drawing needs, read once from the graph.
struct Scena {
    ioni: Vec<Ion>,
    pos: HashMap<String, Pos>,
    archi: Vec<Arco>,
    best: HashSet<String>,
    carica: String,
    molfile: Option<String>,
    smiles: bool,
}

fn bad() -> Error {
    Error::new("err.msnfigura.bad")
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.str()).map(str::to_string)
}

fn n(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.f64())
}

/// The rule ids shown under a loss: at most two (the weak ones tagged), `+n` for the rest; empty when no rule explains it.
fn regola_di(j: &Value, da: &str, a: &str) -> String {
    let Some(r) = j
        .get("regole")
        .and_then(|r| r.get("archi"))
        .and_then(|l| {
            l.arr()
                .iter()
                .find(|x| s(x, "da").as_deref() == Some(da) && s(x, "a").as_deref() == Some(a))
        })
    else {
        return String::new();
    };
    let reg = r.get("regole").map_or(&[][..], |x| x.arr());
    let comp = r.get("composta").map_or(&[][..], |x| x.arr());
    let ids: Vec<String> = if reg.is_empty() {
        comp.iter()
            .map(|c| {
                c.arr()
                    .iter()
                    .filter_map(|x| x.str())
                    .collect::<Vec<_>>()
                    .join(" + ")
            })
            .collect()
    } else {
        reg.iter()
            .map(|x| {
                let weak = if n(x, "priorita") == Some(3.0) { " (weak)" } else { "" };
                format!("{}{weak}", s(x, "id").unwrap_or_default())
            })
            .collect()
    };
    if ids.is_empty() {
        return String::new();
    }
    let more = if ids.len() > 2 { format!(" +{}", ids.len() - 2) } else { String::new() };
    format!("{}{more}", ids.iter().take(2).cloned().collect::<Vec<_>>().join(", "))
}

fn scena(graph: &str) -> Result<Scena> {
    let done = mzlab_core::msngrafo::completa(graph).map_err(|_| bad())?;
    let j = parse(done.as_bytes()).map_err(|_| bad())?;
    let vista = j.get("vista").ok_or_else(bad)?;
    let mut ioni = Vec::new();
    for x in j.get("nodi").map_or(&[][..], |a| a.arr()) {
        let f = x.get("formula").filter(|f| !f.is_null());
        let cands = f
            .and_then(|f| f.get("candidati"))
            .map_or(&[][..], |c| c.arr())
            .iter()
            .map(|c| Cand {
                formula: s(c, "formula").unwrap_or_default(),
                ppm: n(c, "ppm").unwrap_or(0.0),
                prove: c.get("prove").map_or(&[][..], |p| p.arr()).iter().filter_map(|p| p.str().map(str::to_string)).collect(),
            })
            .collect();
        ioni.push(Ion {
            id: s(x, "id").ok_or_else(bad)?,
            livello: n(x, "livello").unwrap_or(0.0) as i64,
            mz: n(x, "mz").ok_or_else(bad)?,
            isolato: x.get("isolato").and_then(|b| b.bool()).unwrap_or(false),
            cands,
            scelta: f.and_then(|f| s(f, "scelta")),
            has_formula: f.is_some(),
        });
    }
    let mut pos = HashMap::new();
    for x in vista.get("nodi").map_or(&[][..], |a| a.arr()) {
        let best = x.get("best");
        let albero = x.get("albero");
        let radiale = x.get("radiale").filter(|r| !r.is_null());
        pos.insert(
            s(x, "id").ok_or_else(bad)?,
            Pos {
                livelli: x.get("livelli").map_or(&[][..], |l| l.arr()).iter().filter_map(|l| l.f64()).map(|l| l as i64).collect(),
                rel: n(x, "rel"),
                da: best.and_then(|b| s(b, "da")),
                entranti: n(x, "entranti").unwrap_or(0.0) as i64,
                comune: x.get("comune").and_then(|b| b.bool()).unwrap_or(false),
                col: albero.and_then(|a| n(a, "col")).unwrap_or(0.0) as i64,
                riga: albero.and_then(|a| n(a, "riga")).unwrap_or(0.0) as i64,
                anello: radiale.and_then(|a| n(a, "anello")).unwrap_or(0.0) as i64,
                angolo: radiale.and_then(|a| n(a, "angolo")).unwrap_or(0.0),
            },
        );
    }
    let mut archi = Vec::new();
    for a in j.get("archi").map_or(&[][..], |a| a.arr()) {
        let (da, to) = (s(a, "da").ok_or_else(bad)?, s(a, "a").ok_or_else(bad)?);
        let p = a.get("perdita");
        archi.push(Arco {
            misure: a
                .get("misure")
                .map_or(&[][..], |m| m.arr())
                .iter()
                .map(|m| Misura { livello: n(m, "livello").unwrap_or(0.0) as i64, att: s(m, "att"), ce: n(m, "ce"), rel: n(m, "rel").unwrap_or(0.0) })
                .collect(),
            formula: p.and_then(|p| s(p, "formula")),
            da_massa: p.and_then(|p| n(p, "da_massa")).unwrap_or(0.0),
            compat: p.and_then(|p| p.get("compatibile")).map_or(&[][..], |c| c.arr()).iter().filter_map(|c| c.str().map(str::to_string)).collect(),
            regola: regola_di(&j, &da, &to),
            da,
            a: to,
        });
    }
    let best = vista
        .get("archi")
        .map_or(&[][..], |a| a.arr())
        .iter()
        .filter(|a| a.get("best").and_then(|b| b.bool()) == Some(true))
        .map(|a| format!("{}>{}", s(a, "da").unwrap_or_default(), s(a, "a").unwrap_or_default()))
        .collect();
    let o = j.get("opzioni");
    Ok(Scena {
        ioni,
        pos,
        archi,
        best,
        carica: if n(&j, "polarita") == Some(-1.0) { "-" } else { "+" }.into(),
        molfile: o.and_then(|o| s(o, "molfile")).filter(|m| !m.trim().is_empty()),
        smiles: o.and_then(|o| s(o, "smiles")).is_some_and(|m| !m.is_empty()),
    })
}

// ---- labels (the rule of msngrafo-vista.js) ------------------------------------------------------------------------------

/// A piece of a text: plain, subscript or superscript.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Pezzo {
    N(String),
    Sub(String),
    Sup(String),
}

/// The formula split into its plain text and its charge: «C13H25N4O3S+» → («C13H25N4O3S», «+»).
pub(crate) fn dividi_carica(f: &str, dflt: &str) -> (String, String) {
    let mut c = f.chars();
    match c.next_back() {
        Some(l @ ('+' | '-' | '−')) => (c.as_str().to_string(), if l == '+' { "+" } else { "-" }.into()),
        _ => (f.to_string(), dflt.to_string()),
    }
}

/// The formula as pieces to draw: digits are subscripts, the charge a superscript.
pub(crate) fn pezzi_formula(f: &str, carica: &str) -> Vec<Pezzo> {
    let mut out: Vec<Pezzo> = Vec::new();
    let mut cifre = String::new();
    let mut testo = String::new();
    for ch in f.chars() {
        if ch.is_ascii_digit() {
            if !testo.is_empty() {
                out.push(Pezzo::N(std::mem::take(&mut testo)));
            }
            cifre.push(ch);
        } else {
            if !cifre.is_empty() {
                out.push(Pezzo::Sub(std::mem::take(&mut cifre)));
            }
            testo.push(ch);
        }
    }
    if !testo.is_empty() {
        out.push(Pezzo::N(testo));
    }
    if !cifre.is_empty() {
        out.push(Pezzo::Sub(cifre));
    }
    if !carica.is_empty() {
        out.push(Pezzo::Sup(if carica == "-" { "−" } else { carica }.into()));
    }
    out
}

/// The plain form of the pieces (for the accessible names and the tooltips).
fn piano(p: &[Pezzo]) -> String {
    p.iter()
        .map(|x| match x {
            Pezzo::N(t) | Pezzo::Sub(t) | Pezzo::Sup(t) => t.as_str(),
        })
        .collect()
}

/// The digits of a formula as Unicode subscripts and the charge as a superscript, for the texts that take no markup.
fn bella(f: &str) -> String {
    const SUB: [char; 10] = ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];
    f.chars().map(|c| c.to_digit(10).map_or(c, |d| SUB[d as usize])).collect()
}

fn segno(v: f64) -> String {
    let sg = if v > 0.0 { "+" } else if v < 0.0 { "−" } else { "" };
    format!("{sg}{:.2}", v.abs())
}

/// A number as JavaScript prints it: whole numbers without decimals.
fn js_num(v: f64) -> String {
    if v.fract() == 0.0 { format!("{}", v as i64) } else { format!("{v}") }
}

fn livelli_testo(p: &Pos) -> String {
    p.livelli.iter().map(|l| format!("MS{l}")).collect::<Vec<_>>().join(", ")
}

fn prova_testo(p: &str) -> String {
    match p {
        "perdita" => "known loss",
        "isotopi" => "isotopes",
        "msn" => "MSn",
        x => x,
    }
    .to_string()
}

struct Riga {
    parti: Vec<Pezzo>,
    ppm: String,
    on: bool,
    prove: Vec<String>,
}

struct Scatola {
    righe: Vec<Riga>,
    piu: String,
    rel: Option<f64>,
    aria: Vec<String>,
    amb: bool,
}

fn scelta_di(i: &Ion) -> Option<String> {
    i.scelta.clone().or_else(|| if i.cands.len() == 1 { Some(i.cands[0].formula.clone()) } else { None })
}

fn scatola(sc: &Scena, i: &Ion, p: &Pos, compatta: bool) -> Scatola {
    let chosen = scelta_di(i);
    let mut cs: Vec<&Cand> = i.cands.iter().collect();
    cs.sort_by_key(|c| u8::from(Some(&c.formula) != chosen.as_ref()));
    let keep = if compatta { 1 } else { MAXC };
    let righe = cs
        .iter()
        .take(keep)
        .map(|c| {
            let (b, ch) = dividi_carica(&c.formula, &sc.carica);
            Riga { parti: pezzi_formula(&b, &ch), ppm: format!("{} ppm", segno(c.ppm)), on: Some(&c.formula) == chosen.as_ref(), prove: c.prove.iter().map(|x| prova_testo(x)).collect() }
        })
        .collect();
    let piu = if cs.len() > keep { format!("+{} more", cs.len() - keep) } else { String::new() };
    let rel = if p.da.is_some() { p.rel } else { None };
    let mut aria = vec![format!("m/z {:.4}", i.mz), format!("levels {}", livelli_testo(p))];
    if let Some(r) = rel {
        aria.push(format!("{r:.1} % of the base peak"));
    }
    if let Some(c) = &chosen {
        let (b, ch) = dividi_carica(c, &sc.carica);
        aria.push(format!("formula {b}{ch}"));
    } else if !cs.is_empty() {
        aria.push(format!("{} candidate formulas", cs.len()));
    }
    if p.comune {
        aria.push(format!("reached from {} precursors", p.entranti));
    }
    Scatola { righe, piu, rel, aria, amb: i.has_formula && chosen.is_none() }
}

fn tip_nodo(sc: &Scena, i: &Ion, p: &Pos) -> String {
    let mut out = vec![format!("m/z {:.5} · {}", i.mz, livelli_testo(p))];
    for a in sc.archi.iter().filter(|a| a.a == i.id) {
        for m in &a.misure {
            let att = m.att.as_ref().map_or(String::new(), |x| format!(" {}", x.to_uppercase()));
            let ce = m.ce.map_or(String::new(), |c| format!(" {}", js_num(c)));
            out.push(format!("MS{}{att}{ce}: {:.1} %", m.livello, m.rel));
        }
    }
    out.join("\n")
}

/// The label of a loss: the pieces of the first line and, if any, of the second («(compatible with H2O)»).
fn perdita_testo(a: &Arco) -> (Vec<Pezzo>, Option<Vec<Pezzo>>) {
    if let Some(f) = &a.formula {
        let mut v = vec![Pezzo::N("−".into())];
        v.extend(pezzi_formula(f, ""));
        return (v, None);
    }
    let comp: Vec<&String> = a.compat.iter().take(2).collect();
    let mut second = None;
    if !comp.is_empty() {
        let mut v = vec![Pezzo::N("(compatible with ".into())];
        for (k, c) in comp.iter().enumerate() {
            if k > 0 {
                v.push(Pezzo::N(" / ".into()));
            }
            v.extend(pezzi_formula(c, ""));
        }
        v.push(Pezzo::N(")".into()));
        second = Some(v);
    }
    (vec![Pezzo::N(format!("Δ {:.3} Da", a.da_massa))], second)
}

fn tip_perdita(a: &Arco) -> String {
    if let Some(f) = &a.formula {
        return format!("−{}", bella(f));
    }
    let comp: Vec<String> = a.compat.iter().take(2).map(|c| bella(c)).collect();
    let extra = if comp.is_empty() { String::new() } else { format!(" (compatible with {})", comp.join(" / ")) };
    format!("Δ {:.3} Da{extra}", a.da_massa)
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
    let idx: HashMap<&str, usize> = sc.ioni.iter().enumerate().map(|(k, i)| (i.id.as_str(), k)).collect();
    let valid: Vec<&Arco> = sc.archi.iter().filter(|a| idx.contains_key(a.da.as_str()) && idx.contains_key(a.a.as_str())).collect();
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
    36.0 + LH * (m.righe.len().max(1) as f64) + if m.piu.is_empty() { 0.0 } else { LH } + extra + 6.0
}

fn dispone(sc: &Scena, vis: &[usize], vista: Vista, extra: &dyn Fn(&Ion, &Pos) -> f64) -> Disposizione {
    let compatta = vista == Vista::Radiale;
    let size: HashMap<&str, (f64, f64)> = vis
        .iter()
        .map(|&k| {
            let (i, p) = (&sc.ioni[k], &sc.pos[&sc.ioni[k].id]);
            (i.id.as_str(), (if compatta { WR } else { W }, altezza(&scatola(sc, i, p, compatta), extra(i, p))))
        })
        .collect();
    let mut caselle = HashMap::new();
    let mut centro = (0.0, 0.0);
    let mut anelli = Vec::new();
    if vista == Vista::Albero {
        let mut rows: Vec<i64> = vis.iter().map(|&k| sc.pos[&sc.ioni[k].id].riga).collect();
        rows.sort_unstable();
        rows.dedup();
        let mut row_h = vec![0.0f64; rows.len()];
        for &k in vis {
            let r = rows.iter().position(|&x| x == sc.pos[&sc.ioni[k].id].riga).unwrap_or(0);
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
            caselle.insert(i.id.clone(), Box_ { x: 20.0 + p.col as f64 * (W + GAP_X), y: top[r], w, h });
        }
    } else {
        let max_ring = vis.iter().map(|&k| sc.pos[&sc.ioni[k].id].anello).max().unwrap_or(0);
        let mut at: HashMap<String, Box_> = HashMap::new();
        let mut placed: Vec<String> = Vec::new();
        let mut radius = 0.0f64;
        for r in 0..=max_ring {
            let here: Vec<usize> = vis.iter().copied().filter(|&k| sc.pos[&sc.ioni[k].id].anello == r).collect();
            if here.is_empty() {
                continue;
            }
            let put = |rad: f64| -> Vec<(String, Box_)> {
                here.iter()
                    .map(|&k| {
                        let (i, p) = (&sc.ioni[k], &sc.pos[&sc.ioni[k].id]);
                        let a = p.angolo * std::f64::consts::PI / 180.0;
                        let (w, h) = size[i.id.as_str()];
                        let (cx, cy) = if r == 0 { (0.0, 0.0) } else { (rad * a.sin(), -rad * a.cos()) };
                        (i.id.clone(), Box_ { x: cx - w / 2.0, y: cy - h / 2.0, w, h })
                    })
                    .collect()
            };
            let clash = |list: &[(String, Box_)]| {
                list.iter().enumerate().any(|(a, p)| list[a + 1..].iter().any(|q| tocca(&p.1, &q.1)) || placed.iter().any(|id| tocca(&p.1, &at[id])))
            };
            radius = if r == 0 { 0.0 } else { (radius + 40.0).max(120.0) };
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
            caselle.insert(id, Box_ { x: b.x - x0, y: b.y - y0, ..b });
        }
        centro = (-x0, -y0);
        // the rings were measured from the middle of the figure before the shift: the radius does not change with it
    }
    let (mut w, mut h) = (0.0f64, 0.0f64);
    for b in caselle.values() {
        w = w.max(b.x + b.w + 30.0);
        h = h.max(b.y + b.h + 30.0);
    }
    Disposizione { caselle, w: w.max(60.0), h: h.max(60.0), centro, anelli }
}

// ---- molecule ---------------------------------------------------------------------------------------------------------

/// The molecule of a molfile as lines and symbols from its 2D coordinates (inner SVG elements in a box of DW × DH), or None when the file has no 2D coordinates.
fn molecola(molfile: &str) -> Option<String> {
    let m = mzlab_mol::Mol::from_molfile(molfile).ok()?;
    let xy = &m.coords.as_ref()?.xy;
    if xy.len() != m.atoms.len() || xy.iter().any(|p| p.len() < 2) {
        return None;
    }
    let (x0, x1) = xy.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| (a.0.min(p[0]), a.1.max(p[0])));
    let (y0, y1) = xy.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| (a.0.min(p[1]), a.1.max(p[1])));
    if x1 - x0 < 1e-6 && y1 - y0 < 1e-6 {
        return None;
    }
    let pad = 10.0;
    let k = ((DW - 2.0 * pad) / (x1 - x0).max(1e-6)).min((DH - 2.0 * pad) / (y1 - y0).max(1e-6)).min(30.0);
    let at = |p: &Vec<f64>| ((DW - k * (x1 - x0)) / 2.0 + k * (p[0] - x0), (DH - k * (y1 - y0)) / 2.0 + k * (y1 - p[1]));
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
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn num(v: f64) -> String {
    let t = format!("{v:.2}");
    t.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A `<text>` with its pieces: subscripts and the charge in tspans.
fn testo(at: &str, parti: &[Pezzo]) -> String {
    let mut o = format!("<text {at}>");
    for p in parti {
        match p {
            Pezzo::N(t) => o += &esc(t),
            Pezzo::Sub(t) => o += &format!("<tspan baseline-shift=\"sub\" font-size=\"75%\">{}</tspan>", esc(t)),
            Pezzo::Sup(t) => o += &format!("<tspan baseline-shift=\"super\" font-size=\"75%\">{}</tspan>", esc(t)),
        }
    }
    o + "</text>"
}

fn bordo(b: &Box_, tx: f64, ty: f64) -> (f64, f64) {
    let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
    let (dx, dy) = (tx - cx, ty - cy);
    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }
    let k = (if dx != 0.0 { b.w / 2.0 / dx.abs() } else { f64::INFINITY }).min(if dy != 0.0 { b.h / 2.0 / dy.abs() } else { f64::INFINITY });
    (cx + dx * k, cy + dy * k)
}

fn archi_svg(sc: &Scena, d: &Disposizione, vista: Vista, vis: &HashSet<&str>) -> String {
    let mut arcs: Vec<&Arco> = sc.archi.iter().filter(|a| vis.contains(a.da.as_str()) && vis.contains(a.a.as_str())).collect();
    arcs.sort_by_key(|a| sc.best.contains(&format!("{}>{}", a.da, a.a)));
    let mut o = String::new();
    for a in arcs {
        let (p, q) = (d.caselle[&a.da], d.caselle[&a.a]);
        let best = sc.best.contains(&format!("{}>{}", a.da, a.a));
        let rel = a.misure.iter().map(|m| m.rel).fold(0.0f64, f64::max);
        let (stroke, width, dash) = if best { (BEST, format!("{:.2}", 1.0 + (1.0 + rel).log10() * 1.2), "") } else { (LINE, "1".into(), " stroke-dasharray=\"4 3\"") };
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
            let (s0, e0) = (bordo(&p, q.x + q.w / 2.0, q.y + q.h / 2.0), bordo(&q, p.x + p.w / 2.0, p.y + p.h / 2.0));
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
        o += &testo(&format!("x=\"{}\" y=\"{}\" {halo}", num(lx), num(ly - 3.0)), &first);
        let mut dy = 9.0;
        if let Some(sec) = second {
            o += &testo(&format!("x=\"{}\" y=\"{}\" {halo}", num(lx), num(ly + dy)), &sec);
            dy += 11.0;
        }
        if !a.regola.is_empty() {
            o += &format!("<text x=\"{}\" y=\"{}\" {halo}>{}</text>", num(lx), num(ly + dy), esc(&a.regola));
        }
        o += "</g>";
    }
    o
}

fn nodo_svg(sc: &Scena, i: &Ion, p: &Pos, b: &Box_, vista: Vista, mol: &Mol_) -> String {
    let m = scatola(sc, i, p, vista == Vista::Radiale);
    let radice = p.da.is_none();
    let mut o = format!(
        "<g transform=\"translate({},{})\" role=\"img\" aria-label=\"{}\"><title>{}</title>",
        num(b.x),
        num(b.y),
        esc(&m.aria.join(", ")),
        esc(&tip_nodo(sc, i, p))
    );
    let dash = if i.isolato { "" } else { " stroke-dasharray=\"5 3\"" };
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
        let (fill, weight) = if r.on { (INK, " font-weight=\"700\"") } else { (MUTED, "") };
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
                o += &format!("<g transform=\"translate({},{})\" aria-hidden=\"true\">{inner}</g>", num((b.w - DW) / 2.0), num(y - 6.0));
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
enum Mol_ {
    Disegno(String),
    Niente,
    Nessuna,
}

fn molecola_radice(sc: &Scena) -> Mol_ {
    match &sc.molfile {
        Some(m) => molecola(m).map_or(Mol_::Niente, Mol_::Disegno),
        None if sc.smiles => Mol_::Niente,
        None => Mol_::Nessuna,
    }
}

/// The molecule of the root as a small stand-alone SVG (150 × 80), or None when the graph has no molfile with 2D coordinates.
pub fn molecola_svg(graph: &str) -> Result<Option<String>> {
    let sc = scena(graph)?;
    Ok(sc.molfile.as_deref().and_then(molecola).map(|i| format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {DW} {DH}\">{i}</svg>")))
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
    let narcs = sc.archi.iter().filter(|a| vis.contains(a.da.as_str()) && vis.contains(a.a.as_str())).count();
    let (modo, nome) = if vista == Vista::Radiale { ("radial", "Fragmentation graph, radial view") } else { ("tree", "Fragmentation graph, tree view") };
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
        let mut l = vec![format!("node {}", i.id), format!("m/z {:.4}{rel}", i.mz), livelli_testo(p)];
        for r in &m.righe {
            let tail = if r.prove.is_empty() { String::new() } else { format!("  {}", r.prove.join(" ")) };
            l.push(format!("{}  {}{tail}", piano(&r.parti), r.ppm));
        }
        if !m.piu.is_empty() {
            l.push(m.piu.clone());
        }
        out.push(l.join(" | "));
    }
    for a in sc.archi.iter().filter(|a| vis.contains(a.da.as_str()) && vis.contains(a.a.as_str())) {
        let best = sc.best.contains(&format!("{}>{}", a.da, a.a));
        if vista == Vista::Radiale && !best {
            continue;
        }
        let (first, second) = perdita_testo(a);
        out.push(format!("arc {}>{} | {} | {} | {}", a.da, a.a, piano(&first), second.as_deref().map_or(String::new(), piano), a.regola));
    }
    Ok(out)
}
