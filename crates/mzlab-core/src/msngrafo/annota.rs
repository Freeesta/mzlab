//! The formulas of the fragments from the ion of the root given by the user (`docs/agenti/msn-grafo.md` §3): the sub-formulas of the root
//! within the window of each node, reduced to a fixed point on the DAG (a fragment is under a formula of each precursor, a candidate of the
//! wide low-mass window needs its evidences), the choices of the user, then the nodes with no candidate leave the graph. No score, no proposal.

mod ricalibra;
use ricalibra::{prefer_even, ricalibra};

use super::candidati::{below, known_loss, m1_consistent, Elementi};
use super::formule::warning;
use super::regole_valuta::{loss_vectors, root_adduct};
use super::{Candidate, Graph, NodeFormula, Options, Unexplained, Warning, STRETTA};
use crate::elements::parse_formula;

struct Cand {
    c: Vec<u32>,
    ppm: f64,
    strict: bool,
    evidence: Vec<&'static str>,
}

struct State<'a> {
    e: &'a Elementi,
    r: usize,
    /// The nodes reached from the root, isotopes left out: the root, then by m/z from high to low.
    order: Vec<usize>,
    genitori: Vec<Vec<usize>>,
    figli: Vec<Vec<usize>>,
    cand: Vec<Vec<Cand>>,
    user: Vec<bool>,
}

/// The nodes reached from the root along the arcs, isotopes left out.
fn raggiunti(g: &Graph, r: usize) -> Vec<bool> {
    let mut vivo = vec![false; g.nodi.len()];
    vivo[r] = true;
    let mut pila = vec![r];
    while let Some(p) = pila.pop() {
        for a in g.archi.iter().filter(|a| a.da == p) {
            if !vivo[a.a] && g.nodi[a.a].isotope_of.is_none() {
                vivo[a.a] = true;
                pila.push(a.a);
            }
        }
    }
    vivo
}

/// The window of a node, in ppm: wider below `mz_basso`.
fn finestra(o: &Options, mz: f64) -> f64 {
    if mz < o.mz_low {
        o.ppm_low.max(o.ppm_formula)
    } else {
        o.ppm_formula
    }
}

fn cand(e: &Elementi, o: &Options, mz: f64, c: Vec<u32>) -> Cand {
    let t = e.mz(&c);
    let ppm = (mz - t) / t * 1e6;
    Cand {
        c,
        ppm,
        strict: ppm.abs() <= o.ppm_formula,
        evidence: vec![],
    }
}

fn state<'a>(g: &Graph, e: &'a Elementi, r: usize, rc: &[u32], o: &Options) -> State<'a> {
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
            .candidates(mz - t, mz + t)
            .into_iter()
            .map(|c| cand(e, o, mz, c))
            .collect();
    }
    State {
        e,
        r,
        order: ordine,
        genitori,
        figli,
        cand: cs,
        user: vec![false; n],
    }
}

/// The formulas chosen by the user, on the node closest to their m/z within its window: the only candidate of that node.
fn choices(g: &mut Graph, st: &mut State, o: &Options) {
    for (mz, text) in &o.choices {
        let j = st.order[1..]
            .iter()
            .copied()
            .filter(|&j| (g.nodi[j].mz - mz).abs() <= finestra(o, *mz) * mz * 1e-6)
            .min_by(|&a, &b| {
                (g.nodi[a].mz - mz)
                    .abs()
                    .total_cmp(&(g.nodi[b].mz - mz).abs())
            });
        let Some(j) = j else {
            warning(g, "warn.msngrafo.fixedNotFound", None);
            continue;
        };
        let c = parse_formula(text.trim_end_matches(['+', '-']))
            .ok()
            .and_then(|f| st.e.conta(&f));
        let Some(c) = c else {
            warning(g, "warn.msngrafo.userConflict", Some(j));
            continue;
        };
        if !st.cand[j].iter().any(|x| x.c == c) {
            warning(g, "warn.msngrafo.userConflict", Some(j));
        }
        st.cand[j] = vec![cand(st.e, o, g.nodi[j].mz, c)];
        st.user[j] = true;
    }
}

/// The evidences of a candidate of node `j` (for one of the wide window).
fn evidence(g: &Graph, st: &State, o: &Options, j: usize, c: &[u32]) -> Vec<&'static str> {
    let mut v = vec![];
    let loss = st.genitori[j].iter().any(|&p| {
        st.cand[p]
            .iter()
            .any(|q| below(c, &q.c) && known_loss(st.e, &q.c, c))
    });
    if loss {
        v.push("perdita");
    }
    if o.regole.is_some() && rule_loss(g, st, o, j, c) {
        v.push("regola");
    }
    // behind a window narrower than the M+1 the isotopes are not expected: no evidence for or against
    let stretta = g.nodi[j].upstream.is_some_and(|w| w < STRETTA);
    if !stretta && g.nodi[j].m1.is_some_and(|x| m1_consistent(st.e, c, x)) {
        v.push("isotopi");
    }
    let msn = !g.nodi[j].albero.is_empty()
        && st.figli[j]
            .iter()
            .any(|&f| st.cand[f].iter().any(|x| x.strict && below(&x.c, c)));
    if msn {
        v.push("msn");
    }
    v
}

/// The loss from a candidate of a precursor of node `j` to `c` is the one of a rule of `fragrules/1`.
fn rule_loss(g: &Graph, st: &State, o: &Options, j: usize, c: &[u32]) -> bool {
    let add = root_adduct(o, g.polarita);
    st.genitori[j].iter().any(|&p| {
        let vs = loss_vectors(st.e, g.polarita, add, g.radici.contains(&p));
        st.cand[p].iter().any(|q| {
            below(c, &q.c) && {
                let d: Vec<u32> = q.c.iter().zip(c).map(|(a, b)| a - b).collect();
                vs.contains(&d)
            }
        })
    })
}

/// One pass of the reduction; true when something changed.
fn step(g: &Graph, st: &mut State, o: &Options) -> bool {
    let mut changed = false;
    for k in 1..st.order.len() {
        let j = st.order[k];
        if st.user[j] {
            continue;
        }
        let n = st.cand[j].len();
        // under a candidate of each precursor that has some, and at least one has some (a fragment only of unexplained ions is not one)
        let explained = st.genitori[j].iter().any(|&p| !st.cand[p].is_empty());
        let tieni: Vec<bool> = st.cand[j]
            .iter()
            .map(|x| {
                explained
                    && st.genitori[j].iter().all(|&p| {
                        st.cand[p].is_empty() || st.cand[p].iter().any(|q| below(&x.c, &q.c))
                    })
            })
            .collect();
        // a precursor that does not hold any candidate (a co-isolated ion, a wrong formula) must not take a fragment away from the others
        let tieni = if tieni.iter().any(|x| *x) {
            tieni
        } else {
            st.cand[j]
                .iter()
                .map(|x| {
                    st.genitori[j]
                        .iter()
                        .any(|&p| st.cand[p].iter().any(|q| below(&x.c, &q.c)))
                })
                .collect()
        };
        retain_mask(&mut st.cand[j], &tieni);
        let pr: Vec<Vec<&'static str>> = st.cand[j]
            .iter()
            .map(|x| {
                if x.strict {
                    vec![]
                } else {
                    evidence(g, st, o, j, &x.c)
                }
            })
            .collect();
        for (x, p) in st.cand[j].iter_mut().zip(pr) {
            x.evidence = p;
        }
        st.cand[j].retain(|x| x.strict || x.evidence.len() >= o.evidence_min);
        changed |= st.cand[j].len() < n;
    }
    changed | contains(g, st)
}

/// A candidate of an isolated precursor contains a candidate of each fragment that has some (unless none would be left).
fn contains(g: &Graph, st: &mut State) -> bool {
    let mut changed = false;
    for k in 1..st.order.len() {
        let p = st.order[k];
        if g.nodi[p].albero.is_empty() || st.user[p] {
            continue;
        }
        for &f in &st.figli[p] {
            let ok: Vec<bool> = st.cand[p]
                .iter()
                .map(|q| st.cand[f].is_empty() || st.cand[f].iter().any(|c| below(&c.c, &q.c)))
                .collect();
            if ok.iter().any(|x| *x) && !ok.iter().all(|x| *x) {
                retain_mask(&mut st.cand[p], &ok);
                changed = true;
            }
        }
    }
    changed
}

fn formula(g: &Graph, st: &State, o: &Options, j: usize) -> NodeFormula {
    let mut cs: Vec<Candidate> = st.cand[j]
        .iter()
        .map(|x| Candidate {
            formula: st.e.testo(&x.c),
            theoretical_mz: st.e.mz(&x.c),
            ppm: x.ppm,
            dbe: st.e.dbe(&x.c),
            even: st.e.even(&x.c, st.e.pol),
            window: if x.strict {
                o.ppm_formula
            } else {
                finestra(o, g.nodi[j].mz)
            },
            prove: x.evidence.clone(),
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
    } else if st.user[j] {
        Some("utente")
    } else if cs.len() == 1 {
        Some("unica")
    } else {
        None
    };
    NodeFormula {
        choice: fonte.map(|_| cs[0].formula.clone()),
        candidates: cs,
        provenance: fonte,
    }
}

/// The loss of every arc between two nodes with a choice, the fragment under the precursor.
fn losses(g: &mut Graph, st: &State) {
    let choice = |i: usize| {
        let f = g.nodi[i].formula.as_ref()?;
        f.choice.as_ref()?;
        Some(st.cand[i][0].c.clone())
    };
    let v: Vec<Option<(String, f64)>> = g
        .archi
        .iter()
        .map(|a| {
            let (p, c) = (choice(a.da)?, choice(a.a)?);
            if !below(&c, &p) {
                return None;
            }
            let d: Vec<u32> = p.iter().zip(&c).map(|(x, y)| x - y).collect();
            let oss = g.nodi[a.da].mz - g.nodi[a.a].mz;
            let teo = st.e.mass(&d);
            Some((st.e.loss_text(&d), (oss - teo) / g.nodi[a.da].mz * 1e6))
        })
        .collect();
    for (a, f) in g.archi.iter_mut().zip(v) {
        a.formula = f;
    }
}

/// Keeps the items whose flag is true.
fn retain_mask<T>(v: &mut Vec<T>, tieni: &[bool]) {
    let mut f = tieni.iter();
    v.retain(|_| *f.next().unwrap_or(&false));
}

/// The peaks of each spectrum whose node leaves the graph, on the node of its precursor: how many nodes, the 3 most intense.
fn unexplained(g: &mut Graph, tieni: &[bool]) {
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
        g.nodi[p].unexplained = Some(Unexplained {
            n: v.len(),
            peaks: v.iter().take(3).map(|x| (x.1, x.2)).collect(),
        });
    }
}

/// Share of the fragment intensity (sum of % of the base peak, spectra of kept precursors) left without a node above which the graph warns.
const UNEXPLAINED_SHARE: f64 = 0.5;

/// A graph-level note when the formula leaves most of the fragment intensity unexplained (a wrong ion formula or adduct looks like this).
fn muchi_non_spiegati(g: &mut Graph) {
    let (mut tot, mut outside) = (0.0, 0.0);
    for sp in g.spettri.iter().filter(|s| s.precursore.is_some()) {
        for &(_, rel, k) in &sp.picchi {
            tot += rel;
            if k.is_none() {
                outside += rel;
            }
        }
    }
    if tot > 0.0 && outside / tot >= UNEXPLAINED_SHARE {
        warning(g, "warn.msngrafo.mostUnexplained", None);
    }
}

/// The nodes with a formula stay (with their isotopes), the others leave the graph; ids, arcs, spectra and notes follow.
fn togli(g: &mut Graph, tieni: &[bool]) {
    unexplained(g, tieni);
    let mut new = vec![None; g.nodi.len()];
    let mut k = 0;
    for (i, t) in tieni.iter().enumerate() {
        if *t {
            new[i] = Some(k);
            k += 1;
        }
    }
    let mut removed: Vec<String> = g
        .nodi
        .iter()
        .zip(tieni)
        .filter(|(_, t)| !**t)
        .flat_map(|(n, _)| n.albero.iter().cloned())
        .collect();
    g.scartati.formula = g.nodi.len() - k;
    retain_mask(&mut g.nodi, tieni);
    for n in g.nodi.iter_mut() {
        n.isotope_of = n.isotope_of.and_then(|x| new[x]);
    }
    g.archi.retain(|a| tieni[a.da] && tieni[a.a]);
    for a in g.archi.iter_mut() {
        (a.da, a.a) = (new[a.da].unwrap(), new[a.a].unwrap());
    }
    g.radici = g.radici.iter().filter_map(|&r| new[r]).collect();
    for sp in g.spettri.iter_mut() {
        sp.precursore = sp.precursore.and_then(|x| new[x]);
        for p in sp.picchi.iter_mut() {
            p.2 = p.2.and_then(|x| new[x]);
        }
    }
    for a in g.avvisi.iter_mut() {
        a.nodo = a.nodo.and_then(|x| new[x]);
    }
    muchi_non_spiegati(g);
    crate::sortc::slice_ord(&mut removed);
    g.avvisi.extend(removed.into_iter().map(|albero| Warning {
        chiave: "warn.msngrafo.isolatedUnexplained",
        albero,
        nodo: None,
    }));
}

/// The formulas of the nodes below root `r` (counts `rc`) and of their losses; the nodes they do not explain leave the graph.
pub(super) fn annotate(g: &mut Graph, e: &Elementi, r: usize, rc: Vec<u32>, o: &Options) {
    let mut st = state(g, e, r, &rc, o);
    choices(g, &mut st, o);
    while step(g, &mut st, o) {}
    if prefer_even(g, &mut st, o) {
        while step(g, &mut st, o) {}
    }
    if ricalibra(&mut st, o) {
        while step(g, &mut st, o) {}
    }
    for k in 1..st.order.len() {
        let j = st.order[k];
        let outside = st.genitori[j].iter().any(|&p| {
            !st.cand[p].is_empty()
                && !st.cand[j]
                    .iter()
                    .any(|x| st.cand[p].iter().any(|q| below(&x.c, &q.c)))
        });
        let gia = g
            .avvisi
            .iter()
            .any(|a| a.chiave == "warn.msngrafo.userConflict" && a.nodo == Some(j));
        if st.user[j] && outside && !gia {
            warning(g, "warn.msngrafo.userConflict", Some(j));
        }
    }
    let mut tieni = vec![false; g.nodi.len()];
    for &j in &st.order {
        if !st.cand[j].is_empty() {
            g.nodi[j].formula = Some(formula(g, &st, o, j));
            tieni[j] = true;
        }
    }
    for i in 0..g.nodi.len() {
        if g.nodi[i].isotope_of.is_some_and(|m| tieni[m]) {
            tieni[i] = true;
        }
    }
    losses(g, &st);
    togli(g, &tieni);
}
