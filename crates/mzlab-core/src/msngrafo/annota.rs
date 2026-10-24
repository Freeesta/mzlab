//! The formulas of the fragments from the ion of the root given by the user (`docs/agenti/msn-grafo.md` §3): the sub-formulas of the root
//! within the window of each node, reduced to a fixed point on the DAG (a fragment is under a formula of each precursor, a candidate of the
//! wide low-mass window needs its evidences), the choices of the user, then the nodes with no candidate leave the graph. No score, no proposal.

mod ricalibra;
use ricalibra::{preferisci_pari, ricalibra};

use super::candidati::{m1_coerente, perdita_nota, sotto, Elementi};
use super::formule::avviso;
use super::regole_valuta::{addotto_radice, vettori_perdita};
use super::{Avviso, Candidato, FormulaNodo, Grafo, NonSpiegati, Opzioni, STRETTA};
use crate::elements::parse_formula;

struct Cand {
    c: Vec<u32>,
    ppm: f64,
    stretto: bool,
    prove: Vec<&'static str>,
}

struct Stato<'a> {
    e: &'a Elementi,
    r: usize,
    /// The nodes reached from the root, isotopes left out: the root, then by m/z from high to low.
    ordine: Vec<usize>,
    genitori: Vec<Vec<usize>>,
    figli: Vec<Vec<usize>>,
    cand: Vec<Vec<Cand>>,
    utente: Vec<bool>,
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

/// The window of a node, in ppm: wider below `mz_basso`.
fn finestra(o: &Opzioni, mz: f64) -> f64 {
    if mz < o.mz_basso {
        o.ppm_basso.max(o.ppm_formula)
    } else {
        o.ppm_formula
    }
}

fn cand(e: &Elementi, o: &Opzioni, mz: f64, c: Vec<u32>) -> Cand {
    let t = e.mz(&c);
    let ppm = (mz - t) / t * 1e6;
    Cand {
        c,
        ppm,
        stretto: ppm.abs() <= o.ppm_formula,
        prove: vec![],
    }
}

fn stato<'a>(g: &Grafo, e: &'a Elementi, r: usize, rc: &[u32], o: &Opzioni) -> Stato<'a> {
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
    let mut cs: Vec<Vec<Cand>> = (0..n).map(|_| vec![]).collect();
    cs[r] = vec![cand(e, o, g.nodi[r].mz, rc.to_vec())];
    for &j in &ordine[1..] {
        let mz = g.nodi[j].mz;
        let t = finestra(o, mz) * mz * 1e-6;
        cs[j] = e
            .candidati(mz - t, mz + t)
            .into_iter()
            .map(|c| cand(e, o, mz, c))
            .collect();
    }
    Stato {
        e,
        r,
        ordine,
        genitori,
        figli,
        cand: cs,
        utente: vec![false; n],
    }
}

/// The formulas chosen by the user, on the node closest to their m/z within its window: the only candidate of that node.
fn scelte(g: &mut Grafo, st: &mut Stato, o: &Opzioni) {
    for (mz, testo) in &o.scelte {
        let j = st.ordine[1..]
            .iter()
            .copied()
            .filter(|&j| (g.nodi[j].mz - mz).abs() <= finestra(o, *mz) * mz * 1e-6)
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
        let Some(c) = c else {
            avviso(g, "warn.msngrafo.userConflict", Some(j));
            continue;
        };
        if !st.cand[j].iter().any(|x| x.c == c) {
            avviso(g, "warn.msngrafo.userConflict", Some(j));
        }
        st.cand[j] = vec![cand(st.e, o, g.nodi[j].mz, c)];
        st.utente[j] = true;
    }
}

/// The evidences of a candidate of node `j` (for one of the wide window).
fn prove(g: &Grafo, st: &Stato, o: &Opzioni, j: usize, c: &[u32]) -> Vec<&'static str> {
    let mut v = vec![];
    let perdita = st.genitori[j].iter().any(|&p| {
        st.cand[p]
            .iter()
            .any(|q| sotto(c, &q.c) && perdita_nota(st.e, &q.c, c))
    });
    if perdita {
        v.push("perdita");
    }
    if o.regole.is_some() && perdita_regola(g, st, o, j, c) {
        v.push("regola");
    }
    // behind a window narrower than the M+1 the isotopes are not expected: no evidence for or against
    let stretta = g.nodi[j].a_monte.is_some_and(|w| w < STRETTA);
    if !stretta && g.nodi[j].m1.is_some_and(|x| m1_coerente(st.e, c, x)) {
        v.push("isotopi");
    }
    let msn = !g.nodi[j].albero.is_empty()
        && st.figli[j]
            .iter()
            .any(|&f| st.cand[f].iter().any(|x| x.stretto && sotto(&x.c, c)));
    if msn {
        v.push("msn");
    }
    v
}

/// The loss from a candidate of a precursor of node `j` to `c` is the one of a rule of `fragrules/1`.
fn perdita_regola(g: &Grafo, st: &Stato, o: &Opzioni, j: usize, c: &[u32]) -> bool {
    let add = addotto_radice(o, g.polarita);
    st.genitori[j].iter().any(|&p| {
        let vs = vettori_perdita(st.e, g.polarita, add, g.radici.contains(&p));
        st.cand[p].iter().any(|q| {
            sotto(c, &q.c) && {
                let d: Vec<u32> = q.c.iter().zip(c).map(|(a, b)| a - b).collect();
                vs.contains(&d)
            }
        })
    })
}

/// One pass of the reduction; true when something changed.
fn passo(g: &Grafo, st: &mut Stato, o: &Opzioni) -> bool {
    let mut cambiato = false;
    for k in 1..st.ordine.len() {
        let j = st.ordine[k];
        if st.utente[j] {
            continue;
        }
        let n = st.cand[j].len();
        // under a candidate of each precursor that has some, and at least one has some (a fragment only of unexplained ions is not one)
        let spiegati = st.genitori[j].iter().any(|&p| !st.cand[p].is_empty());
        let tieni: Vec<bool> = st.cand[j]
            .iter()
            .map(|x| {
                spiegati
                    && st.genitori[j].iter().all(|&p| {
                        st.cand[p].is_empty() || st.cand[p].iter().any(|q| sotto(&x.c, &q.c))
                    })
            })
            .collect();
        filtra(&mut st.cand[j], &tieni);
        let pr: Vec<Vec<&'static str>> = st.cand[j]
            .iter()
            .map(|x| {
                if x.stretto {
                    vec![]
                } else {
                    prove(g, st, o, j, &x.c)
                }
            })
            .collect();
        for (x, p) in st.cand[j].iter_mut().zip(pr) {
            x.prove = p;
        }
        st.cand[j].retain(|x| x.stretto || x.prove.len() >= o.prove_min);
        cambiato |= st.cand[j].len() < n;
    }
    cambiato | contiene(g, st)
}

/// A candidate of an isolated precursor contains a candidate of each fragment that has some (unless none would be left).
fn contiene(g: &Grafo, st: &mut Stato) -> bool {
    let mut cambiato = false;
    for k in 1..st.ordine.len() {
        let p = st.ordine[k];
        if g.nodi[p].albero.is_empty() || st.utente[p] {
            continue;
        }
        for &f in &st.figli[p] {
            let ok: Vec<bool> = st.cand[p]
                .iter()
                .map(|q| st.cand[f].is_empty() || st.cand[f].iter().any(|c| sotto(&c.c, &q.c)))
                .collect();
            if ok.iter().any(|x| *x) && !ok.iter().all(|x| *x) {
                filtra(&mut st.cand[p], &ok);
                cambiato = true;
            }
        }
    }
    cambiato
}

fn formula(g: &Grafo, st: &Stato, o: &Opzioni, j: usize) -> FormulaNodo {
    let mut cs: Vec<Candidato> = st.cand[j]
        .iter()
        .map(|x| Candidato {
            formula: st.e.testo(&x.c),
            mz_teorico: st.e.mz(&x.c),
            ppm: x.ppm,
            dbe: st.e.dbe(&x.c),
            pari: st.e.pari(&x.c, st.e.pol),
            finestra: if x.stretto {
                o.ppm_formula
            } else {
                finestra(o, g.nodi[j].mz)
            },
            prove: x.prove.clone(),
        })
        .collect();
    crate::sortc::slice_by(&mut cs, |a, b| {
        a.ppm
            .abs()
            .total_cmp(&b.ppm.abs())
            .then(a.formula.cmp(&b.formula))
    });
    let fonte = if j == st.r {
        Some("radice")
    } else if st.utente[j] {
        Some("utente")
    } else if cs.len() == 1 {
        Some("unica")
    } else {
        None
    };
    FormulaNodo {
        scelta: fonte.map(|_| cs[0].formula.clone()),
        candidati: cs,
        fonte,
    }
}

/// The loss of every arc between two nodes with a choice, the fragment under the precursor.
fn perdite(g: &mut Grafo, st: &Stato) {
    let scelta = |i: usize| {
        let f = g.nodi[i].formula.as_ref()?;
        f.scelta.as_ref()?;
        Some(st.cand[i][0].c.clone())
    };
    let v: Vec<Option<(String, f64)>> = g
        .archi
        .iter()
        .map(|a| {
            let (p, c) = (scelta(a.da)?, scelta(a.a)?);
            if !sotto(&c, &p) {
                return None;
            }
            let d: Vec<u32> = p.iter().zip(&c).map(|(x, y)| x - y).collect();
            let oss = g.nodi[a.da].mz - g.nodi[a.a].mz;
            let teo = st.e.massa(&d);
            Some((st.e.testo_perdita(&d), (oss - teo) / g.nodi[a.da].mz * 1e6))
        })
        .collect();
    for (a, f) in g.archi.iter_mut().zip(v) {
        a.formula = f;
    }
}

/// Keeps the items whose flag is true.
fn filtra<T>(v: &mut Vec<T>, tieni: &[bool]) {
    let mut f = tieni.iter();
    v.retain(|_| *f.next().unwrap_or(&false));
}

/// The peaks of each spectrum whose node leaves the graph, on the node of its precursor: how many nodes, the 3 most intense.
fn non_spiegati(g: &mut Grafo, tieni: &[bool]) {
    let mut per: Vec<Vec<(usize, f64, f64)>> = vec![vec![]; g.nodi.len()];
    for sp in &g.spettri {
        let Some(p) = sp.precursore.filter(|&p| tieni[p]) else {
            continue;
        };
        for &(mz, rel, k) in &sp.picchi {
            if let Some(k) = k.filter(|&k| !tieni[k]) {
                per[p].push((k, mz, rel));
            }
        }
    }
    for (p, mut v) in per.into_iter().enumerate() {
        if v.is_empty() {
            continue;
        }
        // one entry per node left out, its most intense peak
        crate::sortc::slice_by(&mut v, |a, b| a.0.cmp(&b.0).then(b.2.total_cmp(&a.2)));
        v.dedup_by_key(|x| x.0);
        crate::sortc::slice_by(&mut v, |a, b| b.2.total_cmp(&a.2).then(a.1.total_cmp(&b.1)));
        g.nodi[p].non_spiegati = Some(NonSpiegati {
            n: v.len(),
            picchi: v.iter().take(3).map(|x| (x.1, x.2)).collect(),
        });
    }
}

/// Share of the fragment intensity (sum of % of the base peak, spectra of kept precursors) left without a node above which the graph warns.
const QUOTA_NON_SPIEGATA: f64 = 0.5;

/// A graph-level note when the formula leaves most of the fragment intensity unexplained (a wrong ion formula or adduct looks like this).
fn muchi_non_spiegati(g: &mut Grafo) {
    let (mut tot, mut fuori) = (0.0, 0.0);
    for sp in g.spettri.iter().filter(|s| s.precursore.is_some()) {
        for &(_, rel, k) in &sp.picchi {
            tot += rel;
            if k.is_none() {
                fuori += rel;
            }
        }
    }
    if tot > 0.0 && fuori / tot >= QUOTA_NON_SPIEGATA {
        avviso(g, "warn.msngrafo.mostUnexplained", None);
    }
}

/// The nodes with a formula stay (with their isotopes), the others leave the graph; ids, arcs, spectra and notes follow.
fn togli(g: &mut Grafo, tieni: &[bool]) {
    non_spiegati(g, tieni);
    let mut nuovo = vec![None; g.nodi.len()];
    let mut k = 0;
    for (i, t) in tieni.iter().enumerate() {
        if *t {
            nuovo[i] = Some(k);
            k += 1;
        }
    }
    let mut tolti: Vec<String> = g
        .nodi
        .iter()
        .zip(tieni)
        .filter(|(_, t)| !**t)
        .flat_map(|(n, _)| n.albero.iter().cloned())
        .collect();
    g.scartati.formula = g.nodi.len() - k;
    filtra(&mut g.nodi, tieni);
    for n in g.nodi.iter_mut() {
        n.isotopo_di = n.isotopo_di.and_then(|x| nuovo[x]);
    }
    g.archi.retain(|a| tieni[a.da] && tieni[a.a]);
    for a in g.archi.iter_mut() {
        (a.da, a.a) = (nuovo[a.da].unwrap(), nuovo[a.a].unwrap());
    }
    g.radici = g.radici.iter().filter_map(|&r| nuovo[r]).collect();
    for sp in g.spettri.iter_mut() {
        sp.precursore = sp.precursore.and_then(|x| nuovo[x]);
        for p in sp.picchi.iter_mut() {
            p.2 = p.2.and_then(|x| nuovo[x]);
        }
    }
    for a in g.avvisi.iter_mut() {
        a.nodo = a.nodo.and_then(|x| nuovo[x]);
    }
    muchi_non_spiegati(g);
    crate::sortc::slice_ord(&mut tolti);
    g.avvisi.extend(tolti.into_iter().map(|albero| Avviso {
        chiave: "warn.msngrafo.isolatedUnexplained",
        albero,
        nodo: None,
    }));
}

/// The formulas of the nodes below root `r` (counts `rc`) and of their losses; the nodes they do not explain leave the graph.
pub(super) fn annota(g: &mut Grafo, e: &Elementi, r: usize, rc: Vec<u32>, o: &Opzioni) {
    let mut st = stato(g, e, r, &rc, o);
    scelte(g, &mut st, o);
    while passo(g, &mut st, o) {}
    if preferisci_pari(g, &mut st, o) {
        while passo(g, &mut st, o) {}
    }
    if ricalibra(&mut st, o) {
        while passo(g, &mut st, o) {}
    }
    for k in 1..st.ordine.len() {
        let j = st.ordine[k];
        let fuori = st.genitori[j].iter().any(|&p| {
            !st.cand[p].is_empty()
                && !st.cand[j]
                    .iter()
                    .any(|x| st.cand[p].iter().any(|q| sotto(&x.c, &q.c)))
        });
        let gia = g
            .avvisi
            .iter()
            .any(|a| a.chiave == "warn.msngrafo.userConflict" && a.nodo == Some(j));
        if st.utente[j] && fuori && !gia {
            avviso(g, "warn.msngrafo.userConflict", Some(j));
        }
    }
    let mut tieni = vec![false; g.nodi.len()];
    for &j in &st.ordine {
        if !st.cand[j].is_empty() {
            g.nodi[j].formula = Some(formula(g, &st, o, j));
            tieni[j] = true;
        }
    }
    for i in 0..g.nodi.len() {
        if g.nodi[i].isotopo_di.is_some_and(|m| tieni[m]) {
            tieni[i] = true;
        }
    }
    perdite(g, &st);
    togli(g, &tieni);
}
