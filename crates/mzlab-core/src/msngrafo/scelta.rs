//! The choice of the formulas of the graph (`docs/agenti/msn-grafo.md` §3): candidates of every node below the root, one calibration
//! shift, a reduction to a fixed point on the DAG (a fragment is a sub-formula of each precursor), then from the root down the best
//! candidate under the formulas already chosen for its precursors. Ties broken by the formula in Hill order: the same bytes every time.

use super::candidati::{punteggio, punteggio_perdita, sotto, Elementi};
use super::formule::avviso;
use super::{Alternativa, FormulaNodo, Grafo, Opzioni};
use crate::elements::parse_formula;

/// The second candidate is this close to the first in score: the proposal stands, the node is marked.
const AMBIGUA: f64 = 0.2;
/// The calibration needs this many nodes with a single candidate.
const MIN_CALIBRA: usize = 3;
const MAX_ALTERNATIVE: usize = 4;

struct Stato<'a> {
    e: &'a Elementi,
    r: usize,
    /// Nodes below the root, isotopes left out, in the order of the choice (m/z from high to low, the root first).
    ordine: Vec<usize>,
    genitori: Vec<Vec<usize>>,
    figli: Vec<Vec<usize>>,
    mz: Vec<f64>,
    tolp: Vec<f64>,
    cand: Vec<Vec<Vec<u32>>>,
    n_raw: Vec<usize>,
}

impl Stato<'_> {
    fn ppm(&self, j: usize, c: &[u32]) -> f64 {
        let t = self.e.mz(c);
        (self.mz[j] - t) / t * 1e6
    }

    fn formula(&self, j: usize, c: &[u32]) -> (String, f64, f64) {
        (self.e.testo(c), self.ppm(j, c), self.e.dbe(c))
    }
}

/// The nodes reached from the root along the arcs, isotopes left out.
fn raggiunti(g: &Grafo, r: usize) -> Vec<bool> {
    let mut vivo = vec![false; g.nodi.len()];
    vivo[r] = true;
    let mut pila = vec![r];
    while let Some(p) = pila.pop() {
        for a in g.archi.iter().filter(|a| a.da == p) {
            if !vivo[a.a] && g.nodi[a.a].isotopo_di.is_none() {
                vivo[a.a] = true;
                pila.push(a.a);
            }
        }
    }
    vivo
}

fn stato<'a>(g: &Grafo, e: &'a Elementi, r: usize, o: &Opzioni) -> Stato<'a> {
    let n = g.nodi.len();
    let vivo = raggiunti(g, r);
    let (mut genitori, mut figli) = (vec![vec![]; n], vec![vec![]; n]);
    for a in g.archi.iter().filter(|a| vivo[a.da] && vivo[a.a]) {
        genitori[a.a].push(a.da);
        figli[a.da].push(a.a);
    }
    let mut ordine: Vec<usize> = (0..n).filter(|&i| vivo[i] && i != r).collect();
    crate::sortc::by(&mut ordine, |a, b| {
        g.nodi[*b].mz.total_cmp(&g.nodi[*a].mz).then(a.cmp(b))
    });
    ordine.insert(0, r);
    let mz: Vec<f64> = g.nodi.iter().map(|x| x.mz).collect();
    let tolp = mz.iter().map(|m| o.tol(*m) / m * 1e6).collect();
    Stato {
        e,
        r,
        ordine,
        genitori,
        figli,
        mz,
        tolp,
        cand: vec![vec![]; n],
        n_raw: vec![0; n],
    }
}

fn candidati(st: &mut Stato, rc: &[u32]) {
    for k in 0..st.ordine.len() {
        let j = st.ordine[k];
        st.cand[j] = if j == st.r {
            vec![rc.to_vec()]
        } else {
            let t = st.tolp[j] * st.mz[j] * 1e-6;
            st.e.candidati(st.mz[j] - t, st.mz[j] + t)
        };
        st.n_raw[j] = st.cand[j].len();
    }
}

/// The median error of the nodes with a single candidate, removed from every m/z; `None` with fewer than `MIN_CALIBRA` of them.
fn calibra(st: &mut Stato, rc: &[u32]) -> Option<(f64, usize)> {
    let mut err: Vec<f64> = st.ordine[1..]
        .iter()
        .filter(|&&j| st.cand[j].len() == 1)
        .map(|&j| st.ppm(j, &st.cand[j][0]))
        .collect();
    if err.len() < MIN_CALIBRA {
        return None;
    }
    crate::sortc::by(&mut err, |a, b| a.total_cmp(b));
    let h = err.len() / 2;
    let med = if err.len() % 2 == 1 {
        err[h]
    } else {
        (err[h - 1] + err[h]) / 2.0
    };
    for m in st.mz.iter_mut() {
        *m /= 1.0 + med * 1e-6;
    }
    candidati(st, rc);
    Some((med, err.len()))
}

/// To a fixed point: a candidate of a fragment must be under a candidate of each precursor that has some; a candidate of an isolated
/// precursor must contain a candidate of each fragment that has some (unless none would be left).
fn riduci(g: &Grafo, st: &mut Stato) {
    loop {
        let mut cambiato = false;
        for &j in &st.ordine[1..] {
            for &p in &st.genitori[j] {
                if st.cand[p].is_empty() {
                    continue;
                }
                let (cp, n) = (st.cand[p].clone(), st.cand[j].len());
                st.cand[j].retain(|c| cp.iter().any(|q| sotto(c, q)));
                cambiato |= st.cand[j].len() < n;
            }
        }
        for &p in &st.ordine[1..] {
            if g.nodi[p].albero.is_empty() {
                continue;
            }
            for &f in &st.figli[p] {
                let cf = &st.cand[f];
                let tieni: Vec<Vec<u32>> = st.cand[p]
                    .iter()
                    .filter(|q| cf.is_empty() || cf.iter().any(|c| sotto(c, q)))
                    .cloned()
                    .collect();
                if !tieni.is_empty() && tieni.len() < st.cand[p].len() {
                    st.cand[p] = tieni;
                    cambiato = true;
                }
            }
        }
        if !cambiato {
            break;
        }
    }
}

/// The formulas fixed by the user, on the node closest to their m/z within the tolerance.
fn fissate(g: &mut Grafo, st: &Stato, o: &Opzioni) -> Vec<Option<Vec<u32>>> {
    let mut out = vec![None; g.nodi.len()];
    for (mz, testo) in &o.fissate {
        let j = st.ordine[1..]
            .iter()
            .copied()
            .filter(|&j| (g.nodi[j].mz - mz).abs() <= o.tol(*mz))
            .min_by(|&a, &b| {
                (g.nodi[a].mz - mz)
                    .abs()
                    .total_cmp(&(g.nodi[b].mz - mz).abs())
            });
        let Some(j) = j else {
            avviso(g, "warn.msngrafo.fixedNotFound", None);
            continue;
        };
        let c = parse_formula(testo.trim_end_matches(['+', '-']))
            .ok()
            .and_then(|f| st.e.conta(&f));
        match c {
            Some(c) => out[j] = Some(c),
            None => avviso(g, "warn.msngrafo.userConflict", Some(j)),
        }
    }
    out
}

fn alternative(
    st: &Stato,
    j: usize,
    ord: &[(f64, String, Vec<u32>)],
    via: &[u32],
) -> Vec<Alternativa> {
    ord.iter()
        .filter(|x| x.2 != via)
        .take(MAX_ALTERNATIVE)
        .map(|(s, f, c)| Alternativa {
            formula: f.clone(),
            ppm: st.ppm(j, c),
            dbe: st.e.dbe(c),
            punteggio: *s,
        })
        .collect()
}

/// The formula of node `j` under the formulas chosen for its precursors; the bool says that a fixed formula breaks a constraint.
fn nodo(
    st: &Stato,
    j: usize,
    scelte: &[Option<Vec<u32>>],
    fissata: Option<&Vec<u32>>,
) -> (Option<(FormulaNodo, Vec<u32>)>, bool) {
    let sopra: Vec<&Vec<u32>> = st.genitori[j]
        .iter()
        .filter_map(|&p| scelte[p].as_ref())
        .collect();
    let mut ord: Vec<(f64, String, Vec<u32>)> = st.cand[j]
        .iter()
        .filter(|c| sopra.iter().all(|p| sotto(c, p)))
        .map(|c| {
            let s = punteggio(st.e, c, st.ppm(j, c), st.tolp[j])
                + sopra
                    .iter()
                    .map(|p| punteggio_perdita(st.e, p, c))
                    .sum::<f64>();
            (s, st.e.testo(c), c.clone())
        })
        .collect();
    crate::sortc::slice_by(&mut ord, |a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let (c, motivo, fonte, ambigua) = match (fissata, ord.first()) {
        (Some(c), _) => (c.clone(), ("why.msngrafo.user", vec![]), "utente", false),
        (None, None) => return (None, false),
        (None, Some(x)) => {
            let motivo = if st.n_raw[j] == 1 {
                ("why.msngrafo.unique", vec![("ppm", st.tolp[j])])
            } else if ord.len() == 1 {
                ("why.msngrafo.tree", vec![("n", st.n_raw[j] as f64)])
            } else {
                ("why.msngrafo.best", vec![("n", ord.len() as f64)])
            };
            let amb = ord.len() > 1 && x.0 - ord[1].0 < AMBIGUA;
            (x.2.clone(), motivo, "auto", amb)
        }
    };
    let conflitto = !sopra.iter().all(|p| sotto(&c, p));
    let (proposta, ppm, dbe) = st.formula(j, &c);
    let f = FormulaNodo {
        proposta,
        ppm,
        dbe,
        pari: st.e.pari(&c, st.e.pol),
        n_candidati: st.n_raw[j],
        alternative: alternative(st, j, &ord, &c),
        motivo,
        fonte,
        ambigua,
    };
    (Some((f, c)), conflitto)
}

fn radice(g: &Grafo, st: &Stato, rc: &[u32]) -> FormulaNodo {
    let (proposta, _, dbe) = st.formula(st.r, rc);
    FormulaNodo {
        proposta,
        ppm: g.ione.as_ref().map_or(0.0, |i| i.ppm),
        dbe,
        pari: st.e.pari(rc, st.e.pol),
        n_candidati: 1,
        alternative: vec![],
        motivo: ("why.msngrafo.root", vec![]),
        fonte: "radice",
        ambigua: false,
    }
}

/// The loss of every arc between two nodes with a formula, the fragment under the precursor.
fn perdite(g: &mut Grafo, st: &Stato, scelte: &[Option<Vec<u32>>]) {
    for a in g.archi.iter_mut() {
        if let (Some(p), Some(c)) = (&scelte[a.da], &scelte[a.a]) {
            if sotto(c, p) {
                let d: Vec<u32> = p.iter().zip(c).map(|(x, y)| x - y).collect();
                let teo = st.e.massa(&d);
                let oss = st.mz[a.da] - st.mz[a.a];
                a.formula = Some((st.e.testo_perdita(&d), (oss - teo) / st.mz[a.da] * 1e6));
            }
        }
    }
}

/// The formulas of the nodes below root `r` (counts `rc`), of their losses, and the calibration.
pub(super) fn scegli(g: &mut Grafo, e: &Elementi, r: usize, rc: Vec<u32>, o: &Opzioni) {
    let mut st = stato(g, e, r, o);
    candidati(&mut st, &rc);
    if o.calibra {
        g.calibrazione = calibra(&mut st, &rc);
    }
    riduci(g, &mut st);
    let fisse = fissate(g, &st, o);
    let mut scelte: Vec<Option<Vec<u32>>> = vec![None; g.nodi.len()];
    scelte[r] = Some(rc.clone());
    g.nodi[r].formula = Some(radice(g, &st, &rc));
    for k in 1..st.ordine.len() {
        let j = st.ordine[k];
        let (f, conflitto) = nodo(&st, j, &scelte, fisse[j].as_ref());
        if conflitto {
            avviso(g, "warn.msngrafo.userConflict", Some(j));
        }
        if f.is_none() && st.n_raw[j] > 0 {
            avviso(g, "warn.msngrafo.noFormula", Some(j));
        }
        if let Some((f, c)) = f {
            scelte[j] = Some(c);
            g.nodi[j].formula = Some(f);
        }
    }
    perdite(g, &st, &scelte);
}
