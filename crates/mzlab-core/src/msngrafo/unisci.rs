//! The merging of `msngrafo`: every spectrum of the tree gives observations (its peaks above `min_rel`, the exact precursor of each node read
//! in the spectrum of its parent), the observations closer than the tolerance become one node, and each peak an arc from the precursor
//! of its spectrum.

use std::collections::BTreeMap;

use super::{
    Counts, Edge, GNode, Graph, Input, Measure, Options, Spettro, SpettroG, Warning, PERDITE,
};

/// 13C - 12C.
const C13: f64 = crate::elements::C13_STEP;
/// The isolated precursor is the most intense peak of the parent within this many Da of the m/z of the filter (as `msntree.rs`).
const WINDOW: f64 = 0.5;
/// A second peak in the isolation window above this fraction of the first one: the window took both.
const SECONDO: f64 = 0.3;

/// One peak of one spectrum, or the precursor of a node of the tree (`prec_di`).
struct Oss {
    mz: f64,
    rel: f64,
    /// The node of the tree whose spectrum it is in; `None` = the MS1.
    sorg: Option<usize>,
    level: u8,
    prec_di: Option<usize>,
    /// Index (in the observations) of the peak whose M+1 it is.
    iso_di: Option<usize>,
    /// Its M+1 in the same spectrum, % of it (any intensity, not only above `min_rel`).
    m1: Option<f64>,
}

/// The peaks of a spectrum as (m/z, % of the base peak), in m/z order.
fn relativi(s: &Spettro) -> Vec<(f64, f64)> {
    let mx = s.rel.iter().copied().fold(0.0, f64::max);
    if mx <= 0.0 {
        return vec![];
    }
    let mut v: Vec<(f64, f64)> =
        s.mz.iter()
            .zip(&s.rel)
            .filter(|(_, r)| **r > 0.0)
            .map(|(m, r)| (*m, r / mx * 100.0))
            .collect();
    crate::sortc::by(&mut v, |a, b| a.0.total_cmp(&b.0));
    v
}

/// The isolated precursor in the parent's spectrum: (m/z, % of the base peak) and whether a second peak shares the window.
fn precursore(s: Option<&Spettro>, nominal: f64) -> (Option<(f64, f64)>, bool) {
    let Some(s) = s else { return (None, false) };
    let mut w: Vec<(f64, f64)> = relativi(s)
        .into_iter()
        .filter(|(m, _)| (m - nominal).abs() <= WINDOW)
        .collect();
    crate::sortc::by(&mut w, |a, b| b.1.total_cmp(&a.1).then(a.0.total_cmp(&b.0)));
    let amb = w.len() > 1 && w[1].1 >= SECONDO * w[0].1;
    (w.first().copied(), amb)
}

/// The peaks of the spectrum of a node of the tree that enter the graph: the most intense above `min_rel`, not heavier than the precursor,
/// not the precursor itself; with the index (in the result) of the peak whose M+1 it is, and its own M+1 in % of it.
fn peaks(
    s: &Spettro,
    prec: f64,
    o: &Options,
    c: &mut Counts,
) -> Vec<(f64, f64, Option<usize>, Option<f64>)> {
    let all = relativi(s);
    let mut forti: Vec<(f64, f64)> = all.iter().copied().filter(|p| p.1 >= o.min_rel).collect();
    crate::sortc::by(&mut forti, |a, b| {
        b.1.total_cmp(&a.1).then(a.0.total_cmp(&b.0))
    });
    forti.truncate(o.max_picchi);
    c.deboli += all.len() - forti.len();
    crate::sortc::by(&mut forti, |a, b| a.0.total_cmp(&b.0));
    let tp = o.tol(prec);
    let mut out: Vec<(f64, f64, Option<usize>, Option<f64>)> = vec![];
    for (m, r) in forti {
        if m > prec + tp {
            c.pesanti += 1;
            continue;
        }
        if (m - prec).abs() <= tp {
            continue;
        }
        // the M+1 of a lighter peak, no more intense than a molecule of m/12 carbons allows
        let mono = out.iter().position(|q| {
            (m - q.0 - C13).abs() <= o.tol(m) && r <= q.1 * (0.011 * q.0 / 12.0 + 0.1)
        });
        if mono.is_some() && !o.isotopi {
            c.isotopi += 1;
            continue;
        }
        let m1 = all
            .iter()
            .filter(|q| (q.0 - m - C13).abs() <= o.tol(m))
            .map(|q| q.1)
            .fold(None, |a: Option<f64>, x| Some(a.map_or(x, |a| a.max(x))));
        out.push((m, r, mono, m1.map(|x| x / r * 100.0)));
    }
    out
}

/// The spectra of this activation are in the graph (all with no choice).
fn attiva(o: &Options, att: &str) -> bool {
    o.activation
        .as_deref()
        .is_none_or(|a| att.eq_ignore_ascii_case(a))
}

/// All the observations of the tree, with the notes on the precursors.
fn osserva(ing: &Input, o: &Options, c: &mut Counts, warnings: &mut Vec<Warning>) -> Vec<Oss> {
    let nodes = &ing.albero.nodi;
    let mut obs: Vec<Oss> = vec![];
    for (t, n) in nodes.iter().enumerate() {
        let (_, att, _) = n.key.last().expect("a node of the tree has a path");
        let par = match n.genitore {
            Some(g) => ing.spettri.get(g),
            None => ing.ms1,
        };
        let (trovato, amb) = precursore(par, n.key.last().unwrap().0);
        if amb {
            warnings.push(Warning {
                chiave: "warn.msngrafo.isolation",
                albero: n.id.clone(),
                nodo: None,
            });
        }
        if trovato.is_none() {
            warnings.push(Warning {
                chiave: "warn.msngrafo.precNotFound",
                albero: n.id.clone(),
                nodo: None,
            });
        }
        let (pm, pr) = trovato.unwrap_or((n.key.last().unwrap().0, 0.0));
        let lp = n.genitore.map_or(1, |g| nodes[g].livello);
        obs.push(Oss {
            mz: pm,
            rel: pr,
            sorg: n.genitore,
            level: lp,
            prec_di: Some(t),
            iso_di: None,
            m1: None,
        });
        let Some(s) = ing.spettri.get(t).filter(|_| attiva(o, att)) else {
            continue;
        };
        let base = obs.len();
        for (m, r, mono, m1) in peaks(s, pm, o, c) {
            obs.push(Oss {
                mz: m,
                rel: r,
                sorg: Some(t),
                level: n.livello,
                prec_di: None,
                iso_di: mono.map(|i| base + i),
                m1,
            });
        }
    }
    obs
}

/// Groups of observations (indices) in m/z order: a new group where the step exceeds the tolerance, and a group wider than twice the
/// tolerance split at its widest step (no chains).
fn groups(obs: &[Oss], o: &Options) -> Vec<Vec<usize>> {
    let mut ord: Vec<usize> = (0..obs.len()).collect();
    crate::sortc::by(&mut ord, |a, b| {
        obs[*a].mz.total_cmp(&obs[*b].mz).then(a.cmp(b))
    });
    let mut grezzi: Vec<Vec<usize>> = vec![];
    for (j, &i) in ord.iter().enumerate() {
        if j == 0 || obs[i].mz - obs[ord[j - 1]].mz > o.tol(obs[i].mz) {
            grezzi.push(vec![]);
        }
        grezzi.last_mut().unwrap().push(i);
    }
    let mut out = vec![];
    let mut pila: Vec<Vec<usize>> = grezzi.into_iter().rev().collect();
    while let Some(gr) = pila.pop() {
        let (a, b) = (obs[gr[0]].mz, obs[*gr.last().unwrap()].mz);
        if gr.len() < 2 || b - a <= 2.0 * o.tol((a + b) / 2.0) {
            out.push(gr);
            continue;
        }
        let mut cut = 0;
        for k in 1..gr.len() - 1 {
            if obs[gr[k + 1]].mz - obs[gr[k]].mz > obs[gr[cut + 1]].mz - obs[gr[cut]].mz {
                cut = k;
            }
        }
        pila.push(gr[cut + 1..].to_vec());
        pila.push(gr[..=cut].to_vec());
    }
    out
}

/// The node of a group of observations (`isotopo_di` is filled later).
fn node(gr: &[usize], obs: &[Oss], ing: &Input) -> GNode {
    let peaks: Vec<&Oss> = gr
        .iter()
        .map(|&i| &obs[i])
        .filter(|x| x.prec_di.is_none())
        .collect();
    let mz = if peaks.is_empty() {
        gr.iter().map(|&i| obs[i].mz).sum::<f64>() / gr.len() as f64
    } else {
        let w: f64 = peaks.iter().map(|x| x.rel).sum();
        peaks.iter().map(|x| x.mz * x.rel).sum::<f64>() / w
    };
    let mut albero: Vec<String> = gr
        .iter()
        .filter_map(|&i| obs[i].prec_di)
        .map(|t| ing.albero.nodi[t].id.clone())
        .collect();
    crate::sortc::slice_ord(&mut albero);
    albero.dedup();
    let mut sorg: Vec<Option<usize>> = peaks.iter().map(|x| x.sorg).collect();
    crate::sortc::slice_ord(&mut sorg);
    sorg.dedup();
    let forte = peaks.iter().max_by(|a, b| a.rel.total_cmp(&b.rel));
    GNode {
        mz,
        dev_ppm: gr
            .iter()
            .map(|&i| (obs[i].mz - mz).abs() / mz * 1e6)
            .fold(0.0, f64::max),
        livello: gr.iter().map(|&i| obs[i].level).min().unwrap_or(1),
        albero,
        osservazioni: sorg.len(),
        rel_max: gr.iter().map(|&i| obs[i].rel).fold(0.0, f64::max),
        isotope_of: None,
        // the M+1 of the most intense peak of the node, and the windows that let it through
        m1: forte.and_then(|x| x.m1),
        larghezza: gr
            .iter()
            .filter_map(|&i| obs[i].prec_di.and_then(|t| larghezza(ing, t)))
            .reduce(f64::max),
        upstream: forte.and_then(|x| x.sorg).and_then(|t| a_monte(ing, t)),
        unexplained: None,
        pos: (0, 0),
        formula: None,
    }
}

fn larghezza(ing: &Input, t: usize) -> Option<f64> {
    ing.widths.get(t).copied().flatten()
}

/// The narrowest known isolation window on the path of node `t` of the tree (itself and its ancestors).
fn a_monte(ing: &Input, t: usize) -> Option<f64> {
    let mut m: Option<f64> = None;
    let mut c = Some(t);
    while let Some(k) = c {
        if let Some(w) = larghezza(ing, k) {
            m = Some(m.map_or(w, |x| x.min(w)));
        }
        c = ing.albero.nodi[k].genitore;
    }
    m
}

/// Every spectrum of the tree in the graph, with its peaks and their nodes.
fn spettri(ing: &Input, o: &Options, obs: &[Oss], di: &[usize], prec: &[usize]) -> Vec<SpettroG> {
    let mut out = vec![];
    for (t, n) in ing.albero.nodi.iter().enumerate() {
        let (_, att, ce) = n
            .key
            .last()
            .cloned()
            .expect("a node of the tree has a path");
        if ing.spettri.get(t).is_none() || !attiva(o, &att) {
            continue;
        }
        let peaks = obs
            .iter()
            .enumerate()
            .filter(|(_, x)| x.sorg == Some(t) && x.prec_di.is_none())
            .map(|(i, x)| (x.mz, x.rel, Some(di[i])))
            .collect();
        out.push(SpettroG {
            albero: n.id.clone(),
            level: n.livello,
            att,
            ce,
            precursore: Some(prec[t]),
            larghezza: larghezza(ing, t),
            picchi: peaks,
        });
    }
    out
}

/// The spectrum of node `t` of the tree as a measure of an arc.
fn measure(ing: &Input, t: usize, rel: f64) -> Measure {
    let n = &ing.albero.nodi[t];
    let (_, att, ce) = n.key.last().unwrap();
    Measure {
        albero: n.id.clone(),
        level: n.livello,
        att: att.clone(),
        ce: *ce,
        n_scansioni: n.scansioni.len(),
        rel,
    }
}

/// The arcs: an isolated precursor hangs from the precursor of its parent's spectrum, a peak from the precursor of its own spectrum; only
/// the spectra of the chosen activation give measures.
fn edges(
    ing: &Input,
    obs: &[Oss],
    di: &[usize],
    prec: &[usize],
    nodes: &[GNode],
    o: &Options,
) -> Vec<Edge> {
    let mut m: BTreeMap<(usize, usize), Vec<Measure>> = BTreeMap::new();
    for (i, x) in obs.iter().enumerate() {
        let Some(s) = x
            .sorg
            .filter(|&s| attiva(o, &ing.albero.nodi[s].key.last().unwrap().1))
        else {
            continue;
        };
        let (p, f) = (prec[s], di[i]);
        if p == f || nodes[f].mz >= nodes[p].mz {
            continue;
        }
        let v = m.entry((p, f)).or_default();
        match v.iter_mut().find(|y| y.albero == ing.albero.nodi[s].id) {
            Some(y) => y.rel = y.rel.max(x.rel),
            None => v.push(measure(ing, s, x.rel)),
        }
    }
    m.into_iter()
        .map(|((da, a), mut measures)| {
            crate::sortc::slice_by(&mut measures, |x, y| {
                x.level
                    .cmp(&y.level)
                    .then(x.att.cmp(&y.att))
                    .then(x.ce.total_cmp(&y.ce))
                    .then(x.albero.cmp(&y.albero))
            });
            let loss = nodes[da].mz - nodes[a].mz;
            let t = o.tol(nodes[da].mz) + o.tol(nodes[a].mz);
            let isolation = ing
                .albero
                .nodi
                .iter()
                .enumerate()
                .any(|(c, n)| prec[c] == a && n.genitore.is_some_and(|g| prec[g] == da));
            Edge {
                da,
                a,
                perdita: loss,
                compatibili: PERDITE
                    .iter()
                    .filter(|(_, pm)| (loss - pm).abs() <= t)
                    .map(|(f, _)| *f)
                    .collect(),
                formula: None,
                isolamento: isolation,
                misure: measures,
            }
        })
        .collect()
}

/// The graph without the drawing.
pub(super) fn merge(ing: &Input, o: &Options) -> Graph {
    let (mut discarded, mut warnings) = (Counts::default(), vec![]);
    let obs = osserva(ing, o, &mut discarded, &mut warnings);
    let gr = groups(&obs, o);
    let mut nodes: Vec<(GNode, usize)> = gr
        .iter()
        .enumerate()
        .map(|(k, g)| (node(g, &obs, ing), k))
        .collect();
    crate::sortc::slice_by(&mut nodes, |a, b| {
        a.0.livello
            .cmp(&b.0.livello)
            .then(a.0.mz.total_cmp(&b.0.mz))
            .then(a.1.cmp(&b.1))
    });
    let mut rank = vec![0usize; gr.len()];
    for (r, (_, k)) in nodes.iter().enumerate() {
        rank[*k] = r;
    }
    let mut di = vec![0usize; obs.len()];
    for (k, g) in gr.iter().enumerate() {
        for &i in g {
            di[i] = rank[k];
        }
    }
    let mut nodi: Vec<GNode> = nodes.into_iter().map(|(n, _)| n).collect();
    // a node made only of M+1 peaks (never isolated) is the isotope of the node of the first of them
    for (k, g) in gr.iter().enumerate() {
        let solo_iso = g.iter().all(|&i| obs[i].iso_di.is_some());
        if solo_iso {
            nodi[rank[k]].isotope_of = obs[g[0]].iso_di.map(|i| di[i]);
        }
    }
    let mut prec = vec![0usize; ing.albero.nodi.len()];
    for (i, x) in obs.iter().enumerate() {
        if let Some(t) = x.prec_di {
            prec[t] = di[i];
        }
    }
    let archi = edges(ing, &obs, &di, &prec, &nodi, o);
    let spettri = spettri(ing, o, &obs, &di, &prec);
    let radici = (0..nodi.len()).filter(|&i| nodi[i].livello == 1).collect();
    Graph {
        nodi,
        archi,
        radici,
        polarita: ing.polarita,
        scartati: discarded,
        avvisi: warnings,
        spettri,
        ..Graph::default()
    }
}
